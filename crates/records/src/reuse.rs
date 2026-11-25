//! Matched ordinary reuse and reverse dependencies for actual v2 records.
//! All candidate content is hashed on every query, including signatures and
//! disclosures. B3 shares B2's assurance; indexing is not assumed to be faster.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Work {
    pub full_checks: u64,
    pub signatures_verified: u64,
    pub cache_hits: u64,
    pub input_bytes_hashed: u64,
    pub head_comparisons: u64,
    pub snapshot_keys_scanned: u64,
    pub index_links_visited: u64,
    pub claims_invalidated: u64,
    pub expiry_claims_scanned: u64,
}
#[derive(Clone)]
struct Entry {
    report: checker::Report,
    completed: u64,
    support: Vec<(String, String, u64)>,
}
pub struct OrdinaryCache {
    entries: BTreeMap<String, Entry>,
    capacity: usize,
    pub work: Work,
}
impl OrdinaryCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            capacity,
            work: Work::default(),
        }
    }
    pub fn check(&mut self, r: &Record, d: &Disclosure, t: &Trust, q: &Request) -> checker::Report {
        let bytes = canonical::bytes(
            "verification-cache-input",
            &(r, d, t, &q.context, q.current),
        );
        self.work.input_bytes_hashed += bytes.len() as u64;
        let key = veri_adam_protocol::digest(&bytes);
        if let Some(e) = self.entries.get(&key)
            && q.now_ms >= e.completed
        {
            self.work.cache_hits += 1;
            let mut result = e.report.clone();
            result.signatures_verified = 0;
            if q.current {
                let mut changed = false;
                let mut expired = false;
                for (source, id, expiry) in &e.support {
                    self.work.head_comparisons += 1;
                    changed |= q.heads.get(source) != Some(id);
                    expired |= q.now_ms >= *expiry;
                }
                if changed || expired {
                    result.outcome = "unconfirmed".into();
                    result.reason = if changed {
                        "unconfirmed: support head absent or replaced"
                    } else {
                        "unconfirmed: support expired at trusted verifier time"
                    }
                    .into();
                    result.fusion_classification = None;
                    result.crew_classification = None;
                }
            }
            return result;
        }
        let report = checker::check(r, Some(d), t, q);
        self.work.full_checks += 1;
        self.work.signatures_verified += report.signatures_verified as u64;
        if report.outcome.starts_with("verified_") && self.capacity > 0 {
            if self.entries.len() >= self.capacity && !self.entries.contains_key(&key) {
                self.entries.pop_first();
            }
            let support = r
                .observations
                .iter()
                .filter(|o| o.payload.purpose == Purpose::Fusion)
                .map(|o| {
                    Ok((
                        o.payload.source_id.clone(),
                        canonical::id("observation", &o.payload),
                        o.payload
                            .captured_at_ms
                            .checked_add(t.policy.evidence_ttl_ms)
                            .ok_or("expiry overflow")?,
                    ))
                })
                .collect::<Result<Vec<_>, String>>();
            if let Ok(support) = support {
                self.entries.insert(
                    key,
                    Entry {
                        report: report.clone(),
                        completed: r.proposal.payload.completed_at_ms,
                        support,
                    },
                );
            }
        }
        report
    }
}
#[derive(Clone)]
struct Support {
    heads: BTreeMap<String, String>,
    expires: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invalidation {
    pub claim_id: String,
    pub cause: String,
}
pub struct Dependencies {
    pub cache: OrdinaryCache,
    claims: BTreeMap<String, Support>,
    reverse: BTreeMap<String, BTreeSet<String>>,
    heads: BTreeMap<String, String>,
    trust_id: Option<String>,
    capacity: usize,
    pub work: Work,
}
impl Dependencies {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: OrdinaryCache::new(capacity),
            claims: BTreeMap::new(),
            reverse: BTreeMap::new(),
            heads: BTreeMap::new(),
            trust_id: None,
            capacity,
            work: Work::default(),
        }
    }
    fn remove(&mut self, id: &str) {
        if let Some(s) = self.claims.remove(id) {
            for source in s.heads.keys() {
                self.work.index_links_visited += 1;
                if let Some(ids) = self.reverse.get_mut(source) {
                    ids.remove(id);
                    if ids.is_empty() {
                        self.reverse.remove(source);
                    }
                }
            }
        }
    }
    /// `heads` must come from the trusted authenticated update stream, not an
    /// unverified network message. Runtime invokes this after source validation.
    pub fn update(
        &mut self,
        t: &Trust,
        heads: &BTreeMap<String, String>,
        now: u64,
    ) -> Vec<Invalidation> {
        let bytes = canonical::bytes("dependency-configuration", t);
        self.work.input_bytes_hashed += bytes.len() as u64;
        let trust_id = veri_adam_protocol::digest(&bytes);
        let mut affected: BTreeMap<String, String> = BTreeMap::new();
        if self.trust_id.as_ref().is_some_and(|old| old != &trust_id) {
            for id in self.claims.keys() {
                affected.insert(
                    id.clone(),
                    "policy/calibration/registry/role configuration changed".into(),
                );
            }
        }
        self.trust_id = Some(trust_id);
        let sources: BTreeSet<_> = self.heads.keys().chain(heads.keys()).cloned().collect();
        for source in sources {
            self.work.snapshot_keys_scanned += 1;
            if self.heads.get(&source) != heads.get(&source)
                && let Some(ids) = self.reverse.get(&source)
            {
                for id in ids {
                    self.work.index_links_visited += 1;
                    affected.insert(
                        id.clone(),
                        format!("supporting source {source} replaced or unavailable"),
                    );
                }
            }
        }
        // A bounded scan is counted explicitly; this is not claimed to be an
        // optimized expiry scheduler or incremental fusion implementation.
        for (id, s) in &self.claims {
            self.work.expiry_claims_scanned += 1;
            if now >= s.expires {
                affected.insert(id.clone(), "support expired".into());
            }
        }
        self.heads = heads.clone();
        let result: Vec<_> = affected
            .into_iter()
            .map(|(claim_id, cause)| Invalidation { claim_id, cause })
            .collect();
        self.work.claims_invalidated += result.len() as u64;
        for e in &result {
            self.remove(&e.claim_id);
        }
        result
    }
    pub fn check(&mut self, r: &Record, d: &Disclosure, t: &Trust, q: &Request) -> checker::Report {
        let report = self.cache.check(r, d, t, q);
        if report.outcome == "verified_current_awaiting_recording" && self.capacity > 0 {
            let claim = canonical::id("proposal", &r.proposal.payload);
            if !self.claims.contains_key(&claim) {
                if self.claims.len() >= self.capacity
                    && let Some(id) = self.claims.keys().next().cloned()
                {
                    self.remove(&id);
                }
                let mut heads = BTreeMap::new();
                let mut expires = u64::MAX;
                for o in &r.observations {
                    if o.payload.purpose == Purpose::Fusion {
                        heads.insert(
                            o.payload.source_id.clone(),
                            canonical::id("observation", &o.payload),
                        );
                        expires = expires.min(
                            o.payload
                                .captured_at_ms
                                .saturating_add(t.policy.evidence_ttl_ms),
                        );
                        self.reverse
                            .entry(o.payload.source_id.clone())
                            .or_default()
                            .insert(claim.clone());
                        self.work.index_links_visited += 1;
                    }
                }
                self.claims.insert(claim, Support { heads, expires });
            }
        }
        report
    }
    pub fn active_claims(&self) -> usize {
        self.claims.len()
    }
}
