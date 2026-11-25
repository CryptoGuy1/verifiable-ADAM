use std::{collections::BTreeMap, time::Duration};
use veri_adam_agents::weighted::Runtime;
use veri_adam_decision::{calibration::calibrate, fixtures, reasoner::RuleReasoner, *};
use veri_adam_records::*;
fn runtime(reasoner: Box<dyn Reasoner + Send>) -> Runtime {
    let (keys, registry) = Keyring::local().unwrap();
    let trust = Trust {
        registry,
        policy: fixtures::policy(),
        calibration: calibrate(&fixtures::calibration_data()).unwrap(),
        voters: BTreeMap::from([
            (VotingRole::Sensor, "sensor-role".into()),
            (VotingRole::Aggregator, "aggregator-role".into()),
            (VotingRole::Decision, "decision-role".into()),
        ]),
        coordinator: "coordinator".into(),
    };
    Runtime::new(trust, keys, reasoner).unwrap()
}
#[test]
fn workers_sign_their_own_ballots_and_export_authenticated_history() {
    let mut rt = runtime(Box::new(RuleReasoner { confidence: 0.8 }));
    let first = rt
        .round(
            fixtures::event("first", 10000, &[("sensor-a", 790), ("sensor-b", 830)]),
            false,
        )
        .unwrap();
    assert_eq!(
        first.verification.unwrap().outcome,
        "verified_current_awaiting_recording"
    );
    let record = first.record.unwrap();
    assert_eq!(record.ballots.len(), 3);
    assert_eq!(
        record.proposal.payload.fusion_classification,
        Classification::Normal
    );
    let second = rt
        .round(
            fixtures::event("second", 50000, &[("sensor-a", 900), ("sensor-b", 500)]),
            false,
        )
        .unwrap();
    assert_eq!(second.event.temporal_window.len(), 1);
    assert_eq!(
        second.verification.unwrap().outcome,
        "verified_current_awaiting_recording"
    );
    assert!(
        second
            .messages
            .iter()
            .filter(|m| m.message == "signed-role-ballot")
            .all(|m| m.accepted)
    );
}
#[test]
fn evidence_tampering_stops_before_inference_and_votes() {
    let mut rt = runtime(Box::new(RuleReasoner { confidence: 0.8 }));
    let r = rt
        .round(
            fixtures::event("tampered", 10000, &[("sensor-a", 900), ("sensor-b", 500)]),
            true,
        )
        .unwrap();
    assert!(r.stage.is_none() && r.record.is_none());
    assert!(r.messages.iter().all(|m| m.message == "signed-evidence"));
}
#[test]
fn missing_ballot_keeps_the_original_eligible_set() {
    let mut rt = runtime(Box::new(RuleReasoner { confidence: 0.8 }));
    let mut e = fixtures::event("lost", 10000, &[("sensor-a", 790), ("sensor-b", 900)]);
    e.missing_ballots.insert(VotingRole::Decision);
    let r = rt.round(e, false).unwrap();
    assert_eq!(r.stage.unwrap().eligible_roles.len(), 3);
    assert_eq!(r.verification.unwrap().outcome, "unconfirmed");
}
struct Disagree;
impl Reasoner for Disagree {
    fn infer(
        &mut self,
        c: &ReasoningContext,
        _: Option<&str>,
        _: u64,
    ) -> Result<ReasonerResponse, String> {
        let mut r = veri_adam_decision::reasoner::rule_recommendation(c, 0.8);
        r.classification = Classification::Anomaly;
        r.severity = Severity::Low;
        r.recommended_action = "VentilationAdvisory".into();
        Ok(ReasonerResponse {
            backend: "test-independent-decision".into(),
            raw_response: serde_json::to_string(&r).unwrap(),
            metadata: serde_json::json!({}),
        })
    }
}
#[test]
fn actual_agents_preserve_legitimate_disagreement() {
    let mut rt = runtime(Box::new(Disagree));
    let mut e = fixtures::event("spike", 10000, &[("sensor-a", 700), ("sensor-b", 400)]);
    e.temporal_window.push(PastReading {
        evidence_id: "past".into(),
        source_id: "sensor-a".into(),
        captured_at_ms: 5000,
        co2_ppm: 300,
    });
    let r = rt.round(e, false).unwrap();
    let v = r.verification.unwrap();
    assert_eq!(v.fusion_classification, Some(Classification::Normal));
    assert_eq!(v.crew_classification, Some(Classification::Anomaly));
    assert!(v.outcome.starts_with("verified_"));
}
struct Slow;
impl Reasoner for Slow {
    fn infer(
        &mut self,
        _: &ReasoningContext,
        _: Option<&str>,
        _: u64,
    ) -> Result<ReasonerResponse, String> {
        std::thread::sleep(Duration::from_millis(250));
        Err("injected stall".into())
    }
}
#[test]
fn stalled_decision_actor_cannot_release_or_block_forever() {
    let mut rt = runtime(Box::new(Slow));
    rt.trust.policy.event_budget_ms = 100;
    // Workers deliberately pin their original policy, so use a fresh short-budget runtime.
    let trust = rt.trust.clone();
    let (keys, registry) = Keyring::local().unwrap();
    let mut trust = trust;
    trust.registry = registry;
    let mut rt = Runtime::new(trust, keys, Box::new(Slow)).unwrap();
    let started = std::time::Instant::now();
    let r = rt
        .round(
            fixtures::event("late", 10000, &[("sensor-a", 900), ("sensor-b", 500)]),
            false,
        )
        .unwrap();
    assert!(started.elapsed() < Duration::from_millis(240));
    assert!(
        !r.verification
            .as_ref()
            .is_some_and(|v| v.outcome.starts_with("verified_"))
    );
}
