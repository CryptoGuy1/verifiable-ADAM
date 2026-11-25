//! Shared paper-derived ADAM decision semantics. This stage is NOT an authenticated
//! v2 protocol, private proof or durable action-release implementation.
pub mod calibration;
pub mod fixtures;
mod pipeline;
pub mod reasoner;
pub use pipeline::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub fn bounded_id(value: &str) -> Result<(), String> {
    veri_adam_protocol::identifier(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Classification {
    Normal,
    Anomaly,
}
impl Classification {
    pub fn from_anomaly(value: bool) -> Self {
        if value { Self::Anomaly } else { Self::Normal }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    None,
    Low,
    Moderate,
    High,
    Critical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VotingRole {
    Sensor,
    Aggregator,
    Decision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recommendation {
    pub classification: Classification,
    pub confidence: f64,
    pub severity: Severity,
    pub reasoning: String,
    pub recommended_action: String,
    pub contributing_factors: Vec<String>,
    pub requires_human_review: bool,
}
impl Recommendation {
    pub fn validate(&self) -> Result<(), String> {
        if !self.confidence.is_finite()
            || !(0.0..=1.0).contains(&self.confidence)
            || self.reasoning.trim().is_empty()
            || self.reasoning.len() > 2400
            || self.recommended_action.is_empty()
            || self.recommended_action.len() > 64
            || self.contributing_factors.len() > 16
            || self.contributing_factors.iter().any(|s| s.len() > 256)
        {
            return Err("invalid seven-field recommendation".into());
        }
        Ok(())
    }
    pub fn id(&self) -> String {
        veri_adam_protocol::digest(&serde_json::to_vec(self).expect("validated recommendation"))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alignment {
    pub severity: Severity,
    pub action: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceConcentration {
    TriggerReading,
    WeightedFusion,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPolicy {
    pub version: u64,
    pub provenance: String,
    pub threshold_ppm: u32,
    pub event_budget_ms: u64,
    pub evidence_ttl_ms: u64,
    pub competition_window_ms: u64,
    pub confidence_floor: f64,
    pub high_concentration_ppm: u32,
    pub high_concentration_source: GovernanceConcentration,
    pub permitted_actions: BTreeSet<String>,
    pub passive_actions: BTreeSet<String>,
    pub normal_alignment: Alignment,
    pub anomaly_alignment: Alignment,
    pub fallback_confidence: f64,
}
impl ApplicationPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.version == 0
            || self.provenance.trim().is_empty()
            || self.provenance.len() > 1200
            || self.threshold_ppm == 0
            || self.threshold_ppm > calibration::MAX_PPM
            || self.event_budget_ms == 0
            || self.event_budget_ms > 300_000
            || self.evidence_ttl_ms == 0
            || self.evidence_ttl_ms > 300_000
            || self.competition_window_ms > 300_000
            || self.high_concentration_ppm > calibration::MAX_PPM
            || !self.confidence_floor.is_finite()
            || !(0.0..=1.0).contains(&self.confidence_floor)
            || !self.fallback_confidence.is_finite()
            || !(0.0..=1.0).contains(&self.fallback_confidence)
            || self.permitted_actions.is_empty()
            || self.permitted_actions.len() > 32
        {
            return Err("invalid explicit application policy".into());
        }
        for a in &self.permitted_actions {
            bounded_id(a)?;
        }
        if !self.passive_actions.is_subset(&self.permitted_actions)
            || !self
                .permitted_actions
                .contains(&self.normal_alignment.action)
            || !self
                .permitted_actions
                .contains(&self.anomaly_alignment.action)
        {
            return Err("alignment/passive action missing from permitted set".into());
        }
        Ok(())
    }
    pub fn id(&self) -> String {
        veri_adam_protocol::digest(&serde_json::to_vec(self).expect("validated policy"))
    }
    pub fn alignment(&self, class: Classification) -> &Alignment {
        match class {
            Classification::Normal => &self.normal_alignment,
            Classification::Anomaly => &self.anomaly_alignment,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reading {
    pub evidence_id: String,
    pub source_id: String,
    pub sequence: u64,
    pub captured_at_ms: u64,
    pub co2_ppm: u32,
    pub data_kind: String,
}
impl Reading {
    pub fn validate(&self) -> Result<(), String> {
        bounded_id(&self.evidence_id)?;
        bounded_id(&self.source_id)?;
        if self.sequence == 0
            || self.co2_ppm > calibration::MAX_PPM
            || !["synthetic", "replay", "physical"].contains(&self.data_kind.as_str())
        {
            return Err("invalid reading".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PastReading {
    pub evidence_id: String,
    pub source_id: String,
    pub captured_at_ms: u64,
    pub co2_ppm: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrewAvailability {
    pub sensor: bool,
    pub aggregator: bool,
    pub decision: bool,
    pub coordinator: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub source_event_id: String,
    pub location_id: String,
    pub event_started_at_ms: u64,
    pub recommended_at_ms: u64,
    pub recommendation: Recommendation,
    pub backend: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventInput {
    pub event_id: String,
    pub session_id: String,
    pub location_id: String,
    pub nonce: String,
    pub started_at_ms: u64,
    pub trigger_source: String,
    pub readings: Vec<Reading>,
    pub temporal_window: Vec<PastReading>,
    pub availability: CrewAvailability,
    pub missing_ballots: BTreeSet<VotingRole>,
    pub competing: Vec<Candidate>,
}
