//! Fsynced, hash-linked local history and fail-closed at-most-once advisory
//! authorization. Signed checkpoints need an independently retained pin for
//! rollback detection; this is not an immutable log or blockchain.
use crate::*;
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use veri_adam_decision::Clock;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Attempt,
    ReleaseReservation,
    DependencyInvalidation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub kind: Kind,
    pub context: Context,
    pub proposal_id: Option<String>,
    pub outcome: String,
    pub at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub sequence: u64,
    pub previous: String,
    pub event: Event,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub sequence: u64,
    pub tip: String,
    pub registry_hash: String,
    pub coordinator_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub context: Context,
    pub proposal_id: String,
    pub checkpoint: Signed<Checkpoint>,
    pub reservation: Entry,
    pub advisory: DecisionSummary,
    pub authorized_at_ms: u64,
}
pub struct Journal<'a> {
    dir: PathBuf,
    file: File,
    trust: &'a Trust,
    keys: &'a Keyring,
    pub checkpoint: Signed<Checkpoint>,
    consumed: BTreeSet<(String, String)>,
    poisoned: bool,
}
fn io(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn checkpoint_valid(c: &Signed<Checkpoint>, t: &Trust) -> Result<(), String> {
    if c.payload.registry_hash != t.registry.id() || c.payload.coordinator_id != t.coordinator {
        return Err("audit checkpoint trust mismatch".into());
    }
    decode::<32>(&c.payload.tip)?;
    veri_adam_protocol::verify(
        &canonical::bytes("audit-checkpoint", &c.payload),
        &c.signature,
        t.registry.key(&t.coordinator, Role::Coordinator)?,
    )
}
fn read_entries(f: &mut File, trust: &Trust) -> Result<Vec<Entry>, String> {
    if f.metadata().map_err(io)?.len() > 64 * 1024 * 1024 {
        return Err("audit log exceeds configured read bound".into());
    }
    f.seek(SeekFrom::Start(0)).map_err(io)?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(io)?;
    if !s.is_empty() && !s.ends_with('\n') {
        return Err("incomplete audit tail; no release allowed".into());
    }
    let mut entries = Vec::new();
    let mut previous = "00".repeat(32);
    for line in s.lines() {
        if line.len() > 65536 {
            return Err("audit entry too large".into());
        }
        let signed: Signed<Entry> = serde_json::from_str(line).map_err(io)?;
        veri_adam_protocol::verify(
            &canonical::bytes("audit-entry", &signed.payload),
            &signed.signature,
            trust.registry.key(&trust.coordinator, Role::Coordinator)?,
        )?;
        let e = signed.payload;
        e.event.context.validate()?;
        if e.sequence != entries.len() as u64 + 1
            || e.previous != previous
            || e.event.outcome.len() > 160
        {
            return Err("audit sequence/hash linkage mismatch".into());
        }
        if let Some(id) = &e.event.proposal_id {
            decode::<32>(id)?;
        }
        previous = canonical::id("audit-entry", &e);
        entries.push(e);
    }
    Ok(entries)
}
fn matches_prefix(c: &Checkpoint, entries: &[Entry]) -> bool {
    if c.sequence > entries.len() as u64 {
        return false;
    }
    c.tip
        == if c.sequence == 0 {
            "00".repeat(32)
        } else {
            canonical::id("audit-entry", &entries[c.sequence as usize - 1])
        }
}
fn inspect(
    entries: &[Entry],
    c: &Signed<Checkpoint>,
    t: &Trust,
    pin: Option<&Signed<Checkpoint>>,
) -> Result<(), String> {
    checkpoint_valid(c, t)?;
    if c.payload.sequence != entries.len() as u64 || !matches_prefix(&c.payload, entries) {
        return Err("audit truncation/corruption or unacknowledged tail".into());
    }
    if let Some(pin) = pin {
        checkpoint_valid(pin, t)?;
        if !matches_prefix(&pin.payload, entries) {
            return Err("audit rollback relative to independently retained checkpoint".into());
        }
    }
    Ok(())
}
pub fn verify(
    path: &Path,
    t: &Trust,
    pin: Option<&Signed<Checkpoint>>,
) -> Result<Signed<Checkpoint>, String> {
    let mut file = File::open(path.join("events.jsonl")).map_err(io)?;
    file.try_lock_shared().map_err(io)?;
    let entries = read_entries(&mut file, t)?;
    let c = serde_json::from_slice(&std::fs::read(path.join("checkpoint.json")).map_err(io)?)
        .map_err(io)?;
    inspect(&entries, &c, t, pin)?;
    Ok(c)
}
pub fn verify_release(
    release: &Signed<Release>,
    record: &Record,
    t: &Trust,
    dir: &Path,
    pin: Option<&Signed<Checkpoint>>,
) -> Result<(), String> {
    verify(dir, t, pin)?;
    veri_adam_protocol::verify(
        &canonical::bytes("proposal", &record.proposal.payload),
        &record.proposal.signature,
        t.registry.key(&t.coordinator, Role::Coordinator)?,
    )?;
    let p = &release.payload;
    veri_adam_protocol::verify(
        &canonical::bytes("advisory-release", p),
        &release.signature,
        t.registry.key(&t.coordinator, Role::Coordinator)?,
    )?;
    checkpoint_valid(&p.checkpoint, t)?;
    if p.authorized_at_ms < record.proposal.payload.completed_at_ms
        || p.reservation.event.at_ms > p.authorized_at_ms
        || p.reservation.event.outcome != "advisory_release_reserved"
        || p.context != record.proposal.payload.context
        || p.proposal_id != canonical::id("proposal", &record.proposal.payload)
        || record.proposal.payload.final_decision.as_ref() != Some(&p.advisory)
        || p.reservation.event.kind != Kind::ReleaseReservation
        || p.reservation.event.context != p.context
        || p.reservation.event.proposal_id.as_ref() != Some(&p.proposal_id)
        || p.checkpoint.payload.sequence != p.reservation.sequence
        || p.checkpoint.payload.tip != canonical::id("audit-entry", &p.reservation)
        || p.authorized_at_ms >= record.proposal.payload.deadline_ms
    {
        return Err("advisory release/recording binding mismatch".into());
    }
    let mut f = File::open(dir.join("events.jsonl")).map_err(io)?;
    f.try_lock_shared().map_err(io)?;
    let entries = read_entries(&mut f, t)?;
    if entries
        .get(p.reservation.sequence.saturating_sub(1) as usize)
        .is_none_or(|e| canonical::id("audit-entry", e) != p.checkpoint.payload.tip)
    {
        return Err("release reservation absent from retained history".into());
    }
    Ok(())
}
impl<'a> Journal<'a> {
    /// Explicitly acknowledge a complete, valid but uncheckpointed crash tail.
    /// Reservations remain consumed; recovery never retroactively releases them.
    /// Partial lines, corrupt chains and rollback relative to `pin` are refused.
    pub fn recover(
        dir: &Path,
        t: &'a Trust,
        keys: &'a Keyring,
        pin: Option<&Signed<Checkpoint>>,
    ) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .open(dir.join("events.jsonl"))
            .map_err(io)?;
        file.try_lock().map_err(io)?;
        let entries = read_entries(&mut file, t)?;
        let old: Signed<Checkpoint> =
            serde_json::from_slice(&std::fs::read(dir.join("checkpoint.json")).map_err(io)?)
                .map_err(io)?;
        checkpoint_valid(&old, t)?;
        if !matches_prefix(&old.payload, &entries) {
            return Err("crash recovery refused corrupt/truncated checkpoint prefix".into());
        }
        if let Some(pin) = pin {
            checkpoint_valid(pin, t)?;
            if !matches_prefix(&pin.payload, &entries) {
                return Err("crash recovery refused rollback".into());
            }
        }
        let checkpoint = keys.signed(
            &t.coordinator,
            "audit-checkpoint",
            Checkpoint {
                sequence: entries.len() as u64,
                tip: entries
                    .last()
                    .map(|e| canonical::id("audit-entry", e))
                    .unwrap_or_else(|| "00".repeat(32)),
                registry_hash: t.registry.id(),
                coordinator_id: t.coordinator.clone(),
            },
        )?;
        let mut j = Self {
            dir: dir.into(),
            file,
            trust: t,
            keys,
            checkpoint,
            consumed: BTreeSet::new(),
            poisoned: true,
        };
        let temp = dir.join("checkpoint.next");
        if temp.exists() {
            std::fs::remove_file(temp).map_err(io)?;
        }
        j.persist_checkpoint()
    }
    pub fn open(
        dir: &Path,
        t: &'a Trust,
        keys: &'a Keyring,
        pin: Option<&Signed<Checkpoint>>,
    ) -> Result<Self, String> {
        t.validate()?;
        std::fs::create_dir_all(dir).map_err(io)?;
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(dir.join("events.jsonl"))
            .map_err(io)?;
        file.try_lock().map_err(io)?;
        let entries = read_entries(&mut file, t)?;
        let cp = dir.join("checkpoint.json");
        let checkpoint = if cp.exists() {
            serde_json::from_slice(&std::fs::read(cp).map_err(io)?).map_err(io)?
        } else {
            if !entries.is_empty() || pin.is_some() {
                return Err("missing audit checkpoint; explicit recovery required".into());
            }
            keys.signed(
                &t.coordinator,
                "audit-checkpoint",
                Checkpoint {
                    sequence: 0,
                    tip: "00".repeat(32),
                    registry_hash: t.registry.id(),
                    coordinator_id: t.coordinator.clone(),
                },
            )?
        };
        inspect(&entries, &checkpoint, t, pin)?;
        let consumed = entries
            .iter()
            .filter(|e| e.event.kind == Kind::ReleaseReservation)
            .map(|e| {
                (
                    e.event.context.session_id.clone(),
                    e.event.context.event_id.clone(),
                )
            })
            .collect();
        let mut j = Self {
            dir: dir.into(),
            file,
            trust: t,
            keys,
            checkpoint,
            consumed,
            poisoned: false,
        };
        if !j.dir.join("checkpoint.json").exists() {
            j.file.sync_all().map_err(io)?;
            j.persist_checkpoint()?;
        }
        Ok(j)
    }
    fn persist_checkpoint(&mut self) -> Result<(), String> {
        let temp = self.dir.join("checkpoint.next");
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(io)?;
        f.write_all(&serde_json::to_vec(&self.checkpoint).map_err(io)?)
            .map_err(io)?;
        f.sync_all().map_err(io)?;
        std::fs::rename(temp, self.dir.join("checkpoint.json")).map_err(io)?;
        File::open(&self.dir).map_err(io)?.sync_all().map_err(io)
    }
    pub fn append(&mut self, event: Event) -> Result<Signed<Checkpoint>, String> {
        if self.poisoned {
            return Err("audit writer failed; restart/recovery required".into());
        }
        event.context.validate()?;
        if event.outcome.len() > 160 {
            return Err("audit outcome too long".into());
        }
        if let Some(id) = &event.proposal_id {
            decode::<32>(id)?;
        }
        let entry = Entry {
            sequence: self.checkpoint.payload.sequence + 1,
            previous: self.checkpoint.payload.tip.clone(),
            event,
        };
        let signed = self
            .keys
            .signed(&self.trust.coordinator, "audit-entry", entry.clone())?;
        let mut bytes = serde_json::to_vec(&signed).map_err(io)?;
        if bytes.len() > 65536 {
            return Err("audit entry too large".into());
        }
        bytes.push(b'\n');
        // Poison until both fsyncs and the atomic checkpoint replacement succeed.
        self.poisoned = true;
        self.file.write_all(&bytes).map_err(io)?;
        self.file.sync_all().map_err(io)?;
        self.checkpoint = self.keys.signed(
            &self.trust.coordinator,
            "audit-checkpoint",
            Checkpoint {
                sequence: entry.sequence,
                tip: canonical::id("audit-entry", &entry),
                registry_hash: self.trust.registry.id(),
                coordinator_id: self.trust.coordinator.clone(),
            },
        )?;
        self.persist_checkpoint()?;
        self.poisoned = false;
        if entry.event.kind == Kind::ReleaseReservation {
            self.consumed
                .insert((entry.event.context.session_id, entry.event.context.event_id));
        }
        Ok(self.checkpoint.clone())
    }
    /// On a serial local runtime, `request.heads` is a trusted snapshot and no
    /// concurrent update is applied while this method owns the release gate.
    /// Reserving first gives at-most-once authorization, not guaranteed delivery.
    pub fn authorize(
        &mut self,
        r: &Record,
        d: &Disclosure,
        request: &Request,
        clock: &dyn Clock,
    ) -> Result<Signed<Release>, String> {
        if !request.current {
            return Err("historical processing cannot authorize a current advisory".into());
        }
        let key = (
            request.context.session_id.clone(),
            request.context.event_id.clone(),
        );
        if self.consumed.contains(&key) {
            return Err("advisory already reserved/released; duplicate withheld".into());
        }
        let mut q = request.clone();
        q.now_ms = clock.now_ms();
        let report = checker::check(r, Some(d), self.trust, &q);
        if report.outcome != "verified_current_awaiting_recording"
            || q.now_ms >= r.proposal.payload.deadline_ms
        {
            return Err(
                "release refused: decision is not currently verified within deadline".into(),
            );
        }
        let id = canonical::id("proposal", &r.proposal.payload);
        let previous = self.checkpoint.payload.tip.clone();
        let event = Event {
            kind: Kind::ReleaseReservation,
            context: q.context.clone(),
            proposal_id: Some(id.clone()),
            outcome: "advisory_release_reserved".into(),
            at_ms: q.now_ms,
        };
        let checkpoint = self.append(event.clone())?;
        let reservation = Entry {
            sequence: checkpoint.payload.sequence,
            previous,
            event,
        };
        q.now_ms = clock.now_ms();
        if q.now_ms < reservation.event.at_ms
            || q.now_ms >= r.proposal.payload.deadline_ms
            || checker::check(r, Some(d), self.trust, &q).outcome
                != "verified_current_awaiting_recording"
        {
            return Err("recording acknowledgment arrived too late; reservation consumed, advisory withheld".into());
        }
        let release = self.keys.signed(
            &self.trust.coordinator,
            "advisory-release",
            Release {
                context: q.context,
                proposal_id: id,
                checkpoint,
                reservation,
                advisory: r
                    .proposal
                    .payload
                    .final_decision
                    .clone()
                    .ok_or("final advisory absent")?,
                authorized_at_ms: q.now_ms,
            },
        )?;
        let finish = clock.now_ms();
        if finish < release.payload.authorized_at_ms
            || finish >= r.proposal.payload.deadline_ms
            || r.observations
                .iter()
                .filter(|o| o.payload.purpose == Purpose::Fusion)
                .any(|o| {
                    o.payload
                        .captured_at_ms
                        .checked_add(self.trust.policy.evidence_ttl_ms)
                        .is_none_or(|expiry| finish >= expiry)
                })
        {
            return Err("release signing exceeded deadline; advisory withheld".into());
        }
        Ok(release)
    }
}
