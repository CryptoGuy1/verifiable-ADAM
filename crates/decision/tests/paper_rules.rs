use std::{cell::Cell, collections::VecDeque};
use veri_adam_decision::{calibration::*, reasoner::*, *};

struct Manual(Cell<u64>);
impl Clock for Manual {
    fn now_ms(&self) -> u64 {
        self.0.get()
    }
}
struct Script<'a> {
    responses: VecDeque<Result<String, String>>,
    clock: &'a Manual,
    advance: u64,
    calls: usize,
    repairs: Vec<bool>,
}
impl Reasoner for Script<'_> {
    fn infer(
        &mut self,
        _: &ReasoningContext,
        repair: Option<&str>,
        _: u64,
    ) -> Result<ReasonerResponse, String> {
        self.calls += 1;
        self.repairs.push(repair.is_some());
        self.clock.0.set(self.clock.0.get() + self.advance);
        let raw = self
            .responses
            .pop_front()
            .unwrap_or_else(|| Err("unavailable".into()))?;
        Ok(ReasonerResponse {
            backend: "test-script-not-a-live-model".into(),
            raw_response: raw,
            metadata: serde_json::json!({"test":true}),
        })
    }
}
fn recommendation(class: Classification) -> Recommendation {
    let p = fixtures::policy();
    let a = p.alignment(class);
    Recommendation {
        classification: class,
        confidence: 0.8,
        severity: a.severity,
        reasoning: "Explicit test reasoner position, independently compared with the role ballots."
            .into(),
        recommended_action: a.action.clone(),
        contributing_factors: vec!["test".into()],
        requires_human_review: false,
    }
}
fn script(clock: &Manual, responses: Vec<Result<Recommendation, String>>) -> Script<'_> {
    Script {
        responses: responses
            .into_iter()
            .map(|r| r.map(|d| serde_json::to_string(&d).unwrap()))
            .collect(),
        clock,
        advance: 0,
        calls: 0,
        repairs: vec![],
    }
}
fn profile() -> CalibrationProfile {
    calibrate(&fixtures::calibration_data()).unwrap()
}
fn input(id: &str, start: u64, a: u32, b: u32) -> EventInput {
    fixtures::event(id, start, &[("sensor-a", a), ("sensor-b", b)])
}

#[test]
fn calibration_follows_sample_residual_variance_not_bias_or_equal_weights() {
    let c = profile();
    assert!(c.synthetic);
    assert_eq!(c.weights[0].paired_count, 500);
    assert!((c.weights[0].residual_variance_ppm2 - 1000.0 / 499.0).abs() < 1e-12);
    assert!((c.weights[1].residual_variance_ppm2 - 4000.0 / 499.0).abs() < 1e-12);
    assert_eq!(
        (c.weights[0].weight_units, c.weights[1].weight_units),
        (800000, 200000)
    );
    let mut biased = fixtures::calibration_data();
    for p in &mut biased.series[0].pairs {
        p.sensor_ppm += 100;
    }
    let b = calibrate(&biased).unwrap();
    assert_eq!(b.weights[0].weight_units, 800000);
    assert_eq!(b.weights[0].residual_mean_ppm, 100.0);
}
#[test]
fn weighted_mean_dispersion_and_missing_source_renormalization_match_hand_vectors() {
    let p = profile();
    let e = input("weighted", 1000, 790, 830);
    let f = fuse(&e.readings, &p, 800).unwrap();
    assert_eq!(f.weighted_sum, 798000000);
    assert_eq!(f.weight_sum, 1000000);
    assert_eq!(f.fused_ppm, 798.0);
    assert_eq!(f.dispersion_ppm, 16.0);
    assert_eq!(f.threshold_class, Classification::Normal);
    assert!(f.quantization_delta_ppm.abs() < 1e-10);
    let single = fuse(&e.readings[1..], &p, 800).unwrap();
    assert_eq!(single.fused_ppm, 830.0);
    assert_eq!(single.weight_sum, 200000);
    assert_eq!(single.dispersion_ppm, 0.0);
    assert_eq!(single.threshold_class, Classification::Anomaly);
    let exact = input("boundary", 1000, 790, 840);
    assert_eq!(
        fuse(&exact.readings, &p, 800).unwrap().threshold_class,
        Classification::Anomaly
    );
}
#[test]
fn invalid_calibration_and_source_substitution_are_rejected() {
    let mut data = fixtures::calibration_data();
    data.series[0].pairs = vec![
        PairedObservation {
            sensor_ppm: 700,
            reference_ppm: 600
        };
        2
    ];
    assert!(
        calibrate(&data)
            .unwrap_err()
            .contains("zero residual variance")
    );
    let mut data = fixtures::calibration_data();
    data.series[1].source_id = "sensor-a".into();
    assert!(calibrate(&data).is_err());
    let mut data = fixtures::calibration_data();
    data.series[0].pairs[0].sensor_ppm = 40001;
    assert!(calibrate(&data).is_err());
    let mut e = input("unknown", 1000, 790, 830);
    e.readings[1].source_id = "not-calibrated".into();
    assert!(fuse(&e.readings, &profile(), 800).is_err());
}
#[test]
fn largest_remainder_is_deterministic_and_profiles_do_not_reweight_per_event() {
    let mut d = fixtures::calibration_data();
    d.series[1].pairs = d.series[0].pairs.clone();
    let mut third = d.series[0].clone();
    third.source_id = "sensor-c".into();
    d.series.push(third);
    let p = calibrate(&d).unwrap();
    assert_eq!(
        p.weights.iter().map(|w| w.weight_units).collect::<Vec<_>>(),
        vec![333334, 333333, 333333]
    );
    let original = p.id();
    let e = input("fixed", 1000, 600, 1000);
    let _ = fuse(&e.readings, &p, 800).unwrap();
    assert_eq!(p.id(), original);
}
#[test]
fn sensor_uses_provided_temporal_baseline_instead_of_copying_the_reasoner() {
    let mut e = input("spike", 1000, 700, 400);
    e.temporal_window = vec![PastReading {
        evidence_id: "past-300".into(),
        source_id: "sensor-a".into(),
        captured_at_ms: 999,
        co2_ppm: 300,
    }];
    let clock = Manual(Cell::new(1000));
    let mut model = script(&clock, vec![Ok(recommendation(Classification::Normal))]);
    let r = DecisionEngine::new()
        .evaluate(&e, &fixtures::policy(), &profile(), &mut model, &clock)
        .unwrap();
    assert_eq!(r.sensor_assessment.as_ref().unwrap().baseline_ppm, 300.0);
    assert_eq!(r.ballots[0].classification, Classification::Anomaly);
    assert_eq!(r.ballots[1].classification, Classification::Normal);
    assert_eq!(r.final_classification, Some(Classification::Normal));
    assert_eq!(r.status, StageStatus::ApprovedAwaitingRecording);
}
#[test]
fn internal_history_is_prior_only_bounded_and_separated_by_session() {
    let mut engine = DecisionEngine::new();
    let clock = Manual(Cell::new(1000));
    let mut reasoner = RuleReasoner { confidence: 0.8 };
    for i in 0..8 {
        let e = input(&format!("history-{i}"), 1000 + i * 100, 300, 400);
        clock.0.set(e.started_at_ms);
        engine
            .evaluate(&e, &fixtures::policy(), &profile(), &mut reasoner, &clock)
            .unwrap();
    }
    let e = input("current", 1900, 700, 400);
    clock.0.set(1900);
    let r = engine
        .evaluate(&e, &fixtures::policy(), &profile(), &mut reasoner, &clock)
        .unwrap();
    let a = r.sensor_assessment.unwrap();
    assert_eq!(a.baseline_origin, "internal_history");
    assert_eq!(a.baseline_evidence_ids.len(), 6);
    assert_eq!(a.classification, Classification::Anomaly);
    let mut e = input("new-session", 2000, 700, 400);
    e.session_id = "next-session".into();
    clock.0.set(2000);
    let r = engine
        .evaluate(&e, &fixtures::policy(), &profile(), &mut reasoner, &clock)
        .unwrap();
    assert_eq!(
        r.sensor_assessment.unwrap().baseline_origin,
        "current_reading_initialization"
    );
}
#[test]
fn authentic_majority_may_disagree_with_fusion_without_being_forged() {
    let e = input("legitimate-disagreement", 1000, 790, 900);
    let clock = Manual(Cell::new(1000));
    let mut model = script(&clock, vec![Ok(recommendation(Classification::Normal))]);
    let r = DecisionEngine::new()
        .evaluate(&e, &fixtures::policy(), &profile(), &mut model, &clock)
        .unwrap();
    assert_eq!(r.fusion.as_ref().unwrap().fused_ppm, 812.0);
    assert_eq!(
        r.fusion.as_ref().unwrap().threshold_class,
        Classification::Anomaly
    );
    assert_eq!(r.final_classification, Some(Classification::Normal));
    assert_eq!(r.crew_support, Some(2.0 / 3.0));
    assert_eq!(r.status, StageStatus::ApprovedAwaitingRecording);
    assert!(!r.action_released && !r.authenticated_record && !r.private_proof_verified);

    let mut e = input("temporal-majority", 1000, 700, 400);
    e.temporal_window = vec![PastReading {
        evidence_id: "past-300".into(),
        source_id: "sensor-a".into(),
        captured_at_ms: 999,
        co2_ppm: 300,
    }];
    let mut model = script(&clock, vec![Ok(recommendation(Classification::Anomaly))]);
    let r = DecisionEngine::new()
        .evaluate(&e, &fixtures::policy(), &profile(), &mut model, &clock)
        .unwrap();
    assert_eq!(r.fusion.unwrap().threshold_class, Classification::Normal);
    assert_eq!(r.final_classification, Some(Classification::Anomaly));
    assert_eq!(r.status, StageStatus::ApprovedAwaitingRecording);
}

#[test]
fn the_governance_concentration_source_is_an_explicit_policy_choice() {
    let clock = Manual(Cell::new(1000));
    let mut p = fixtures::policy();
    p.high_concentration_ppm = 850;
    let mut r = recommendation(Classification::Anomaly);
    r.recommended_action = "ContinueMonitoring".into();
    let e = input("guard-input", 1000, 900, 500);
    let mut model = script(&clock, vec![Ok(r.clone())]);
    let a = DecisionEngine::new()
        .evaluate(&e, &p, &profile(), &mut model, &clock)
        .unwrap();
    assert_eq!(a.status, StageStatus::GovernanceRejected);
    p.high_concentration_source = GovernanceConcentration::WeightedFusion;
    let mut model = script(&clock, vec![Ok(r)]);
    let b = DecisionEngine::new()
        .evaluate(&e, &p, &profile(), &mut model, &clock)
        .unwrap();
    assert_eq!(b.status, StageStatus::ApprovedAwaitingRecording);
    assert_ne!(a.policy_hash, b.policy_hash);
}
#[test]
fn crew_override_aligns_action_but_keeps_selected_reasoner_confidence() {
    let e = input("crew-override", 1000, 900, 500);
    let clock = Manual(Cell::new(1000));
    let mut rec = recommendation(Classification::Normal);
    rec.confidence = 0.42;
    let mut model = script(&clock, vec![Ok(rec)]);
    let r = DecisionEngine::new()
        .evaluate(&e, &fixtures::policy(), &profile(), &mut model, &clock)
        .unwrap();
    let selected = &r.selected_recommendation.as_ref().unwrap().recommendation;
    let d = r.final_decision.unwrap();
    assert_eq!(selected.classification, Classification::Normal);
    assert_eq!(d.classification, Classification::Anomaly);
    assert_eq!(d.confidence, 0.42);
    assert_eq!(d.severity, Severity::Low);
    assert_eq!(d.recommended_action, "VentilationAdvisory");
    assert_eq!(r.crew_support, Some(2.0 / 3.0));
}
#[test]
fn crew_formation_and_quorum_are_fixed_not_lowered_by_missing_ballots() {
    let p = fixtures::policy();
    let c = profile();
    let clock = Manual(Cell::new(1000));
    let mut model = RuleReasoner { confidence: 0.8 };
    let mut e = input("missing", 1000, 790, 900);
    e.missing_ballots.insert(VotingRole::Decision);
    let r = DecisionEngine::new()
        .evaluate(&e, &p, &c, &mut model, &clock)
        .unwrap();
    assert_eq!(r.quorum, 2);
    assert_eq!(r.eligible_roles.len(), 3);
    assert_eq!(r.status, StageStatus::Unresolved);
    let mut e = input("degraded", 1000, 900, 850);
    e.availability.aggregator = false;
    let r = DecisionEngine::new()
        .evaluate(&e, &p, &c, &mut model, &clock)
        .unwrap();
    assert_eq!(
        r.eligible_roles,
        vec![VotingRole::Sensor, VotingRole::Decision]
    );
    assert_eq!(r.quorum, 2);
    assert!(r.degraded_mode);
    e.event_id = "degraded-missing".into();
    e.missing_ballots.insert(VotingRole::Decision);
    assert_eq!(
        DecisionEngine::new()
            .evaluate(&e, &p, &c, &mut model, &clock)
            .unwrap()
            .status,
        StageStatus::Unresolved
    );
    e.availability.sensor = false;
    assert_eq!(
        DecisionEngine::new()
            .evaluate(&e, &p, &c, &mut model, &clock)
            .unwrap()
            .status,
        StageStatus::FormationFailed
    );
    let eligible = vec![
        VotingRole::Sensor,
        VotingRole::Aggregator,
        VotingRole::Decision,
    ];
    let b = RoleBallot {
        role: VotingRole::Sensor,
        classification: Classification::Normal,
    };
    assert!(quorum(&eligible, &[b.clone(), b]).is_err());
}
#[test]
fn competing_recommendation_selection_prioritizes_severity_then_recency() {
    let e = input("local", 1000, 600, 620);
    let candidate = |id: &str, severity, time| {
        let mut r = recommendation(Classification::Anomaly);
        r.severity = severity;
        Candidate {
            source_event_id: id.into(),
            location_id: e.location_id.clone(),
            event_started_at_ms: 900,
            recommended_at_ms: time,
            recommendation: r,
            backend: "test".into(),
        }
    };
    let high = candidate("high", Severity::High, 950);
    let low = candidate("low", Severity::Low, 1000);
    assert_eq!(
        select_candidate(&[low, high.clone()], &e, 1000, 1000)
            .unwrap()
            .source_event_id,
        "high"
    );
    let later = candidate("later-high", Severity::High, 999);
    assert_eq!(
        select_candidate(&[later, high], &e, 1000, 1000)
            .unwrap()
            .source_event_id,
        "later-high"
    );
    let future = candidate("future", Severity::High, 1001);
    assert!(select_candidate(&[future], &e, 1000, 1000).is_err());
}
#[test]
fn selected_recommendation_provenance_is_retained_when_crew_changes_its_class() {
    let mut e = input("local", 1000, 600, 620);
    let mut rec = recommendation(Classification::Anomaly);
    rec.severity = Severity::High;
    rec.confidence = 0.7;
    e.competing.push(Candidate {
        source_event_id: "competing-origin".into(),
        location_id: e.location_id.clone(),
        event_started_at_ms: 900,
        recommended_at_ms: 950,
        recommendation: rec,
        backend: "other-test-reasoner".into(),
    });
    let clock = Manual(Cell::new(1000));
    let mut model = RuleReasoner { confidence: 0.8 };
    let r = DecisionEngine::new()
        .evaluate(&e, &fixtures::policy(), &profile(), &mut model, &clock)
        .unwrap();
    assert_eq!(
        r.selected_recommendation.as_ref().unwrap().source_event_id,
        "competing-origin"
    );
    assert_eq!(
        r.local_recommendation
            .as_ref()
            .unwrap()
            .recommendation
            .classification,
        Classification::Normal
    );
    assert_eq!(
        r.final_decision.as_ref().unwrap().classification,
        Classification::Normal
    );
    assert_eq!(r.final_decision.as_ref().unwrap().confidence, 0.7);
}
#[test]
fn malformed_output_has_exactly_one_repair_then_explicit_fallback() {
    let clock = Manual(Cell::new(1000));
    let mut m = script(&clock, vec![]);
    m.responses = VecDeque::from([Ok("not json".into()), Ok("{}".into())]);
    let r = DecisionEngine::new()
        .evaluate(
            &input("repair", 1000, 900, 850),
            &fixtures::policy(),
            &profile(),
            &mut m,
            &clock,
        )
        .unwrap();
    assert_eq!(m.calls, 2);
    assert_eq!(m.repairs, vec![false, true]);
    assert!(r.degraded_mode);
    assert_eq!(
        r.local_recommendation.unwrap().backend,
        "deterministic-threshold-fallback"
    );
    assert_eq!(r.attempts.len(), 2);
    assert!(r.attempts.iter().all(|a| a.error.is_some()));
    assert_eq!(r.status, StageStatus::ApprovedAwaitingRecording);
}
#[test]
fn successful_repair_is_not_a_failure_fallback_and_low_confidence_is_not_repaired() {
    let clock = Manual(Cell::new(1000));
    let mut m = script(&clock, vec![Ok(recommendation(Classification::Anomaly))]);
    m.responses.push_front(Ok("bad json".into()));
    let r = DecisionEngine::new()
        .evaluate(
            &input("repaired", 1000, 900, 850),
            &fixtures::policy(),
            &profile(),
            &mut m,
            &clock,
        )
        .unwrap();
    assert_eq!(m.calls, 2);
    assert!(!r.degraded_mode);
    let mut low = recommendation(Classification::Normal);
    low.confidence = 0.34;
    let mut m = script(&clock, vec![Ok(low)]);
    let r = DecisionEngine::new()
        .evaluate(
            &input("low-confidence", 1000, 600, 620),
            &fixtures::policy(),
            &profile(),
            &mut m,
            &clock,
        )
        .unwrap();
    assert_eq!(m.calls, 1);
    assert_eq!(r.status, StageStatus::GovernanceRejected);
    assert!(
        r.governance
            .unwrap()
            .reasons
            .contains(&"reasoner_confidence_below_floor".into())
    );
}
#[test]
fn model_failure_falls_back_but_exhausted_budget_never_does() {
    let clock = Manual(Cell::new(1000));
    let mut m = script(&clock, vec![Err("model unavailable".into())]);
    let r = DecisionEngine::new()
        .evaluate(
            &input("unavailable", 1000, 900, 850),
            &fixtures::policy(),
            &profile(),
            &mut m,
            &clock,
        )
        .unwrap();
    assert_eq!(m.calls, 1);
    assert!(r.degraded_mode);
    let mut m = script(&clock, vec![Ok(recommendation(Classification::Anomaly))]);
    m.advance = 30000;
    let r = DecisionEngine::new()
        .evaluate(
            &input("timeout", 1000, 900, 850),
            &fixtures::policy(),
            &profile(),
            &mut m,
            &clock,
        )
        .unwrap();
    assert_eq!(r.status, StageStatus::TimedOut);
    assert!(r.local_recommendation.is_none());
    assert!(!r.action_released);
}
#[test]
fn governance_rejects_unpermitted_passive_critical_and_undisclosed_states() {
    let p = fixtures::policy();
    let mut r = recommendation(Classification::Anomaly);
    r.recommended_action = "UnknownAction".into();
    assert!(
        validate_governance(&r, 0.8, 900, false, false, &p)
            .reasons
            .contains(&"action_not_permitted".into())
    );
    r.recommended_action = "ContinueMonitoring".into();
    assert!(
        validate_governance(&r, 0.8, 5000, false, false, &p)
            .reasons
            .contains(&"passive_action_at_high_concentration".into())
    );
    r.recommended_action = "VentilationAdvisory".into();
    r.severity = Severity::Critical;
    assert!(
        validate_governance(&r, 0.8, 900, false, false, &p)
            .reasons
            .contains(&"critical_event_requires_review".into())
    );
    r.requires_human_review = true;
    assert!(
        validate_governance(&r, 0.8, 900, true, false, &p)
            .reasons
            .contains(&"degraded_anomaly_not_disclosed".into())
    );
    assert!(validate_governance(&r, 0.8, 900, true, true, &p).accepted);
    assert!(
        validate_governance(&r, 0.7, 900, false, false, &p)
            .reasons
            .contains(&"reasoner_confidence_was_reinterpreted".into())
    );
}
#[test]
fn expired_future_and_repeated_events_cannot_be_approved() {
    let mut p = fixtures::policy();
    p.evidence_ttl_ms = 500;
    let clock = Manual(Cell::new(1500));
    let mut m = RuleReasoner { confidence: 0.8 };
    let e = input("expired", 1000, 900, 850);
    assert_eq!(
        DecisionEngine::new()
            .evaluate(&e, &p, &profile(), &mut m, &clock)
            .unwrap()
            .status,
        StageStatus::EvidenceExpired
    );
    let clock_after_inference = Manual(Cell::new(1000));
    let mut delayed = script(
        &clock_after_inference,
        vec![Ok(recommendation(Classification::Anomaly))],
    );
    delayed.advance = 500;
    let r = DecisionEngine::new()
        .evaluate(&e, &p, &profile(), &mut delayed, &clock_after_inference)
        .unwrap();
    assert_eq!(r.status, StageStatus::EvidenceExpired);
    assert!(!r.action_released);
    let mut e = input("future", 1500, 900, 850);
    e.readings[0].captured_at_ms = 1501;
    assert!(
        DecisionEngine::new()
            .evaluate(&e, &p, &profile(), &mut m, &clock)
            .is_err()
    );
    let e = input("duplicate", 1500, 900, 850);
    let mut engine = DecisionEngine::new();
    engine.evaluate(&e, &p, &profile(), &mut m, &clock).unwrap();
    assert!(
        engine
            .evaluate(&e, &p, &profile(), &mut m, &clock)
            .unwrap_err()
            .contains("already processed")
    );
}
