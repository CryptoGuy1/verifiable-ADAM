//! Local, message-driven development agents. Public fixture keys; no device deployment.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::Write,
    process::{Command as Process, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use veri_adam_protocol::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recommendation {
    pub advisory: bool,
    pub evidence_ids: Vec<String>,
    pub explanation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inference {
    pub recommendation: Recommendation,
    pub metadata: serde_json::Value,
}
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceView {
    pub event_id: String,
    pub nonce: String,
    pub policy_hash: String,
    pub threshold_ppm: u32,
    pub evidence: Vec<SignedMeasurement>,
    pub evidence_ids: Vec<String>,
}
pub trait Reasoner: Send + 'static {
    fn label(&self) -> String;
    fn prepare(&mut self) -> Result<Option<serde_json::Value>, String> {
        Ok(None)
    }
    fn infer(&mut self, evidence: &EvidenceView) -> Result<Inference, String>;
}

pub struct RuleReasoner;
impl Reasoner for RuleReasoner {
    fn label(&self) -> String {
        "deterministic-development-rule".into()
    }
    fn infer(&mut self, v: &EvidenceView) -> Result<Inference, String> {
        Ok(Inference {
            recommendation: Recommendation {
                advisory: v
                    .evidence
                    .iter()
                    .map(|m| u64::from(m.payload.co2_ppm))
                    .sum::<u64>()
                    >= u64::from(v.threshold_ppm) * v.evidence.len() as u64,
                evidence_ids: v.evidence_ids.clone(),
                explanation:
                    "Development rule evaluated the exact sum against the experimental threshold."
                        .into(),
            },
            metadata: serde_json::json!({"backend":"rules","model_inference":false}),
        })
    }
}
pub struct OllamaReasoner {
    pub model: String,
    pub adapter: std::path::PathBuf,
}
impl Reasoner for OllamaReasoner {
    fn label(&self) -> String {
        format!("ollama:{}", self.model)
    }
    fn prepare(&mut self) -> Result<Option<serde_json::Value>, String> {
        let output = Process::new("python3")
            .arg(&self.adapter)
            .arg(&self.model)
            .arg("--warmup")
            .stdin(Stdio::null())
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "Model warm-up failed before evidence capture: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        serde_json::from_slice(&output.stdout)
            .map(Some)
            .map_err(|e| e.to_string())
    }
    fn infer(&mut self, evidence: &EvidenceView) -> Result<Inference, String> {
        let mut child = Process::new("python3")
            .arg(&self.adapter)
            .arg(&self.model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Ollama adapter: {e}"))?;
        let bytes = serde_json::to_vec(evidence).map_err(|e| e.to_string())?;
        child
            .stdin
            .take()
            .ok_or("adapter stdin unavailable")?
            .write_all(&bytes)
            .map_err(|e| e.to_string())?;
        // A wall-clock limit also covers model loading, stalled reads and the adapter itself.
        let deadline = Instant::now() + Duration::from_secs(25);
        while child.try_wait().map_err(|e| e.to_string())?.is_none() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Ollama inference exceeded 25-second adapter limit".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(600)
                .collect());
        }
        if output.stdout.len() > 64 * 1024 {
            return Err("adapter response too large".into());
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("invalid inference response: {e}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Sensor,
    Aggregator,
    Decision,
}
impl AgentRole {
    fn identity(self) -> (&'static str, u8) {
        match self {
            Self::Sensor => ("voter-a", 3),
            Self::Aggregator => ("voter-b", 4),
            Self::Decision => ("voter-c", 5),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub domain: String,
    pub event_id: String,
    pub nonce: String,
    pub policy_hash: String,
    pub registry_version: u64,
    pub role: AgentRole,
    pub voter_id: String,
    pub evidence_ids: Vec<String>,
    pub advisory: bool,
    pub explanation: String,
    pub backend: String,
    pub inference_hash: Option<String>,
}
impl Assessment {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // Fixed-order typed JSON, UTF-8, compact serde encoding; separate domain from v1 ballots.
        serde_json::to_vec(self).expect("serializable fixed assessment fields")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAssessment {
    pub payload: Assessment,
    pub signature: String,
}

#[derive(Clone)]
struct Input {
    policy: Policy,
    registry: Registry,
    request: Request,
    measurements: Vec<SignedMeasurement>,
}
fn evidence(input: &Input) -> Result<EvidenceView, String> {
    input.policy.validate(&input.registry)?;
    context(VERSION, &input.request.event_id, &input.request.nonce)?;
    if input.measurements.len() != input.policy.sources.len() {
        return Err("missing source evidence".into());
    }
    let mut sources = BTreeSet::new();
    for sm in &input.measurements {
        let m = &sm.payload;
        m.validate()?;
        if m.event_id != input.request.event_id
            || m.nonce != input.request.nonce
            || m.policy_hash != input.policy.id()
            || !input.policy.sources.contains(&m.signer_id)
            || !sources.insert(m.signer_id.clone())
        {
            return Err("invalid source/context binding".into());
        }
        let key = &input
            .registry
            .member(&m.signer_id, Role::Sensor)
            .ok_or("unauthorized source")?
            .public_key;
        verify(&m.canonical_bytes(), &sm.signature, key)?;
        if m.captured_at_ms > input.request.now_ms
            || input.request.now_ms
                >= m.captured_at_ms
                    .checked_add(input.policy.ttl_ms)
                    .ok_or("expiry overflow")?
            || input.request.heads.get(&m.signer_id) != Some(&m.id())
        {
            return Err("evidence not current".into());
        }
    }
    let mut ids: Vec<_> = input.measurements.iter().map(|m| m.payload.id()).collect();
    ids.sort();
    Ok(EvidenceView {
        event_id: input.request.event_id.clone(),
        nonce: input.request.nonce.clone(),
        policy_hash: input.policy.id(),
        threshold_ppm: input.policy.threshold_ppm,
        evidence: input.measurements.clone(),
        evidence_ids: ids,
    })
}
fn validate_recommendation(r: &Recommendation, v: &EvidenceView) -> Result<(), String> {
    if r.evidence_ids != v.evidence_ids
        || r.explanation.trim().is_empty()
        || r.explanation.len() > 1200
    {
        return Err(
            "model returned missing, reordered or invented evidence IDs, or invalid explanation"
                .into(),
        );
    }
    Ok(())
}
fn validate_assessment(a: &SignedAssessment, input: &Input) -> Result<(), String> {
    let p = &a.payload;
    let view = evidence(input)?;
    if p.domain != "veri-adam/agent-assessment/v1"
        || p.event_id != view.event_id
        || p.nonce != view.nonce
        || p.policy_hash != view.policy_hash
        || p.registry_version != input.registry.version
        || p.evidence_ids != view.evidence_ids
        || p.voter_id != p.role.identity().0
        || p.explanation.is_empty()
        || p.explanation.len() > 1200
    {
        return Err("invalid assessment binding".into());
    }
    verify(
        &p.canonical_bytes(),
        &a.signature,
        &input
            .registry
            .member(&p.voter_id, Role::Voter)
            .ok_or("unauthorized assessment")?
            .public_key,
    )
}

struct AssessmentReply {
    assessment: SignedAssessment,
    inference: Option<Inference>,
}
enum Message {
    Assess(Box<Input>, Sender<Result<AssessmentReply, String>>),
    Ballot(Decision, Sender<Result<SignedBallot, String>>),
    Stop,
}
struct Agent {
    inbox: Sender<Message>,
    worker: Option<JoinHandle<()>>,
}
impl Agent {
    fn start(role: AgentRole, mut reasoner: Option<Box<dyn Reasoner>>) -> Self {
        let (inbox, rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut last: Option<Assessment> = None;
            let mut seen = BTreeSet::new();
            while let Ok(message) = rx.recv() {
                match message {
                    Message::Stop => break,
                    Message::Assess(input, reply) => {
                        last = None;
                        let result = (|| {
                            let v = evidence(&input)?;
                            if !seen.insert(v.event_id.clone()) {
                                return Err("agent refused repeated event".into());
                            }
                            let (advisory,explanation,backend,inference)=match role {
                                AgentRole::Sensor => {
                                    let m=v.evidence.iter().find(|m|m.payload.signer_id=="sensor-a").ok_or("sensor A missing")?;
                                    (m.payload.co2_ppm>=v.threshold_ppm,"Sensor A evaluated its own signed reading against the experimental threshold.".into(),"local-sensor-rule".into(),None)
                                },
                                AgentRole::Aggregator => (
                                    v.evidence.iter().map(|m|u64::from(m.payload.co2_ppm)).sum::<u64>()>=u64::from(v.threshold_ppm)*v.evidence.len() as u64,
                                    "Aggregator independently compared the exact sum of authenticated readings with the threshold.".into(),"equal-weight-development-fusion".into(),None),
                                AgentRole::Decision => {
                                    let r=reasoner.as_mut().ok_or("reasoner unavailable")?;
                                    let inference=r.infer(&v)?; validate_recommendation(&inference.recommendation,&v)?;
                                    (inference.recommendation.advisory,inference.recommendation.explanation.clone(),r.label(),Some(inference))
                                },
                            };
                            let payload = Assessment {
                                domain: "veri-adam/agent-assessment/v1".into(),
                                event_id: v.event_id,
                                nonce: v.nonce,
                                policy_hash: v.policy_hash,
                                registry_version: input.registry.version,
                                role,
                                voter_id: role.identity().0.into(),
                                evidence_ids: v.evidence_ids,
                                advisory,
                                explanation,
                                backend,
                                inference_hash: inference
                                    .as_ref()
                                    .map(|i| digest(&serde_json::to_vec(i).unwrap())),
                            };
                            let signature =
                                sign(&payload.canonical_bytes(), &[role.identity().1; 32]);
                            last = Some(payload.clone());
                            Ok(AssessmentReply {
                                assessment: SignedAssessment { payload, signature },
                                inference,
                            })
                        })();
                        let _ = reply.send(result);
                    }
                    Message::Ballot(d, reply) => {
                        let result = (|| {
                            let a = last.as_ref().ok_or("no assessment to vote from")?;
                            if d.event_id != a.event_id
                                || d.nonce != a.nonce
                                || d.policy_hash != a.policy_hash
                                || d.measurement_ids != a.evidence_ids
                                || d.registry_version != a.registry_version
                                || d.coordinator_id != "coordinator"
                            {
                                return Err("ballot request does not match agent's evidence".into());
                            }
                            let payload = Ballot {
                                protocol_version: VERSION,
                                event_id: d.event_id.clone(),
                                nonce: d.nonce.clone(),
                                decision_hash: d.id(),
                                voter_id: a.voter_id.clone(),
                                advisory: a.advisory,
                            };
                            Ok(SignedBallot {
                                signature: sign(
                                    &payload.canonical_bytes(),
                                    &[role.identity().1; 32],
                                ),
                                payload,
                            })
                        })();
                        let _ = reply.send(result);
                    }
                }
            }
        });
        Self {
            inbox,
            worker: Some(worker),
        }
    }
    fn assess(&self, input: Input) -> Receiver<Result<AssessmentReply, String>> {
        let (tx, rx) = mpsc::channel();
        let _ = self.inbox.send(Message::Assess(Box::new(input), tx));
        rx
    }
}
impl Drop for Agent {
    fn drop(&mut self) {
        let _ = self.inbox.send(Message::Stop);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub time_ms: u64,
    pub round: usize,
    pub agent: String,
    pub kind: String,
    pub detail: String,
    pub advisory_state: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Round {
    pub name: String,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    pub readings: Vec<SignedMeasurement>,
    pub assessments: Vec<SignedAssessment>,
    pub inferences: Vec<Inference>,
    pub absent: Vec<String>,
    pub bundle: Option<Bundle>,
    pub request: Request,
    pub verification: Option<veri_adam_verification::Report>,
    pub reference_outcome: Option<Outcome>,
    pub expiry_check: Option<veri_adam_verification::Report>,
    pub expiry_reference_outcome: Option<Outcome>,
    pub state: String,
    pub reason: String,
    pub deadline_ms: u64,
}
#[derive(Debug, Serialize)]
pub struct Run {
    pub schema_version: u16,
    pub data_kind: String,
    pub public_test_keys: bool,
    pub mode: String,
    pub policy: Policy,
    pub registry: Registry,
    pub rounds: Vec<Round>,
    pub events: Vec<Event>,
    pub duration_ms: u64,
    pub zkp_implemented: bool,
    pub all_reference_outcomes_match: bool,
}
pub struct Runtime {
    policy: Policy,
    registry: Registry,
    agents: Vec<Agent>,
    clock: u64,
    events: Vec<Event>,
    rounds: Vec<Round>,
}
impl Runtime {
    pub fn new(reasoner: Box<dyn Reasoner>) -> Self {
        let (mut policy, registry, _, _) = demo::fixture(CrewMode::NominalThree, [600, 620]);
        policy.ttl_ms = 30_000;
        Self {
            policy,
            registry,
            agents: vec![
                Agent::start(AgentRole::Sensor, None),
                Agent::start(AgentRole::Aggregator, None),
                Agent::start(AgentRole::Decision, Some(reasoner)),
            ],
            clock: 0,
            events: vec![],
            rounds: vec![],
        }
    }
    fn event(
        &mut self,
        time: u64,
        round: usize,
        agent: &str,
        kind: &str,
        detail: &str,
        state: Option<&str>,
    ) {
        self.events.push(Event {
            time_ms: time,
            round,
            agent: agent.into(),
            kind: kind.into(),
            detail: detail.into(),
            advisory_state: state.map(str::to_owned),
        });
    }
    /// Dropped decision messages model a missing inbox delivery; they never call the model.
    pub fn round(
        &mut self,
        name: &str,
        values: [Option<u32>; 2],
        drop_decision: bool,
        tamper: bool,
        deadline_ms: u64,
    ) -> Result<(), String> {
        if !(1..=20_000).contains(&deadline_ms) {
            return Err("agent deadline must be 1..20000 ms".into());
        }
        let index = self.rounds.len();
        let start = self.clock;
        let timer = Instant::now();
        let event_id = format!("agent-round-{}", index + 1);
        let nonce = digest(format!("public-agent-simulation/{}", index + 1).as_bytes());
        self.event(
            start,
            index,
            "coordinator",
            "capture-request",
            "New evidence supersedes previous support; advisory awaits verification.",
            Some("pending"),
        );
        // Separate source workers own their capture and signing operation.
        let readings = thread::scope(|scope| {
            let jobs: Vec<_> = values
                .iter()
                .enumerate()
                .filter_map(|(i, value)| {
                    value.map(|co2| {
                        let event_id = event_id.clone();
                        let nonce = nonce.clone();
                        let policy_hash = self.policy.id();
                        scope.spawn(move || {
                            let payload = Measurement {
                                protocol_version: VERSION,
                                event_id,
                                nonce,
                                signer_id: format!("sensor-{}", if i == 0 { "a" } else { "b" }),
                                sequence: index as u64 + 1,
                                captured_at_ms: start,
                                co2_ppm: co2,
                                temperature_millic: 24000,
                                humidity_millipercent: 50000,
                                light_millilux: 500000,
                                calibration_version: 1,
                                policy_hash,
                            };
                            SignedMeasurement {
                                signature: sign(&payload.canonical_bytes(), &[(i + 1) as u8; 32]),
                                payload,
                            }
                        })
                    })
                })
                .collect();
            jobs.into_iter()
                .map(|j| j.join().expect("source actor panicked"))
                .collect::<Vec<_>>()
        });
        for m in &readings {
            self.event(
                start,
                index,
                &m.payload.signer_id,
                "signed-observation",
                &format!("{} ppm; evidence {}", m.payload.co2_ppm, m.payload.id()),
                None,
            );
        }
        let mut input = Input {
            policy: self.policy.clone(),
            registry: self.registry.clone(),
            request: Request {
                event_id,
                nonce,
                now_ms: start,
                kind: CheckKind::Current,
                heads: readings
                    .iter()
                    .map(|m| (m.payload.signer_id.clone(), m.payload.id()))
                    .collect(),
            },
            measurements: readings.clone(),
        };
        if tamper && let Some(first) = input.measurements.first_mut() {
            first.payload.co2_ppm += 1;
        }
        let mut round = Round {
            name: name.into(),
            started_at_ms: start,
            completed_at_ms: start,
            readings: input.measurements.clone(),
            assessments: vec![],
            inferences: vec![],
            absent: vec![],
            bundle: None,
            request: input.request.clone(),
            verification: None,
            reference_outcome: None,
            expiry_check: None,
            expiry_reference_outcome: None,
            state: "withheld".into(),
            reason: String::new(),
            deadline_ms,
        };
        if let Err(error) = evidence(&input) {
            round.reason = error;
            self.event(
                start,
                index,
                "coordinator",
                "evidence-rejected",
                &round.reason,
                Some("withheld"),
            );
        } else {
            let mut replies = vec![];
            for (i, agent) in self.agents.iter().enumerate() {
                if drop_decision && i == 2 {
                    round
                        .absent
                        .push("voter-c: simulated message loss before inference".into());
                    continue;
                }
                replies.push((i, agent.assess(input.clone())));
            }
            // Reserve time to obtain final ballots and run verification even if one assessor stalls.
            let assessment_budget = deadline_ms - deadline_ms.div_ceil(4).min(500);
            for (i, rx) in replies {
                let remaining =
                    Duration::from_millis(assessment_budget).saturating_sub(timer.elapsed());
                let result = rx.recv_timeout(remaining);
                let now = start + timer.elapsed().as_millis() as u64;
                match result {
                    Ok(Ok(reply)) if now - start <= assessment_budget => {
                        validate_assessment(&reply.assessment, &input)?;
                        if let Some(inference) = reply.inference {
                            if reply.assessment.payload.inference_hash
                                != Some(digest(&serde_json::to_vec(&inference).unwrap()))
                            {
                                return Err("inference binding mismatch".into());
                            }
                            round.inferences.push(inference);
                        }
                        self.event(
                            now,
                            index,
                            &reply.assessment.payload.voter_id,
                            "signed-assessment",
                            &format!(
                                "{}: {}",
                                if reply.assessment.payload.advisory {
                                    "ADVISORY"
                                } else {
                                    "NO ADVISORY"
                                },
                                reply.assessment.payload.explanation
                            ),
                            None,
                        );
                        round.assessments.push(reply.assessment);
                    }
                    Ok(Err(error)) => round
                        .absent
                        .push(format!("voter-{}: {error}", ["a", "b", "c"][i])),
                    _ => round.absent.push(format!(
                        "voter-{}: assessment deadline missed",
                        ["a", "b", "c"][i]
                    )),
                }
            }
            let yes = round
                .assessments
                .iter()
                .filter(|a| a.payload.advisory)
                .count();
            let no = round.assessments.len() - yes;
            if yes < 2 && no < 2 {
                round.reason =
                    "No class obtained two eligible assessments; quorum was not lowered.".into();
            } else {
                let view = evidence(&input)?;
                let payload = Decision {
                    protocol_version: VERSION,
                    event_id: view.event_id,
                    nonce: view.nonce,
                    policy_hash: view.policy_hash,
                    registry_version: self.registry.version,
                    coordinator_id: self.policy.coordinator.clone(),
                    measurement_ids: view.evidence_ids,
                    advisory: yes >= 2,
                };
                let decision = SignedDecision {
                    signature: sign(&payload.canonical_bytes(), &[6; 32]),
                    payload,
                };
                self.event(
                    start + timer.elapsed().as_millis() as u64,
                    index,
                    "coordinator",
                    "collective-proposal",
                    if decision.payload.advisory {
                        "Two assessments support an advisory; requesting bound ballots."
                    } else {
                        "Two assessments support no advisory; requesting bound ballots."
                    },
                    None,
                );
                let mut ballots = vec![];
                for assessment in &round.assessments {
                    let position = match assessment.payload.role {
                        AgentRole::Sensor => 0,
                        AgentRole::Aggregator => 1,
                        AgentRole::Decision => 2,
                    };
                    let (tx, rx) = mpsc::channel();
                    self.agents[position]
                        .inbox
                        .send(Message::Ballot(decision.payload.clone(), tx))
                        .map_err(|e| e.to_string())?;
                    if let Ok(Ok(ballot)) = rx.recv_timeout(
                        Duration::from_millis(deadline_ms).saturating_sub(timer.elapsed()),
                    ) {
                        if timer.elapsed().as_millis() as u64 > deadline_ms {
                            continue;
                        }
                        self.event(
                            start + timer.elapsed().as_millis() as u64,
                            index,
                            &ballot.payload.voter_id,
                            "signed-ballot",
                            if ballot.payload.advisory {
                                "ADVISORY"
                            } else {
                                "NO ADVISORY"
                            },
                            None,
                        );
                        ballots.push(ballot);
                    }
                }
                let bundle = Bundle {
                    decision,
                    measurements: input.measurements.clone(),
                    ballots,
                };
                round.request.now_ms = start + timer.elapsed().as_millis() as u64;
                let checked = veri_adam_verification::check(
                    &bundle,
                    &self.policy,
                    &self.registry,
                    &round.request,
                );
                let reference = veri_adam_reference::check(
                    &bundle,
                    &self.policy,
                    &self.registry,
                    &round.request,
                );
                if checked.outcome != reference {
                    return Err("agent round disagrees with reference".into());
                }
                if checked.outcome == Outcome::VerifiedCurrent
                    && timer.elapsed().as_millis() as u64 <= deadline_ms
                {
                    round.state = if bundle.decision.payload.advisory {
                        "raised"
                    } else {
                        "clear"
                    }
                    .into();
                }
                round.reason = checked.reason.clone();
                round.reference_outcome = Some(reference);
                round.verification = Some(checked);
                round.bundle = Some(bundle);
            }
            for absent in &round.absent {
                self.events.push(Event {
                    time_ms: start + timer.elapsed().as_millis() as u64,
                    round: index,
                    agent: "coordinator".into(),
                    kind: "absent-voter".into(),
                    detail: absent.clone(),
                    advisory_state: None,
                });
            }
            self.event(
                start + timer.elapsed().as_millis() as u64,
                index,
                "verifier",
                "advisory-result",
                &round.reason,
                Some(&round.state),
            );
        }
        round.completed_at_ms = start + timer.elapsed().as_millis() as u64;
        self.clock = round.completed_at_ms + 1000;
        self.rounds.push(round);
        Ok(())
    }
    pub fn finish(mut self, mode: String) -> Run {
        if let Some(last) = self.rounds.last_mut()
            && let Some(bundle) = &last.bundle
            && last
                .verification
                .as_ref()
                .is_some_and(|v| v.outcome == Outcome::VerifiedCurrent)
        {
            let expiry = last.started_at_ms + self.policy.ttl_ms;
            let mut request = last.request.clone();
            request.now_ms = expiry;
            last.expiry_check = Some(veri_adam_verification::check(
                bundle,
                &self.policy,
                &self.registry,
                &request,
            ));
            last.expiry_reference_outcome = Some(veri_adam_reference::check(
                bundle,
                &self.policy,
                &self.registry,
                &request,
            ));
            self.events.push(Event{time_ms:expiry,round:self.rounds.len()-1,agent:"dependency-monitor".into(),kind:"support-expired".into(),detail:"Supporting readings expired; the previous decision cannot remain a current advisory.".into(),advisory_state:Some("withdrawn".into())});
            self.clock = self.clock.max(expiry + 1000);
        }
        self.events.sort_by_key(|e| e.time_ms);
        let all_reference_outcomes_match = self.rounds.iter().all(|r| {
            r.verification.as_ref().map(|v| v.outcome) == r.reference_outcome
                && r.expiry_check.as_ref().map(|v| v.outcome) == r.expiry_reference_outcome
        });
        Run {
            schema_version: 1,
            data_kind: "synthetic-agent-development".into(),
            public_test_keys: true,
            mode,
            policy: self.policy,
            registry: self.registry,
            rounds: self.rounds,
            events: self.events,
            duration_ms: self.clock,
            zkp_implemented: false,
            all_reference_outcomes_match,
        }
    }
}

pub fn showcase(reasoner: Box<dyn Reasoner>, mode: String) -> Result<Run, String> {
    let mut runtime = Runtime::new(reasoner);
    for (name, values, drop_decision, tamper) in [
        ("Normal air", [Some(600), Some(620)], false, false),
        ("High CO2", [Some(930), Some(970)], false, false),
        (
            "Independent disagreement",
            [Some(790), Some(830)],
            false,
            false,
        ),
        ("Missing source", [None, Some(920)], false, false),
        (
            "Altered signed reading",
            [Some(930), Some(970)],
            false,
            true,
        ),
        (
            "Decision inbox message lost",
            [Some(790), Some(830)],
            true,
            false,
        ),
        (
            "High CO2 then evidence expiry",
            [Some(940), Some(960)],
            false,
            false,
        ),
    ] {
        runtime.round(name, values, drop_decision, tamper, 20_000)?;
    }
    Ok(runtime.finish(mode))
}

pub fn html(run: &Run) -> Result<String, String> {
    let data = serde_json::to_string(run)
        .map_err(|e| e.to_string())?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    Ok(include_str!("viewer.html").replace("__SIMULATION_DATA__", &data))
}
pub mod weighted;
