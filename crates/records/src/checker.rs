//! Independently recomputed rules. Does not call DecisionEngine, fuse, quorum,
//! sensor_assessment, select_candidate, assemble or validate_governance.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use veri_adam_decision::{GovernanceConcentration, StageStatus};
use veri_adam_protocol::verify;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub outcome: String,
    pub reason: String,
    pub signatures_verified: usize,
    pub fusion_classification: Option<Classification>,
    pub crew_classification: Option<Classification>,
    pub private_proof_verified: bool,
    pub action_released: bool,
}
pub fn check(
    record: &Record,
    disclosure: Option<&Disclosure>,
    trust: &Trust,
    request: &Request,
) -> Report {
    let mut count = 0;
    let result = check_inner(record, disclosure, trust, request, &mut count);
    match result {
        Ok((outcome,fused,crew)) => Report{outcome:outcome.into(),reason:"independent public-disclosure checks passed; durable recording/release not implemented".into(),signatures_verified:count,fusion_classification:Some(fused),crew_classification:Some(crew),private_proof_verified:false,action_released:false},
        Err(reason) => Report{outcome:if reason.starts_with("unconfirmed:") {"unconfirmed"} else {"rejected"}.into(),reason,signatures_verified:count,fusion_classification:None,crew_classification:None,private_proof_verified:false,action_released:false}
    }
}
fn require(ok: bool, reason: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(reason.into()) }
}
fn signature<T: Serialize>(
    s: &Signed<T>,
    kind: &str,
    key: &str,
    count: &mut usize,
) -> Result<(), String> {
    verify(&canonical::bytes(kind, &s.payload), &s.signature, key)?;
    *count += 1;
    Ok(())
}
fn check_inner(
    r: &Record,
    d: Option<&Disclosure>,
    t: &Trust,
    q: &Request,
    count: &mut usize,
) -> Result<(&'static str, Classification, Classification), String> {
    t.validate()?;
    q.context.validate()?;
    let p = &r.proposal.payload;
    p.context.validate()?;
    require(
        p.protocol_version == 2 && p.context == q.context,
        "event/version/challenge mismatch",
    )?;
    require(
        p.coordinator_id == t.coordinator
            && p.registry_hash == t.registry.id()
            && p.policy_hash == t.policy_id()
            && p.calibration_hash == t.calibration_id(),
        "trusted configuration binding mismatch",
    )?;
    signature(
        &r.proposal,
        "proposal",
        t.registry.key(&p.coordinator_id, Role::Coordinator)?,
        count,
    )?;
    require(
        p.quorum_rule == QUORUM_RULE
            && p.proof_statement == "weighted-fusion-two-committed-co2/v1:reserved-unimplemented",
        "unsupported quorum/proof statement",
    )?;
    let d = d.ok_or("unconfirmed: disclosure required; private proof path is not implemented")?;
    require(
        d.event.readings.len() <= 32
            && d.event.temporal_window.len() <= 4096
            && d.event.competing.len() <= 31
            && r.observations.len() <= 4128
            && r.ballots.len() <= 3,
        "record size bounds exceeded",
    )?;
    // Authenticate every ballot before any stale/head/no-quorum UNCONFIRMED
    // result: expiry must not hide an invalid supplied signature.
    let proposal_id = canonical::id("proposal", p);
    let mut authenticated_roles = BTreeSet::new();
    for ballot in &r.ballots {
        let b = &ballot.payload;
        require(
            b.proposal_id == proposal_id
                && t.voters.get(&b.role) == Some(&b.voter_id)
                && authenticated_roles.insert(b.role),
            "ballot binding/eligibility mismatch",
        )?;
        signature(
            ballot,
            "ballot",
            t.registry.key(&b.voter_id, b.role.into())?,
            count,
        )?;
    }
    decode::<32>(&d.trace_salt)?;
    decode::<32>(&d.reasoner_salt)?;
    require(
        Context::from(&d.event) == p.context,
        "disclosed event context mismatch",
    )?;
    require(
        canonical::hidden("trace-commitment", &d.trace_salt, &(&d.event, &d.stage))
            == p.trace_commitment,
        "trace commitment mismatch",
    )?;
    let e = &d.event;
    let s = &d.stage;
    require(
        s.event_id == e.event_id
            && s.event_nonce == e.nonce
            && s.session_id == e.session_id
            && s.location_id == e.location_id
            && s.started_at_ms == e.started_at_ms
            && s.completed_at_ms == p.completed_at_ms
            && s.deadline_ms == p.deadline_ms,
        "stage context/time mismatch",
    )?;
    require(
        s.record_kind == "adam-decision-stage/v1"
            && !s.action_released
            && !s.durable_recording_acknowledged
            && !s.private_proof_verified
            && !s.authenticated_record,
        "unsupported stage assurance/release claim",
    )?;
    require(
        s.policy_hash == t.policy.id() && s.calibration_hash == t.calibration.id(),
        "stage configuration mismatch",
    )?;
    let deadline = e
        .started_at_ms
        .checked_add(t.policy.event_budget_ms)
        .ok_or("deadline overflow")?;
    require(
        deadline == p.deadline_ms
            && p.completed_at_ms >= e.started_at_ms
            && p.completed_at_ms < deadline
            && q.now_ms >= p.completed_at_ms,
        "invalid decision deadline/completion",
    )?;
    let weights: Vec<_> = t
        .calibration
        .weights
        .iter()
        .map(|w| Weight {
            source_id: w.source_id.clone(),
            units: w.weight_units,
        })
        .collect();
    require(p.weights == weights, "registered weights mismatch")?;
    let mut ids = Vec::new();
    let mut openings = BTreeMap::new();
    for o in &d.openings {
        require(
            openings.insert(o.observation_id.clone(), o).is_none(),
            "duplicate opening",
        )?;
    }
    let expected_count = e.readings.len() + e.temporal_window.len();
    require(
        r.observations.len() == expected_count && openings.len() == expected_count,
        "missing/extra authenticated support",
    )?;
    let mut seen_ord = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();
    let mut sum = 0u64;
    let mut total = 0u64;
    let mut horizon = u64::MAX;
    let mut missing_or_replaced_head = false;
    for o in &r.observations {
        let a = &o.payload;
        let id = canonical::id("observation", a);
        ids.push(id.clone());
        require(
            a.context == p.context
                && a.calibration_hash == p.calibration_hash
                && a.commitment_scheme == COMMITMENT_SCHEME
                && seen_ord.insert(a.ordinal),
            "observation binding/ordinal mismatch",
        )?;
        signature(
            o,
            "observation",
            t.registry.key(&a.source_id, Role::Source)?,
            count,
        )?;
        let opening = openings.get(&id).ok_or("missing commitment opening")?;
        require(
            super::seal::commitment(opening.co2_ppm, &opening.blinding)? == a.commitment,
            "commitment opening mismatch",
        )?;
        let i = a.ordinal as usize;
        if i < e.readings.len() {
            let v = &e.readings[i];
            v.validate()?;
            require(
                a.purpose == Purpose::Fusion
                    && a.source_id == v.source_id
                    && a.sequence == Some(v.sequence)
                    && a.captured_at_ms == v.captured_at_ms
                    && a.data_kind == v.data_kind
                    && opening.co2_ppm == v.co2_ppm,
                "fusion support substitution",
            )?;
            require(
                source_ids.insert(&v.source_id) && evidence_ids.insert(&v.evidence_id),
                "duplicate source/evidence",
            )?;
            require(v.captured_at_ms <= e.started_at_ms, "future capture")?;
            let expiry = v
                .captured_at_ms
                .checked_add(t.policy.evidence_ttl_ms)
                .ok_or("expiry overflow")?;
            horizon = horizon.min(expiry);
            if q.current && q.heads.get(&v.source_id) != Some(&id) {
                missing_or_replaced_head = true;
            }
            let w = t
                .calibration
                .weights
                .iter()
                .find(|w| w.source_id == v.source_id)
                .ok_or("unregistered fusion source")?;
            sum += u64::from(w.weight_units) * u64::from(v.co2_ppm);
            total += u64::from(w.weight_units);
        } else {
            let v = e
                .temporal_window
                .get(i - e.readings.len())
                .ok_or("unexpected observation ordinal")?;
            require(
                a.purpose == Purpose::TemporalBaseline
                    && a.source_id == v.source_id
                    && a.sequence.is_none()
                    && a.captured_at_ms == v.captured_at_ms
                    && a.data_kind == "historical_baseline"
                    && opening.co2_ppm == v.co2_ppm,
                "temporal support substitution",
            )?;
            require(
                evidence_ids.insert(&v.evidence_id),
                "duplicate temporal evidence",
            )?;
        }
    }
    ids.sort();
    require(
        ids == p.observation_ids && ids.windows(2).all(|w| w[0] < w[1]),
        "observation ID set mismatch",
    )?;
    require(
        total > 0 && p.completed_at_ms < horizon,
        "empty or expired decision support",
    )?;
    if missing_or_replaced_head {
        return Err("unconfirmed: support head absent or replaced".into());
    }
    if q.current && q.now_ms >= horizon {
        return Err("unconfirmed: support expired at trusted verifier time".into());
    }
    let fused = Classification::from_anomaly(sum >= u64::from(t.policy.threshold_ppm) * total);
    require(
        p.fusion_classification == fused,
        "weighted fused classification mismatch",
    )?;
    let f = s.fusion.as_ref().ok_or("missing fusion transcript")?;
    let mean = sum as f64 / total as f64;
    let mut unquantized_sum = 0.0;
    let mut unquantized_weight = 0.0;
    for v in &e.readings {
        let w = t
            .calibration
            .weights
            .iter()
            .find(|w| w.source_id == v.source_id)
            .unwrap();
        unquantized_sum += w.normalized_inverse_variance * f64::from(v.co2_ppm);
        unquantized_weight += w.normalized_inverse_variance;
    }
    let unquantized_mean = unquantized_sum / unquantized_weight;
    let variance = e
        .readings
        .iter()
        .map(|v| {
            let w = t
                .calibration
                .weights
                .iter()
                .find(|w| w.source_id == v.source_id)
                .unwrap();
            f64::from(w.weight_units) * (f64::from(v.co2_ppm) - mean).powi(2)
        })
        .sum::<f64>()
        / total as f64;
    let mut sources: Vec<_> = source_ids.into_iter().cloned().collect();
    sources.sort();
    require(
        f.weighted_sum == sum
            && f.weight_sum == total
            && f.threshold_class == fused
            && f.contributing_sources == sources
            && f.calibration_hash == t.calibration.id()
            && near(f.fused_ppm, mean)
            && near(f.unquantized_fused_ppm, unquantized_mean)
            && near(f.quantization_delta_ppm, mean - unquantized_mean)
            && near(f.dispersion_ppm, variance.sqrt()),
        "fusion transcript mismatch",
    )?;
    let primary = e
        .readings
        .iter()
        .find(|v| v.source_id == e.trigger_source)
        .ok_or("missing trigger")?;
    require(
        s.trigger_evidence_id.as_ref() == Some(&primary.evidence_id)
            && s.support_evidence_ids
                == e.readings
                    .iter()
                    .map(|v| v.evidence_id.clone())
                    .collect::<Vec<_>>(),
        "support/trigger reference mismatch",
    )?;
    // A stateful producer must export its actual prior baseline as event support.
    // Never accept an unauthenticated internal-history number from its stage record.
    let assessment = s
        .sensor_assessment
        .as_ref()
        .ok_or("missing sensor assessment")?;
    if e.temporal_window.is_empty() && !assessment.baseline_evidence_ids.is_empty() {
        return Err(
            "unconfirmed: producer must disclose and authenticate internal-history support".into(),
        );
    }
    let mut history_sum = 0u64;
    for v in &e.temporal_window {
        identifier(&v.evidence_id)?;
        require(
            v.source_id == primary.source_id
                && v.captured_at_ms < primary.captured_at_ms
                && v.co2_ppm <= 40000,
            "invalid temporal baseline",
        )?;
        history_sum += u64::from(v.co2_ppm);
    }
    let n = e.temporal_window.len() as u64;
    let baseline = if n == 0 {
        f64::from(primary.co2_ppm)
    } else {
        history_sum as f64 / n as f64
    };
    let sensor = Classification::from_anomaly(
        primary.co2_ppm >= t.policy.threshold_ppm
            || if n == 0 {
                primary.co2_ppm == 0
            } else {
                u64::from(primary.co2_ppm) * n >= 2 * history_sum
            },
    );
    let baseline_ids: Vec<_> = e
        .temporal_window
        .iter()
        .map(|v| v.evidence_id.clone())
        .collect();
    require(
        assessment.classification == sensor
            && near(assessment.baseline_ppm, baseline)
            && assessment.baseline_evidence_ids == baseline_ids
            && assessment.baseline_origin
                == if n == 0 {
                    "current_reading_initialization"
                } else {
                    "event_temporal_window"
                },
        "sensor assessment mismatch",
    )?;
    require(
        e.availability.sensor && e.availability.decision && e.availability.coordinator,
        "required role unavailable at formation",
    )?;
    let eligible = if e.availability.aggregator {
        vec![
            VotingRole::Sensor,
            VotingRole::Aggregator,
            VotingRole::Decision,
        ]
    } else {
        vec![VotingRole::Sensor, VotingRole::Decision]
    };
    let needed = eligible.len() / 2 + 1;
    require(
        p.eligible_roles == eligible
            && s.eligible_roles == eligible
            && p.quorum == needed as u32
            && s.quorum == needed,
        "eligible set/quorum mismatch",
    )?;
    let local = s
        .local_recommendation
        .as_ref()
        .ok_or("missing local recommendation")?;
    let fallback = check_reasoner_trace(s, local, fused, mean, variance.sqrt(), t)?;
    require(
        local.source_event_id == e.event_id && local.event_started_at_ms == e.started_at_ms,
        "local recommendation origin mismatch",
    )?;
    let mut candidates = e.competing.clone();
    candidates.push(local.clone());
    let mut origins = BTreeSet::new();
    for c in &candidates {
        c.recommendation.validate()?;
        require(
            c.location_id == e.location_id
                && c.event_started_at_ms.abs_diff(e.started_at_ms)
                    <= t.policy.competition_window_ms
                && c.recommended_at_ms >= c.event_started_at_ms
                && c.recommended_at_ms <= p.completed_at_ms
                && origins.insert(&c.source_event_id),
            "invalid competing recommendation context",
        )?;
    }
    let selected = candidates
        .iter()
        .max_by_key(|c| {
            (
                c.recommendation.severity,
                c.recommended_at_ms,
                &c.source_event_id,
            )
        })
        .unwrap();
    require(
        canonical::bytes("candidate", selected)
            == canonical::bytes(
                "candidate",
                s.selected_recommendation
                    .as_ref()
                    .ok_or("missing selected recommendation")?,
            ),
        "recommendation selection mismatch",
    )?;
    require(
        s.selected_reasoner_output_hash.as_ref() == Some(&selected.recommendation.id())
            && p.selected == DecisionSummary::from(&selected.recommendation)
            && p.selected_reasoner_commitment
                == canonical::hidden(
                    "reasoner-commitment",
                    &d.reasoner_salt,
                    &selected.recommendation,
                ),
        "reasoner output binding mismatch",
    )?;
    let proposal_id = canonical::id("proposal", p);
    let mut roles = BTreeSet::new();
    let mut yes = 0usize;
    for b in &r.ballots {
        let b0 = &b.payload;
        require(
            b0.proposal_id == proposal_id
                && eligible.contains(&b0.role)
                && roles.insert(b0.role)
                && t.voters.get(&b0.role) == Some(&b0.voter_id),
            "ballot binding/eligibility mismatch",
        )?;
        let expected = match b0.role {
            VotingRole::Sensor => sensor,
            VotingRole::Aggregator => fused,
            VotingRole::Decision => selected.recommendation.classification,
        };
        require(b0.classification == expected, "role assessment mismatch")?;
        yes += usize::from(b0.classification == Classification::Anomaly);
    }
    require(
        r.ballots.len() == s.ballots.len()
            && s.ballots.iter().map(|b| b.role).collect::<BTreeSet<_>>() == roles
            && s.ballots.iter().all(|b| {
                r.ballots.iter().any(|a| {
                    a.payload.role == b.role && a.payload.classification == b.classification
                })
            }),
        "stage ballot transcript mismatch",
    )?;
    let no = r.ballots.len() - yes;
    let crew = if yes >= needed {
        Classification::Anomaly
    } else if no >= needed {
        Classification::Normal
    } else {
        return Err("unconfirmed: fixed eligible quorum not reached".into());
    };
    let mut final_rec = selected.recommendation.clone();
    if crew != final_rec.classification {
        let a = match crew {
            Classification::Normal => &t.policy.normal_alignment,
            Classification::Anomaly => &t.policy.anomaly_alignment,
        };
        final_rec.classification = crew;
        final_rec.severity = a.severity;
        final_rec.recommended_action = a.action.clone();
        if final_rec.severity == Severity::Critical {
            final_rec.requires_human_review = true;
        }
    }
    require(
        s.final_classification == Some(crew)
            && s.final_decision.as_ref() == Some(&final_rec)
            && p.final_decision == Some(DecisionSummary::from(&final_rec))
            && s.crew_support
                .is_some_and(|v| near(v, yes.max(no) as f64 / eligible.len() as f64)),
        "final decision/quorum summary mismatch",
    )?;
    let degraded = !e.availability.aggregator || fallback;
    let expected_degraded_reasons: Vec<String> = [
        (!e.availability.aggregator).then_some("aggregator_unavailable_at_formation"),
        fallback.then_some("reasoner_failure_threshold_fallback"),
    ]
    .into_iter()
    .flatten()
    .map(str::to_owned)
    .collect();
    require(
        p.degraded == s.degraded_mode
            && p.degraded == degraded
            && s.degraded_reasons == expected_degraded_reasons,
        "degraded disclosure mismatch",
    )?;
    let reported = match t.policy.high_concentration_source {
        GovernanceConcentration::TriggerReading => primary.co2_ppm,
        GovernanceConcentration::WeightedFusion => (sum / total) as u32,
    };
    let mut reasons = Vec::<String>::new();
    if selected.recommendation.confidence < t.policy.confidence_floor {
        reasons.push("reasoner_confidence_below_floor".into());
    }
    if !t
        .policy
        .permitted_actions
        .contains(&final_rec.recommended_action)
    {
        reasons.push("action_not_permitted".into());
    }
    if reported >= t.policy.high_concentration_ppm
        && t.policy
            .passive_actions
            .contains(&final_rec.recommended_action)
    {
        reasons.push("passive_action_at_high_concentration".into());
    }
    if final_rec.severity == Severity::Critical && !final_rec.requires_human_review {
        reasons.push("critical_event_requires_review".into());
    }
    require(
        p.governance_accepted == reasons.is_empty() && p.governance_reasons == reasons,
        "governance verdict mismatch",
    )?;
    let g = s
        .governance
        .as_ref()
        .ok_or("missing governance transcript")?;
    require(
        g.accepted == reasons.is_empty() && g.reasons == reasons && s.status == p.stage_status,
        "stage governance mismatch",
    )?;
    require(
        reasons.is_empty() && p.stage_status == StageStatus::ApprovedAwaitingRecording,
        "policy withheld decision",
    )?;
    Ok((
        if q.current {
            "verified_current_awaiting_recording"
        } else {
            "verified_historical_awaiting_recording"
        },
        fused,
        crew,
    ))
}
fn near(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

/// Checks reported response/repair/fallback consistency. This authenticates a
/// role's account of inference; it cannot prove that the model actually ran.
fn check_reasoner_trace(
    s: &StageRecord,
    local: &veri_adam_decision::Candidate,
    fused: Classification,
    mean: f64,
    dispersion: f64,
    t: &Trust,
) -> Result<bool, String> {
    require(
        !s.attempts.is_empty() && s.attempts.len() <= 2,
        "invalid reasoner attempt count",
    )?;
    let mut last = s.started_at_ms;
    let mut valid = None;
    for (i, a) in s.attempts.iter().enumerate() {
        require(
            a.repair == (i == 1)
                && a.received_at_ms >= last
                && a.received_at_ms <= local.recommended_at_ms
                && a.received_at_ms < s.deadline_ms,
            "invalid inference/repair chronology",
        )?;
        if i == 1 {
            require(
                s.attempts[0].response.is_some() && valid.is_none(),
                "repair without malformed first response",
            )?;
        }
        last = a.received_at_ms;
        let parsed = a.response.as_ref().and_then(|r| {
            if r.backend.is_empty()
                || r.backend.len() > 160
                || r.raw_response.len() > 16384
                || serde_json::to_vec(&r.metadata).ok()?.len() > 65536
            {
                return None;
            }
            serde_json::from_str::<Recommendation>(&r.raw_response)
                .ok()
                .filter(|r| r.validate().is_ok())
        });
        if let Some(rec) = parsed {
            require(
                a.error.is_none() && i + 1 == s.attempts.len(),
                "valid response has error or spurious retry",
            )?;
            valid = Some((rec, a.response.as_ref().unwrap().backend.clone()));
        } else {
            require(a.error.is_some(), "failed response has no diagnostic")?;
        }
    }
    if let Some((rec, backend)) = valid {
        require(
            local.recommendation == rec && local.backend == backend,
            "local recommendation differs from recorded reasoner response",
        )?;
        Ok(false)
    } else {
        let a = match fused {
            Classification::Normal => &t.policy.normal_alignment,
            Classification::Anomaly => &t.policy.anomaly_alignment,
        };
        let expected = Recommendation {
            classification: fused,
            confidence: t.policy.fallback_confidence,
            severity: a.severity,
            reasoning: format!(
                "The registered weighted concentration is {mean:.6} ppm against the experimental {} ppm threshold.",
                t.policy.threshold_ppm
            ),
            recommended_action: a.action.clone(),
            contributing_factors: vec!["registered_weighted_concentration".into()],
            requires_human_review: t.policy.fallback_confidence < 0.6
                || dispersion > mean / 2.0
                || a.severity == Severity::Critical,
        };
        require(
            local.backend == "deterministic-threshold-fallback" && local.recommendation == expected,
            "invalid deterministic fallback",
        )?;
        Ok(true)
    }
}
