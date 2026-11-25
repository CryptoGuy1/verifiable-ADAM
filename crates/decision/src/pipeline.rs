use crate::{
    calibration::{CalibrationProfile, Fusion, fuse},
    *,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

pub trait Clock {
    fn now_ms(&self) -> u64;
}
pub struct MonotonicClock {
    base: u64,
    start: Instant,
}
impl MonotonicClock {
    pub fn new(base: u64) -> Self {
        Self {
            base,
            start: Instant::now(),
        }
    }
}
impl Clock for MonotonicClock {
    fn now_ms(&self) -> u64 {
        self.base
            .saturating_add(self.start.elapsed().as_millis() as u64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningContext {
    pub event_id: String,
    pub location_id: String,
    pub policy_hash: String,
    pub primary: Reading,
    pub readings: Vec<Reading>,
    pub fusion: Fusion,
    pub sensor_baseline_ppm: f64,
    pub threshold_ppm: u32,
    pub permitted_actions: BTreeSet<String>,
    pub normal_alignment: Alignment,
    pub anomaly_alignment: Alignment,
    pub application_policy: ApplicationPolicy,
    pub prior_resolved_events: Vec<Candidate>,
    pub semantic_retrieval_status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasonerResponse {
    pub backend: String,
    pub raw_response: String,
    pub metadata: serde_json::Value,
}
pub trait Reasoner {
    fn infer(
        &mut self,
        context: &ReasoningContext,
        repair_of: Option<&str>,
        remaining_ms: u64,
    ) -> Result<ReasonerResponse, String>;
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub repair: bool,
    pub received_at_ms: u64,
    pub response: Option<ReasonerResponse>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleBallot {
    pub role: VotingRole,
    pub classification: Classification,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorAssessment {
    pub classification: Classification,
    pub baseline_ppm: f64,
    pub baseline_origin: String,
    pub baseline_evidence_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceVerdict {
    pub accepted: bool,
    pub reasons: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    FormationFailed,
    TimedOut,
    EvidenceExpired,
    Unresolved,
    GovernanceRejected,
    ApprovedAwaitingRecording,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRecord {
    pub record_kind: String,
    pub event_id: String,
    pub event_nonce: String,
    pub session_id: String,
    pub location_id: String,
    pub support_evidence_ids: Vec<String>,
    pub trigger_evidence_id: Option<String>,
    pub policy_hash: String,
    pub calibration_hash: String,
    pub eligible_roles: Vec<VotingRole>,
    pub quorum: usize,
    pub fusion: Option<Fusion>,
    pub sensor_assessment: Option<SensorAssessment>,
    pub local_recommendation: Option<Candidate>,
    pub selected_recommendation: Option<Candidate>,
    pub selected_reasoner_output_hash: Option<String>,
    pub ballots: Vec<RoleBallot>,
    pub final_classification: Option<Classification>,
    pub final_decision: Option<Recommendation>,
    pub crew_support: Option<f64>,
    pub degraded_mode: bool,
    pub degraded_reasons: Vec<String>,
    pub attempts: Vec<Attempt>,
    pub governance: Option<GovernanceVerdict>,
    pub status: StageStatus,
    pub reason: String,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    pub deadline_ms: u64,
    pub authenticated_record: bool,
    pub durable_recording_acknowledged: bool,
    pub action_released: bool,
    pub private_proof_verified: bool,
}

pub fn form_crew(a: &CrewAvailability) -> Result<Vec<VotingRole>, String> {
    if !a.sensor || !a.decision || !a.coordinator {
        return Err("Sensor, Decision and Coordinator are required at formation".into());
    }
    Ok(if a.aggregator {
        vec![
            VotingRole::Sensor,
            VotingRole::Aggregator,
            VotingRole::Decision,
        ]
    } else {
        vec![VotingRole::Sensor, VotingRole::Decision]
    })
}
pub fn quorum(
    eligible: &[VotingRole],
    ballots: &[RoleBallot],
) -> Result<(usize, Option<Classification>, usize), String> {
    let eligible_set: BTreeSet<_> = eligible.iter().copied().collect();
    let nominal = BTreeSet::from([
        VotingRole::Sensor,
        VotingRole::Aggregator,
        VotingRole::Decision,
    ]);
    let degraded = BTreeSet::from([VotingRole::Sensor, VotingRole::Decision]);
    if eligible_set.len() != eligible.len() || (eligible_set != nominal && eligible_set != degraded)
    {
        return Err("invalid eligible crew".into());
    }
    let needed = eligible.len() / 2 + 1;
    let mut seen = BTreeSet::new();
    let mut yes = 0;
    for b in ballots {
        if !eligible_set.contains(&b.role) || !seen.insert(b.role) {
            return Err("unexpected or duplicate voter".into());
        }
        yes += usize::from(b.classification == Classification::Anomaly);
    }
    let no = ballots.len() - yes;
    Ok((
        needed,
        if yes >= needed {
            Some(Classification::Anomaly)
        } else if no >= needed {
            Some(Classification::Normal)
        } else {
            None
        },
        yes.max(no),
    ))
}
pub fn sensor_assessment(
    primary: &Reading,
    history: &[PastReading],
    origin: &str,
    threshold: u32,
) -> SensorAssessment {
    let sum: u64 = history.iter().map(|h| u64::from(h.co2_ppm)).sum();
    let count = history.len() as u64;
    let spike = if count == 0 {
        u64::from(primary.co2_ppm) >= 2 * u64::from(primary.co2_ppm)
    } else {
        u64::from(primary.co2_ppm) * count >= 2 * sum
    };
    SensorAssessment {
        classification: Classification::from_anomaly(primary.co2_ppm >= threshold || spike),
        baseline_ppm: if count == 0 {
            f64::from(primary.co2_ppm)
        } else {
            sum as f64 / count as f64
        },
        baseline_origin: if count == 0 {
            "current_reading_initialization".into()
        } else {
            origin.into()
        },
        baseline_evidence_ids: history.iter().map(|h| h.evidence_id.clone()).collect(),
    }
}
pub fn select_candidate(
    candidates: &[Candidate],
    event: &EventInput,
    now: u64,
    window_ms: u64,
) -> Result<Candidate, String> {
    if candidates.is_empty() || candidates.len() > 32 {
        return Err("invalid recommendation candidate count".into());
    }
    let mut seen = BTreeSet::new();
    for c in candidates {
        c.recommendation.validate()?;
        bounded_id(&c.source_event_id)?;
        if c.location_id != event.location_id
            || c.event_started_at_ms.abs_diff(event.started_at_ms) > window_ms
            || c.recommended_at_ms > now
            || c.recommended_at_ms < c.event_started_at_ms
            || !seen.insert(&c.source_event_id)
        {
            return Err(
                "competing recommendation has wrong location, time or duplicate origin".into(),
            );
        }
    }
    // The paper leaves an exact severity/time tie unspecified. Stable source ID is
    // only a deterministic tie-break, and is documented as an implementation detail.
    Ok(candidates
        .iter()
        .max_by(|a, b| {
            a.recommendation
                .severity
                .cmp(&b.recommendation.severity)
                .then(a.recommended_at_ms.cmp(&b.recommended_at_ms))
                .then(a.source_event_id.cmp(&b.source_event_id))
        })
        .unwrap()
        .clone())
}
pub fn assemble(
    selected: &Recommendation,
    final_class: Classification,
    p: &ApplicationPolicy,
) -> Recommendation {
    let mut final_decision = selected.clone();
    if selected.classification != final_class {
        let alignment = p.alignment(final_class);
        final_decision.classification = final_class;
        final_decision.severity = alignment.severity;
        final_decision.recommended_action = alignment.action.clone();
        if final_decision.severity == Severity::Critical {
            final_decision.requires_human_review = true;
        }
    }
    final_decision
}
pub fn validate_governance(
    final_decision: &Recommendation,
    selected_confidence: f64,
    reported_ppm: u32,
    degraded: bool,
    disclosed: bool,
    p: &ApplicationPolicy,
) -> GovernanceVerdict {
    let mut reasons = Vec::new();
    if p.validate().is_err() {
        reasons.push("invalid_application_policy".into());
    }
    if final_decision.validate().is_err() {
        reasons.push("invalid_decision_fields".into());
    }
    if !selected_confidence.is_finite() || selected_confidence < p.confidence_floor {
        reasons.push("reasoner_confidence_below_floor".into());
    }
    if final_decision.confidence != selected_confidence {
        reasons.push("reasoner_confidence_was_reinterpreted".into());
    }
    if !p
        .permitted_actions
        .contains(&final_decision.recommended_action)
    {
        reasons.push("action_not_permitted".into());
    }
    if reported_ppm >= p.high_concentration_ppm
        && p.passive_actions
            .contains(&final_decision.recommended_action)
    {
        reasons.push("passive_action_at_high_concentration".into());
    }
    if final_decision.severity == Severity::Critical && !final_decision.requires_human_review {
        reasons.push("critical_event_requires_review".into());
    }
    if degraded && final_decision.classification == Classification::Anomaly && !disclosed {
        reasons.push("degraded_anomaly_not_disclosed".into());
    }
    GovernanceVerdict {
        accepted: reasons.is_empty(),
        reasons,
    }
}

struct Budget<'a> {
    clock: &'a dyn Clock,
    last: u64,
    end: u64,
}
impl Budget<'_> {
    fn now(&mut self) -> Result<u64, String> {
        let n = self.clock.now_ms();
        if n < self.last {
            return Err("trusted clock moved backwards".into());
        }
        self.last = n;
        Ok(n)
    }
    fn remaining(&mut self) -> Result<u64, String> {
        let n = self.now()?;
        Ok(self.end.saturating_sub(n))
    }
}

#[derive(Default)]
pub struct DecisionEngine {
    history: BTreeMap<(String, String), Vec<PastReading>>,
    seen: BTreeSet<(String, String)>,
}
impl DecisionEngine {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn evaluate(
        &mut self,
        event: &EventInput,
        p: &ApplicationPolicy,
        calibration: &CalibrationProfile,
        reasoner: &mut dyn Reasoner,
        clock: &dyn Clock,
    ) -> Result<StageRecord, String> {
        p.validate()?;
        calibration.validate()?;
        for s in [
            &event.event_id,
            &event.session_id,
            &event.location_id,
            &event.trigger_source,
        ] {
            bounded_id(s)?;
        }
        veri_adam_protocol::decode::<32>(&event.nonce)?;
        if event.temporal_window.len() > 4096
            || event.readings.len() > 32
            || event.competing.len() > 31
        {
            return Err("event input exceeds limits".into());
        }
        let initial = clock.now_ms();
        if initial < event.started_at_ms {
            return Err("event starts in the future".into());
        }
        let deadline = event
            .started_at_ms
            .checked_add(p.event_budget_ms)
            .ok_or("deadline overflow")?;
        let mut budget = Budget {
            clock,
            last: initial,
            end: deadline,
        };
        let key = (event.session_id.clone(), event.event_id.clone());
        if self.seen.contains(&key) {
            return Err("event already processed in this engine session".into());
        }
        if self.seen.len() >= 4096 {
            return Err("local engine session limit reached".into());
        }
        self.seen.insert(key);
        let mut r = StageRecord {
            record_kind: "adam-decision-stage/v1".into(),
            event_id: event.event_id.clone(),
            event_nonce: event.nonce.clone(),
            session_id: event.session_id.clone(),
            location_id: event.location_id.clone(),
            support_evidence_ids: event
                .readings
                .iter()
                .map(|m| m.evidence_id.clone())
                .collect(),
            trigger_evidence_id: None,
            policy_hash: p.id(),
            calibration_hash: calibration.id(),
            eligible_roles: vec![],
            quorum: 0,
            fusion: None,
            sensor_assessment: None,
            local_recommendation: None,
            selected_recommendation: None,
            selected_reasoner_output_hash: None,
            ballots: vec![],
            final_classification: None,
            final_decision: None,
            crew_support: None,
            degraded_mode: false,
            degraded_reasons: vec![],
            attempts: vec![],
            governance: None,
            status: StageStatus::FormationFailed,
            reason: String::new(),
            started_at_ms: event.started_at_ms,
            completed_at_ms: initial,
            deadline_ms: deadline,
            authenticated_record: false,
            durable_recording_acknowledged: false,
            action_released: false,
            private_proof_verified: false,
        };
        if budget.remaining()? == 0 {
            return Ok(finish(
                r,
                StageStatus::TimedOut,
                "event_budget_exhausted",
                budget.now()?,
            ));
        }
        r.eligible_roles = match form_crew(&event.availability) {
            Ok(v) => v,
            Err(e) => return Ok(finish(r, StageStatus::FormationFailed, &e, budget.now()?)),
        };
        r.quorum = r.eligible_roles.len() / 2 + 1;
        if !event.availability.aggregator {
            r.degraded_mode = true;
            r.degraded_reasons
                .push("aggregator_unavailable_at_formation".into());
        }
        if !event
            .missing_ballots
            .is_subset(&r.eligible_roles.iter().copied().collect())
        {
            return Err("missing-ballot fixture names an ineligible role".into());
        }
        let mut evidence_ids = BTreeSet::new();
        for reading in &event.readings {
            reading.validate()?;
            if !evidence_ids.insert(&reading.evidence_id) {
                return Err("duplicate observation ID".into());
            }
            if reading.captured_at_ms > event.started_at_ms {
                return Err("future observation relative to event capture".into());
            }
        }
        let primary = event
            .readings
            .iter()
            .find(|x| x.source_id == event.trigger_source)
            .ok_or("trigger reading missing")?;
        r.trigger_evidence_id = Some(primary.evidence_id.clone());
        let fusion = fuse(&event.readings, calibration, p.threshold_ppm)?;
        let horizon = event
            .readings
            .iter()
            .map(|m| {
                m.captured_at_ms
                    .checked_add(p.evidence_ttl_ms)
                    .ok_or("expiry overflow")
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min()
            .unwrap();
        r.fusion = Some(fusion.clone());
        if initial >= horizon {
            return Ok(finish(
                r,
                StageStatus::EvidenceExpired,
                "support_expired",
                budget.now()?,
            ));
        }
        let history_key = (event.session_id.clone(), primary.source_id.clone());
        let history = if !event.temporal_window.is_empty() {
            let mut seen = BTreeSet::new();
            for h in &event.temporal_window {
                bounded_id(&h.evidence_id)?;
                if h.source_id != primary.source_id
                    || h.captured_at_ms >= primary.captured_at_ms
                    || h.co2_ppm > calibration::MAX_PPM
                    || !seen.insert(&h.evidence_id)
                {
                    return Err("temporal window must contain unique earlier observations from the triggering source".into());
                }
            }
            event.temporal_window.clone()
        } else {
            self.history
                .get(&history_key)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|h| h.captured_at_ms < primary.captured_at_ms)
                .collect()
        };
        let sensor = sensor_assessment(
            primary,
            &history,
            if event.temporal_window.is_empty() {
                "internal_history"
            } else {
                "event_temporal_window"
            },
            p.threshold_ppm,
        );
        let stored = self.history.entry(history_key).or_default();
        if !stored.iter().any(|h| h.evidence_id == primary.evidence_id) {
            stored.push(PastReading {
                evidence_id: primary.evidence_id.clone(),
                source_id: primary.source_id.clone(),
                captured_at_ms: primary.captured_at_ms,
                co2_ppm: primary.co2_ppm,
            });
            stored.sort_by_key(|h| h.captured_at_ms);
            if stored.len() > 6 {
                stored.drain(..stored.len() - 6);
            }
        }
        r.sensor_assessment = Some(sensor.clone());
        let context = ReasoningContext {
            event_id: event.event_id.clone(),
            location_id: event.location_id.clone(),
            policy_hash: p.id(),
            primary: primary.clone(),
            readings: event.readings.clone(),
            fusion: fusion.clone(),
            sensor_baseline_ppm: sensor.baseline_ppm,
            threshold_ppm: p.threshold_ppm,
            permitted_actions: p.permitted_actions.clone(),
            normal_alignment: p.normal_alignment.clone(),
            anomaly_alignment: p.anomaly_alignment.clone(),
            application_policy: p.clone(),
            prior_resolved_events: vec![],
            semantic_retrieval_status: "not_integrated".into(),
        };
        let mut chosen = None;
        let mut previous_raw: Option<String> = None;
        for repair in [false, true] {
            let remaining = budget.remaining()?;
            if remaining == 0 {
                break;
            }
            match reasoner.infer(&context, previous_raw.as_deref(), remaining) {
                Ok(response) => {
                    let now = budget.now()?;
                    let parsed = if response.backend.is_empty()
                        || response.backend.len() > 160
                        || response.raw_response.len() > 16384
                        || serde_json::to_vec(&response.metadata)
                            .map_err(|e| e.to_string())?
                            .len()
                            > 65536
                    {
                        Err("reasoner response exceeds bounds".into())
                    } else {
                        serde_json::from_str::<Recommendation>(&response.raw_response)
                            .map_err(|e| e.to_string())
                            .and_then(|d| {
                                d.validate()?;
                                Ok(d)
                            })
                    };
                    let error = parsed.as_ref().err().map(|e| e.chars().take(600).collect());
                    previous_raw = Some(response.raw_response.chars().take(16384).collect());
                    let backend = response.backend.clone();
                    r.attempts.push(Attempt {
                        repair,
                        received_at_ms: now,
                        response: Some(response),
                        error,
                    });
                    if now >= deadline {
                        break;
                    }
                    if let Ok(rec) = parsed {
                        chosen = Some((rec, backend));
                        break;
                    }
                }
                Err(error) => {
                    let now = budget.now()?;
                    r.attempts.push(Attempt {
                        repair,
                        received_at_ms: now,
                        response: None,
                        error: Some(error.chars().take(600).collect()),
                    });
                    break;
                }
            }
        }
        if budget.remaining()? == 0 {
            return Ok(finish(
                r,
                StageStatus::TimedOut,
                "reasoner_exhausted_event_budget",
                budget.now()?,
            ));
        }
        let (recommendation, backend) = chosen.unwrap_or_else(|| {
            r.degraded_mode = true;
            r.degraded_reasons
                .push("reasoner_failure_threshold_fallback".into());
            (
                crate::reasoner::rule_recommendation(&context, p.fallback_confidence),
                "deterministic-threshold-fallback".into(),
            )
        });
        let local = Candidate {
            source_event_id: event.event_id.clone(),
            location_id: event.location_id.clone(),
            event_started_at_ms: event.started_at_ms,
            recommended_at_ms: budget.now()?,
            recommendation,
            backend,
        };
        r.local_recommendation = Some(local.clone());
        let mut candidates = event.competing.clone();
        candidates.push(local);
        let selected =
            select_candidate(&candidates, event, budget.now()?, p.competition_window_ms)?;
        r.selected_reasoner_output_hash = Some(selected.recommendation.id());
        r.selected_recommendation = Some(selected.clone());
        for role in &r.eligible_roles {
            if event.missing_ballots.contains(role) {
                continue;
            }
            let classification = match role {
                VotingRole::Sensor => sensor.classification,
                VotingRole::Aggregator => fusion.threshold_class,
                VotingRole::Decision => selected.recommendation.classification,
            };
            r.ballots.push(RoleBallot {
                role: *role,
                classification,
            });
        }
        let (_, class, support) = quorum(&r.eligible_roles, &r.ballots)?;
        if budget.remaining()? == 0 {
            return Ok(finish(
                r,
                StageStatus::TimedOut,
                "ballots_exhausted_event_budget",
                budget.now()?,
            ));
        }
        let Some(class) = class else {
            return Ok(finish(
                r,
                StageStatus::Unresolved,
                "insufficient_matching_ballots",
                budget.now()?,
            ));
        };
        r.final_classification = Some(class);
        r.crew_support = Some(support as f64 / r.eligible_roles.len() as f64);
        let final_decision = assemble(&selected.recommendation, class, p);
        let reported_ppm = match p.high_concentration_source {
            GovernanceConcentration::TriggerReading => primary.co2_ppm,
            // Integer thresholds can be compared with the exact quotient floor;
            // this avoids introducing a display-mean rounding decision.
            GovernanceConcentration::WeightedFusion => {
                (fusion.weighted_sum / fusion.weight_sum) as u32
            }
        };
        let governance = validate_governance(
            &final_decision,
            selected.recommendation.confidence,
            reported_ppm,
            r.degraded_mode,
            r.degraded_mode,
            p,
        );
        r.final_decision = Some(final_decision);
        r.governance = Some(governance.clone());
        let approval_time = budget.now()?;
        if approval_time >= deadline {
            return Ok(finish(
                r,
                StageStatus::TimedOut,
                "governance_exhausted_event_budget",
                approval_time,
            ));
        }
        if approval_time >= horizon {
            return Ok(finish(
                r,
                StageStatus::EvidenceExpired,
                "support_expired_before_publication",
                approval_time,
            ));
        }
        if !governance.accepted {
            return Ok(finish(
                r,
                StageStatus::GovernanceRejected,
                &governance.reasons.join(","),
                approval_time,
            ));
        }
        Ok(finish(
            r,
            StageStatus::ApprovedAwaitingRecording,
            "policy_passed_but_authenticated_durable_release_not_yet_integrated",
            approval_time,
        ))
    }
}
fn finish(mut record: StageRecord, status: StageStatus, reason: &str, now: u64) -> StageRecord {
    record.status = status;
    record.reason = reason.into();
    record.completed_at_ms = now;
    record
}
