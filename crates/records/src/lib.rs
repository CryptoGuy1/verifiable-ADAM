//! Authenticated weighted ADAM records, independent public verification, a private
//! weighted-fusion predicate, local reuse/dependencies and durable advisory
//! authorization. No physical actuation, secure peer transport or v1 reinterpretation.
pub mod audit;
pub mod canonical;
pub mod checker;
pub mod private;
pub mod reuse;
mod seal;
pub use seal::{Keyring, attest, commitment, draft, fresh_nonce, prepare_observations};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use veri_adam_decision::{
    ApplicationPolicy, Classification, EventInput, Recommendation, Severity, StageRecord,
    VotingRole, calibration::CalibrationProfile,
};
use veri_adam_protocol::{decode, identifier, validate_public_key};

pub const COMMITMENT_SCHEME: &str = "ristretto255-pedersen-bulletproofs5/v1";
pub const QUORUM_RULE: &str = "adam-fixed-eligible-strict-majority/v1";
pub const PRIVATE_STATEMENT: &str = "weighted-fusion-two-committed-co2/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Source,
    Sensor,
    Aggregator,
    Decision,
    Coordinator,
}
impl From<VotingRole> for Role {
    fn from(value: VotingRole) -> Self {
        match value {
            VotingRole::Sensor => Self::Sensor,
            VotingRole::Aggregator => Self::Aggregator,
            VotingRole::Decision => Self::Decision,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub id: String,
    pub node_id: String,
    pub role: Role,
    pub public_key: String,
    pub revoked: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub version: u64,
    pub members: Vec<Member>,
}
impl Registry {
    pub fn validate(&self) -> Result<(), String> {
        if self.version == 0 || self.members.is_empty() || self.members.len() > 32 {
            return Err("invalid v2 registry".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut keys = std::collections::BTreeSet::new();
        for m in &self.members {
            identifier(&m.id)?;
            identifier(&m.node_id)?;
            validate_public_key(&m.public_key)?;
            if !ids.insert(&m.id) || !keys.insert(&m.public_key) {
                return Err("duplicate registry identity/key".into());
            }
        }
        Ok(())
    }
    pub fn key(&self, id: &str, role: Role) -> Result<&str, String> {
        self.members
            .iter()
            .find(|m| m.id == id && m.role == role && !m.revoked)
            .map(|m| m.public_key.as_str())
            .ok_or_else(|| "unauthorized/revoked signer or role".into())
    }
    pub fn id(&self) -> String {
        canonical::id("registry", self)
    }
}
/// Trusted independently of the submitted record. Local loading is a trust boundary,
/// not authenticated network distribution or historical version retention.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trust {
    pub registry: Registry,
    pub policy: ApplicationPolicy,
    pub calibration: CalibrationProfile,
    pub voters: BTreeMap<VotingRole, String>,
    pub coordinator: String,
}
impl Trust {
    pub fn validate(&self) -> Result<(), String> {
        self.registry.validate()?;
        self.policy.validate()?;
        self.calibration.validate()?;
        for role in [
            VotingRole::Sensor,
            VotingRole::Aggregator,
            VotingRole::Decision,
        ] {
            self.registry.key(
                self.voters.get(&role).ok_or("missing assigned role")?,
                role.into(),
            )?;
        }
        if self.voters.len() != 3 {
            return Err("unexpected role assignment".into());
        }
        self.registry.key(&self.coordinator, Role::Coordinator)?;
        for w in &self.calibration.weights {
            self.registry.key(&w.source_id, Role::Source)?;
        }
        Ok(())
    }
    pub fn policy_id(&self) -> String {
        canonical::id("application-policy", &self.policy)
    }
    pub fn calibration_id(&self) -> String {
        canonical::id("calibration", &self.calibration)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub event_id: String,
    pub session_id: String,
    pub location_id: String,
    pub nonce: String,
    pub started_at_ms: u64,
}
impl Context {
    pub fn validate(&self) -> Result<(), String> {
        identifier(&self.event_id)?;
        identifier(&self.session_id)?;
        identifier(&self.location_id)?;
        decode::<32>(&self.nonce)?;
        Ok(())
    }
}
impl From<&EventInput> for Context {
    fn from(e: &EventInput) -> Self {
        Self {
            event_id: e.event_id.clone(),
            session_id: e.session_id.clone(),
            location_id: e.location_id.clone(),
            nonce: e.nonce.clone(),
            started_at_ms: e.started_at_ms,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Fusion,
    TemporalBaseline,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub context: Context,
    pub source_id: String,
    pub purpose: Purpose,
    pub ordinal: u32,
    pub sequence: Option<u64>,
    pub captured_at_ms: u64,
    pub data_kind: String,
    pub calibration_hash: String,
    pub commitment_scheme: String,
    pub commitment: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionSummary {
    pub classification: Classification,
    pub confidence: f64,
    pub severity: Severity,
    pub recommended_action: String,
    pub requires_human_review: bool,
}
impl From<&Recommendation> for DecisionSummary {
    fn from(r: &Recommendation) -> Self {
        Self {
            classification: r.classification,
            confidence: r.confidence,
            severity: r.severity,
            recommended_action: r.recommended_action.clone(),
            requires_human_review: r.requires_human_review,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weight {
    pub source_id: String,
    pub units: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub protocol_version: u16,
    pub context: Context,
    pub coordinator_id: String,
    pub registry_hash: String,
    pub policy_hash: String,
    pub calibration_hash: String,
    pub observation_ids: Vec<String>,
    pub weights: Vec<Weight>,
    pub trace_commitment: String,
    pub selected_reasoner_commitment: String,
    pub eligible_roles: Vec<VotingRole>,
    pub quorum_rule: String,
    pub quorum: u32,
    pub fusion_classification: Classification,
    pub selected: DecisionSummary,
    pub final_decision: Option<DecisionSummary>,
    pub degraded: bool,
    pub governance_accepted: bool,
    pub governance_reasons: Vec<String>,
    pub stage_status: veri_adam_decision::StageStatus,
    pub completed_at_ms: u64,
    pub deadline_ms: u64,
    /// Reserved extension, never treated as verified by this public checker.
    pub proof_statement: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ballot {
    pub proposal_id: String,
    pub voter_id: String,
    pub role: VotingRole,
    pub classification: Classification,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signed<T> {
    pub payload: T,
    pub signature: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub proposal: Signed<Proposal>,
    pub observations: Vec<Signed<Observation>>,
    pub ballots: Vec<Signed<Ballot>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Opening {
    pub observation_id: String,
    pub co2_ppm: u32,
    pub blinding: String,
}
/// Authorized/B1 input only. Never an external private-verifier export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Disclosure {
    pub event: EventInput,
    pub stage: StageRecord,
    pub openings: Vec<Opening>,
    pub trace_salt: String,
    pub reasoner_salt: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub context: Context,
    pub now_ms: u64,
    pub current: bool,
    /// Trusted source -> signed v2 fusion observation ID. Required for current checks.
    pub heads: BTreeMap<String, String>,
}
