//! Local inbox-driven weighted agents. Actor keys are moved into their own
//! workers; the coordinator owns only its own key. No UI or Pi service implied.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};
use veri_adam_decision::{self as d, calibration::fuse, *};
use veri_adam_records::{self as r, canonical, checker, *};

type Job = Box<dyn FnOnce(&mut State) + Send>;
#[derive(Clone)]
struct Actor {
    tx: mpsc::Sender<Job>,
}
struct State {
    id: String,
    role: Role,
    keys: Keyring,
    trust: Trust,
    reasoner: Option<Box<dyn d::Reasoner + Send>>,
    active: Option<Input>,
    assessment: Option<Classification>,
    responses: Vec<Result<ReasonerResponse, String>>,
    signed_events: BTreeMap<String, String>,
}
#[derive(Clone)]
struct Input {
    event: EventInput,
    observations: Vec<Signed<Observation>>,
    openings: Vec<Opening>,
    clock: Arc<MonotonicClock>,
}
impl Actor {
    fn spawn(state: State) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name(format!("adam-{}", state.id))
            .spawn(move || {
                let mut state = state;
                while let Ok(job) = rx.recv() {
                    job(&mut state);
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { tx })
    }
    fn call<T: Send + 'static>(
        &self,
        remaining: u64,
        f: impl FnOnce(&mut State) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        if remaining == 0 {
            return Err("agent message deadline exhausted".into());
        }
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Box::new(move |state| {
                let _ = tx.send(f(state));
            }))
            .map_err(|_| "agent inbox disconnected")?;
        rx.recv_timeout(Duration::from_millis(remaining))
            .map_err(|_| "agent reply missing before deadline")?
    }
}
impl State {
    fn activate(&mut self, input: Input) -> Result<(), String> {
        authenticate(&input, &self.trust)?;
        self.assessment = match self.role {
            Role::Sensor => {
                let primary = input
                    .event
                    .readings
                    .iter()
                    .find(|v| v.source_id == input.event.trigger_source)
                    .ok_or("missing trigger")?;
                Some(
                    sensor_assessment(
                        primary,
                        &input.event.temporal_window,
                        "event_temporal_window",
                        self.trust.policy.threshold_ppm,
                    )
                    .classification,
                )
            }
            Role::Aggregator => Some(
                fuse(
                    &input.event.readings,
                    &self.trust.calibration,
                    self.trust.policy.threshold_ppm,
                )?
                .threshold_class,
            ),
            Role::Decision => None,
            _ => return Err("non-voter cannot activate an assessment".into()),
        };
        self.responses.clear();
        self.active = Some(input);
        Ok(())
    }
    fn infer(
        &mut self,
        mut context: ReasoningContext,
        repair: Option<String>,
        remaining: u64,
    ) -> Result<ReasonerResponse, String> {
        let input = self
            .active
            .as_ref()
            .ok_or("Decision actor has no authenticated event")?;
        let expected_fusion = fuse(
            &input.event.readings,
            &self.trust.calibration,
            self.trust.policy.threshold_ppm,
        )?;
        if self.role != Role::Decision
            || context.event_id != input.event.event_id
            || context.location_id != input.event.location_id
            || context.policy_hash != self.trust.policy.id()
            || canonical::id("readings", &context.readings)
                != canonical::id("readings", &input.event.readings)
            || canonical::id("fusion", &context.fusion) != canonical::id("fusion", &expected_fusion)
        {
            return Err("inference context differs from authenticated actor inputs".into());
        }
        context.history_scope = self.trust.registry.id();
        let result = self
            .reasoner
            .as_mut()
            .ok_or("Decision reasoner absent")?
            .infer(&context, repair.as_deref(), remaining);
        self.responses.push(result.clone());
        result
    }
    fn vote(&mut self, p: Proposal, disclosure: Disclosure) -> Result<Signed<Ballot>, String> {
        let input = self
            .active
            .as_ref()
            .ok_or("agent has no active assessment")?;
        authenticate(input, &self.trust)?;
        let deadline = input
            .event
            .started_at_ms
            .checked_add(self.trust.policy.event_budget_ms)
            .ok_or("deadline overflow")?;
        if input.clock.now_ms() >= deadline {
            return Err("late ballot withheld".into());
        }
        let role = match self.role {
            Role::Sensor => VotingRole::Sensor,
            Role::Aggregator => VotingRole::Aggregator,
            Role::Decision => VotingRole::Decision,
            _ => return Err("non-voter".into()),
        };
        let mut ids: Vec<_> = input
            .observations
            .iter()
            .map(|v| canonical::id("observation", &v.payload))
            .collect();
        ids.sort();
        if p.context != Context::from(&input.event)
            || canonical::id("event", &disclosure.event) != canonical::id("event", &input.event)
            || p.observation_ids != ids
            || p.policy_hash != self.trust.policy_id()
            || p.calibration_hash != self.trust.calibration_id()
            || p.registry_hash != self.trust.registry.id()
            || p.trace_commitment
                != canonical::hidden(
                    "trace-commitment",
                    &disclosure.trace_salt,
                    &(&disclosure.event, &disclosure.stage),
                )
            || !p.eligible_roles.contains(&role)
        {
            return Err("agent refused substituted proposal/context/support".into());
        }
        let expected = if role == VotingRole::Decision {
            let local = disclosure
                .stage
                .local_recommendation
                .as_ref()
                .ok_or("missing local recommendation")?;
            if self.responses.len() != disclosure.stage.attempts.len() {
                return Err("coordinator changed inference attempt count".into());
            }
            for (actual, attempt) in self.responses.iter().zip(&disclosure.stage.attempts) {
                match actual {
                    Ok(response)
                        if attempt.response.as_ref().is_some_and(|recorded| {
                            canonical::id("actor-inference", recorded)
                                == canonical::id("actor-inference", response)
                        }) => {}
                    Err(error)
                        if attempt.response.is_none()
                            && attempt.error.as_ref()
                                == Some(&error.chars().take(600).collect::<String>()) => {}
                    _ => return Err("coordinator changed Decision actor inference trace".into()),
                }
            }
            let actual = self
                .responses
                .iter()
                .filter_map(|response| response.as_ref().ok())
                .filter_map(|response| {
                    serde_json::from_str::<Recommendation>(&response.raw_response).ok()
                })
                .rfind(|r| r.validate().is_ok());
            if let Some(actual) = actual {
                if actual != local.recommendation {
                    return Err("coordinator changed Decision actor output".into());
                }
            } else {
                // Only an actual recorded failed attempt can authorize the declared
                // fallback. The independent checker verifies its complete fields.
                if self.responses.is_empty() || local.backend != "deterministic-threshold-fallback"
                {
                    return Err("no inference or fallback assessment".into());
                }
            }
            let mut candidates = input.event.competing.clone();
            candidates.push(local.clone());
            select_candidate(
                &candidates,
                &input.event,
                input.clock.now_ms(),
                self.trust.policy.competition_window_ms,
            )?
            .recommendation
            .classification
        } else {
            self.assessment.ok_or("agent assessment missing")?
        };
        if !disclosure
            .stage
            .ballots
            .iter()
            .any(|b| b.role == role && b.classification == expected)
        {
            return Err("coordinator changed agent assessment".into());
        }
        let proposal_id = canonical::id("proposal", &p);
        let event_key = canonical::id("event-context", &p.context);
        if self
            .signed_events
            .get(&event_key)
            .is_some_and(|old| old != &proposal_id)
        {
            return Err("actor refused conflicting proposal for same challenge".into());
        }
        if self.signed_events.len() >= 4096 && !self.signed_events.contains_key(&event_key) {
            return Err("actor session limit".into());
        }
        let ballot = self.keys.signed(
            &self.id,
            "ballot",
            Ballot {
                proposal_id: proposal_id.clone(),
                voter_id: self.id.clone(),
                role,
                classification: expected,
            },
        )?;
        self.signed_events.insert(event_key, proposal_id);
        Ok(ballot)
    }
}

fn authenticate(input: &Input, t: &Trust) -> Result<(), String> {
    t.validate()?;
    let e = &input.event;
    let mut ids = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    if input.observations.len() != e.readings.len() + e.temporal_window.len()
        || input.openings.len() != input.observations.len()
    {
        return Err("missing authenticated source support".into());
    }
    for o in &input.observations {
        let v = &o.payload;
        let id = canonical::id("observation", v);
        veri_adam_protocol::verify(
            &canonical::bytes("observation", v),
            &o.signature,
            t.registry.key(&v.source_id, Role::Source)?,
        )?;
        if !ids.insert(id.clone())
            || !ordinals.insert(v.ordinal)
            || v.context != Context::from(e)
            || v.calibration_hash != t.calibration_id()
            || v.commitment_scheme != COMMITMENT_SCHEME
        {
            return Err("source context/binding mismatch".into());
        }
        let opening = input
            .openings
            .iter()
            .find(|v| v.observation_id == id)
            .ok_or("source opening missing")?;
        if r::commitment(opening.co2_ppm, &opening.blinding)? != v.commitment {
            return Err("source commitment does not open".into());
        }
        let i = v.ordinal as usize;
        if i < e.readings.len() {
            let m = &e.readings[i];
            m.validate()?;
            let expires = m
                .captured_at_ms
                .checked_add(t.policy.evidence_ttl_ms)
                .ok_or("expiry overflow")?;
            if v.purpose != Purpose::Fusion
                || v.source_id != m.source_id
                || v.sequence != Some(m.sequence)
                || v.captured_at_ms != m.captured_at_ms
                || v.data_kind != m.data_kind
                || opening.co2_ppm != m.co2_ppm
                || m.captured_at_ms > e.started_at_ms
                || input.clock.now_ms() >= expires
            {
                return Err("invalid or stale source evidence".into());
            }
        } else {
            let m = e
                .temporal_window
                .get(i - e.readings.len())
                .ok_or("source ordinal outside event")?;
            if v.purpose != Purpose::TemporalBaseline
                || v.source_id != m.source_id
                || v.captured_at_ms != m.captured_at_ms
                || opening.co2_ppm != m.co2_ppm
                || m.source_id != e.trigger_source
            {
                return Err("invalid authenticated baseline".into());
            }
        }
    }
    Ok(())
}
struct Proxy {
    actor: Actor,
}
impl d::Reasoner for Proxy {
    fn infer(
        &mut self,
        c: &ReasoningContext,
        repair: Option<&str>,
        remaining: u64,
    ) -> Result<ReasonerResponse, String> {
        let c = c.clone();
        let repair = repair.map(str::to_owned);
        self.actor
            .call(remaining, move |state| state.infer(c, repair, remaining))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTrace {
    pub actor: String,
    pub message: String,
    pub at_ms: u64,
    pub accepted: bool,
    pub detail: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Round {
    pub received_evidence: Vec<Signed<Observation>>,
    pub event: EventInput,
    pub stage: Option<StageRecord>,
    pub record: Option<Record>,
    pub disclosure: Option<Disclosure>,
    pub request: Option<Request>,
    pub verification: Option<checker::Report>,
    pub private_proof: Option<r::private::PrivateProof>,
    pub private_verification: Option<r::private::ClaimReport>,
    pub ordinary_verification: Option<checker::Report>,
    pub dependency_verification: Option<checker::Report>,
    pub invalidations: Vec<r::reuse::Invalidation>,
    pub messages: Vec<MessageTrace>,
    pub reason: String,
}
pub struct Runtime {
    pub trust: Trust,
    coordinator: Keyring,
    actors: BTreeMap<String, Actor>,
    engine: DecisionEngine,
    history: BTreeMap<(String, String), Vec<PastReading>>,
    pub ordinary: r::reuse::OrdinaryCache,
    pub dependencies: r::reuse::Dependencies,
    heads: BTreeMap<String, String>,
}
impl Runtime {
    pub fn new(
        trust: Trust,
        mut keys: Keyring,
        reasoner: Box<dyn d::Reasoner + Send>,
    ) -> Result<Self, String> {
        trust.validate()?;
        let mut actors = BTreeMap::new();
        let mut reasoner = Some(reasoner);
        for m in &trust.registry.members {
            if m.role == Role::Coordinator {
                continue;
            }
            let key = keys.0.remove(&m.id).ok_or("missing actor key")?;
            if m.public_key != hex_key(&key.verifying_key().to_bytes()) {
                return Err("actor key/registry mismatch".into());
            }
            let state = State {
                id: m.id.clone(),
                role: m.role,
                keys: Keyring(BTreeMap::from([(m.id.clone(), key)])),
                trust: trust.clone(),
                reasoner: if m.role == Role::Decision {
                    reasoner.take()
                } else {
                    None
                },
                active: None,
                assessment: None,
                responses: vec![],
                signed_events: BTreeMap::new(),
            };
            actors.insert(m.id.clone(), Actor::spawn(state)?);
        }
        if keys.0.len() != 1 || !keys.0.contains_key(&trust.coordinator) {
            return Err("coordinator must own only its own key".into());
        }
        Ok(Self {
            trust,
            coordinator: keys,
            actors,
            engine: DecisionEngine::new(),
            history: BTreeMap::new(),
            ordinary: r::reuse::OrdinaryCache::new(128),
            dependencies: r::reuse::Dependencies::new(128),
            heads: BTreeMap::new(),
        })
    }
    pub fn round(&mut self, mut event: EventInput, tamper: bool) -> Result<Round, String> {
        let clock = Arc::new(MonotonicClock::new(event.started_at_ms));
        let deadline = event
            .started_at_ms
            .checked_add(self.trust.policy.event_budget_ms)
            .ok_or("deadline overflow")?;
        if event.temporal_window.is_empty() {
            event.temporal_window = self
                .history
                .get(&(event.session_id.clone(), event.trigger_source.clone()))
                .cloned()
                .unwrap_or_default();
        }
        let mut round = Round {
            received_evidence: vec![],
            event: event.clone(),
            stage: None,
            record: None,
            disclosure: None,
            request: None,
            verification: None,
            private_proof: None,
            private_verification: None,
            ordinary_verification: None,
            dependency_verification: None,
            invalidations: vec![],
            messages: vec![],
            reason: String::new(),
        };
        let mut observations = Vec::new();
        let mut openings = Vec::new();
        let sources: BTreeSet<_> = event
            .readings
            .iter()
            .map(|v| v.source_id.clone())
            .chain(event.temporal_window.iter().map(|v| v.source_id.clone()))
            .collect();
        for id in sources {
            let e = event.clone();
            let t = self.trust.clone();
            let source = id.clone();
            let result = self
                .actors
                .get(&id)
                .ok_or("source actor missing")?
                .call(deadline.saturating_sub(clock.now_ms()), move |state| {
                    r::prepare_observations(&e, &t, &state.keys, Some(&source))
                });
            trace(&mut round, &id, "signed-evidence", clock.as_ref(), &result);
            match result {
                Ok((a, b)) => {
                    observations.extend(a);
                    openings.extend(b);
                }
                Err(error) => {
                    round.reason = error;
                    return Ok(round);
                }
            }
        }
        observations.sort_by_key(|v| v.payload.ordinal);
        if tamper && let Some(o) = observations.first_mut() {
            o.payload.captured_at_ms = o.payload.captured_at_ms.saturating_sub(1);
        }
        let input = Input {
            event: event.clone(),
            observations: observations.clone(),
            openings: openings.clone(),
            clock: clock.clone(),
        };
        round.received_evidence = observations.clone();
        if let Err(error) = authenticate(&input, &self.trust) {
            round.reason = error;
            return Ok(round);
        }
        for o in &observations {
            if o.payload.purpose == Purpose::Fusion {
                self.heads.insert(
                    o.payload.source_id.clone(),
                    canonical::id("observation", &o.payload),
                );
            }
        }
        round.invalidations = self
            .dependencies
            .update(&self.trust, &self.heads, clock.now_ms());
        let eligible = match form_crew(&event.availability) {
            Ok(v) => v,
            Err(error) => {
                round.reason = error;
                return Ok(round);
            }
        };
        for role in eligible {
            let id = self.trust.voters[&role].clone();
            let input = input.clone();
            let result = self.actors[&id]
                .call(deadline.saturating_sub(clock.now_ms()), move |state| {
                    state.activate(input)
                });
            trace(
                &mut round,
                &id,
                "authenticated-assessment-ready",
                clock.as_ref(),
                &result,
            );
            if let Err(error) = result {
                round.reason = error;
                return Ok(round);
            }
        }
        let decision = self.actors[&self.trust.voters[&VotingRole::Decision]].clone();
        let stage = match self.engine.evaluate(
            &event,
            &self.trust.policy,
            &self.trust.calibration,
            &mut Proxy { actor: decision },
            clock.as_ref(),
        ) {
            Ok(stage) => stage,
            Err(error) => {
                round.reason = error;
                return Ok(round);
            }
        };
        round.stage = Some(stage.clone());
        for reading in &event.readings {
            let h = self
                .history
                .entry((event.session_id.clone(), reading.source_id.clone()))
                .or_default();
            if !h.iter().any(|v| v.evidence_id == reading.evidence_id) {
                h.push(PastReading {
                    evidence_id: reading.evidence_id.clone(),
                    source_id: reading.source_id.clone(),
                    captured_at_ms: reading.captured_at_ms,
                    co2_ppm: reading.co2_ppm,
                });
                h.sort_by_key(|v| v.captured_at_ms);
                if h.len() > 6 {
                    h.remove(0);
                }
            }
        }
        let (mut proposal, disclosure) =
            match r::draft(&event, &stage, &self.trust, &observations, openings) {
                Ok(v) => v,
                Err(error) => {
                    round.reason = error;
                    return Ok(round);
                }
            };
        proposal.proof_statement = PRIVATE_STATEMENT.into();
        let mut ballots = Vec::new();
        for role in &stage.eligible_roles {
            let id = self.trust.voters[role].clone();
            if event.missing_ballots.contains(role) {
                round.messages.push(MessageTrace {
                    actor: id,
                    message: "ballot-lost".into(),
                    at_ms: clock.now_ms(),
                    accepted: false,
                    detail: "explicit transport-loss fixture; eligibility/quorum preserved".into(),
                });
                continue;
            }
            let p = proposal.clone();
            let d = disclosure.clone();
            let result = self.actors[&id]
                .call(deadline.saturating_sub(clock.now_ms()), move |state| {
                    state.vote(p, d)
                });
            trace(
                &mut round,
                &id,
                "signed-role-ballot",
                clock.as_ref(),
                &result,
            );
            if let Ok(b) = result {
                ballots.push(b);
            }
        }
        let heads = observations
            .iter()
            .filter(|o| o.payload.purpose == Purpose::Fusion)
            .map(|o| {
                (
                    o.payload.source_id.clone(),
                    canonical::id("observation", &o.payload),
                )
            })
            .collect();
        let request = Request {
            context: Context::from(&event),
            now_ms: clock.now_ms(),
            current: true,
            heads,
        };
        let record = Record {
            proposal: self
                .coordinator
                .signed(&self.trust.coordinator, "proposal", proposal)?,
            observations,
            ballots,
        };
        let mut verification = checker::check(&record, Some(&disclosure), &self.trust, &request);
        let ordinary = self
            .ordinary
            .check(&record, &disclosure, &self.trust, &request);
        let dependency = self
            .dependencies
            .check(&record, &disclosure, &self.trust, &request);
        if ordinary.outcome != verification.outcome || dependency.outcome != verification.outcome {
            return Err("B1/B2/B3 disagree on actual agent decision".into());
        }
        round.ordinary_verification = Some(ordinary);
        round.dependency_verification = Some(dependency);
        if clock.now_ms() >= deadline && verification.outcome.starts_with("verified_") {
            verification.outcome = "unconfirmed".into();
            verification.reason = "agent signing/checking exhausted event budget; withheld".into();
        }
        round.reason = verification.reason.clone();
        round.record = Some(record);
        round.disclosure = Some(disclosure);
        round.request = Some(request);
        round.verification = Some(verification);
        Ok(round)
    }
}
fn trace<T>(
    round: &mut Round,
    id: &str,
    message: &str,
    clock: &dyn Clock,
    result: &Result<T, String>,
) {
    round.messages.push(MessageTrace {
        actor: id.into(),
        message: message.into(),
        at_ms: clock.now_ms(),
        accepted: result.is_ok(),
        detail: result
            .as_ref()
            .err()
            .cloned()
            .unwrap_or_else(|| "actual actor reply received".into()),
    });
}
fn hex_key(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
