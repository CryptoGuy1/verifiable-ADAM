use std::collections::BTreeMap;
use veri_adam_decision::{calibration::calibrate, fixtures, reasoner::RuleReasoner, *};
use veri_adam_records::{canonical, private, *};
struct Fixed;
impl Clock for Fixed {
    fn now_ms(&self) -> u64 {
        10000
    }
}
struct Case {
    r: Record,
    d: Disclosure,
    t: Trust,
    q: Request,
    keys: Keyring,
}
fn make(a: u32, b: u32) -> Case {
    let (keys, registry) = Keyring::local().unwrap();
    let t = Trust {
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
    let event = fixtures::event("private-test", 10000, &[("sensor-a", a), ("sensor-b", b)]);
    let stage = DecisionEngine::new()
        .evaluate(
            &event,
            &t.policy,
            &t.calibration,
            &mut RuleReasoner { confidence: 0.8 },
            &Fixed,
        )
        .unwrap();
    let (r, d) = attest(&event, &stage, &t, &keys).unwrap();
    let heads = r
        .observations
        .iter()
        .map(|o| {
            (
                o.payload.source_id.clone(),
                canonical::id("observation", &o.payload),
            )
        })
        .collect();
    let q = Request {
        context: Context::from(&event),
        now_ms: 10000,
        current: true,
        heads,
    };
    let mut c = Case { r, d, t, q, keys };
    c.r.proposal.payload.proof_statement = PRIVATE_STATEMENT.into();
    resign(&mut c);
    c
}
fn resign(c: &mut Case) {
    let id = canonical::id("proposal", &c.r.proposal.payload);
    c.r.proposal = c
        .keys
        .signed("coordinator", "proposal", c.r.proposal.payload.clone())
        .unwrap();
    for b in &mut c.r.ballots {
        b.payload.proposal_id = id.clone();
        *b = c
            .keys
            .signed(&b.payload.voter_id, "ballot", b.payload.clone())
            .unwrap();
    }
}
#[test]
fn weighted_both_readings_prove_both_sides_and_exact_boundary() {
    for (a, b, expected) in [
        (790, 830, Classification::Normal),
        (800, 800, Classification::Anomaly),
        (0, 0, Classification::Normal),
        (40000, 40000, Classification::Anomaly),
    ] {
        let c = make(a, b);
        assert_eq!(c.r.proposal.payload.fusion_classification, expected);
        let proof = private::prove(&c.r, &c.d, &c.t, &c.q).unwrap();
        let result = private::check(&c.r, &proof, &c.t, &c.q);
        assert!(result.accepted, "{}", result.reason);
        assert!(result.private_fusion_verified);
        assert!(!result.full_decision_processing_verified && !result.action_released);
        let v = serde_json::to_value(&proof).unwrap();
        assert_eq!(v.as_object().unwrap().len(), 3);
        assert!(v.get("co2_ppm").is_none() && v.get("blinding").is_none());
    }
}
#[test]
fn a_false_weighted_predicate_cannot_be_proved() {
    let mut c = make(790, 830);
    c.r.proposal.payload.fusion_classification = Classification::Anomaly;
    resign(&mut c);
    assert!(private::prove(&c.r, &c.d, &c.t, &c.q).is_err());
}
#[test]
fn corrupted_proof_replaced_commitment_and_event_substitution_fail() {
    let c = make(900, 500);
    let proof = private::prove(&c.r, &c.d, &c.t, &c.q).unwrap();
    let mut bad = proof.clone();
    bad.proof_hex.replace_range(0..2, "ff");
    assert!(!private::check(&c.r, &bad, &c.t, &c.q).accepted);
    let other = make(900, 500);
    assert!(!private::check(&other.r, &proof, &other.t, &other.q).accepted);
    let mut c = c;
    c.r.proposal.payload.context.nonce = "ff".repeat(32);
    c.q.context = c.r.proposal.payload.context.clone();
    resign(&mut c);
    let mut rebound = proof;
    rebound.proposal_id = canonical::id("proposal", &c.r.proposal.payload);
    assert!(!private::check(&c.r, &rebound, &c.t, &c.q).accepted);
}
#[test]
fn proof_does_not_override_freshness_revocation_or_registered_weights() {
    let mut c = make(900, 500);
    let proof = private::prove(&c.r, &c.d, &c.t, &c.q).unwrap();
    c.q.now_ms = 40000;
    assert!(!private::check(&c.r, &proof, &c.t, &c.q).accepted);
    c.q.current = false;
    assert!(private::check(&c.r, &proof, &c.t, &c.q).accepted);
    c.q.current = true;
    c.q.now_ms = 10000;
    c.q.heads.clear();
    assert!(!private::check(&c.r, &proof, &c.t, &c.q).accepted);
    c.t.registry.members[0].revoked = true;
    assert!(!private::check(&c.r, &proof, &c.t, &c.q).accepted);
    let mut c = make(900, 500);
    c.r.proposal.payload.weights[0].units = 500000;
    resign(&mut c);
    assert!(private::prove(&c.r, &c.d, &c.t, &c.q).is_err());
}
#[test]
fn missing_or_wrong_openings_cannot_generate_the_statement() {
    let mut c = make(900, 500);
    c.d.openings[0].co2_ppm = 0;
    assert!(private::prove(&c.r, &c.d, &c.t, &c.q).is_err());
    c.d.openings.clear();
    assert!(private::prove(&c.r, &c.d, &c.t, &c.q).is_err());
}
