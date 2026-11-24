use veri_adam_protocol::*;
use veri_adam_verification::check;

fn assert_outcome(p: &Policy, r: &Registry, q: &Request, b: &Bundle, expected: Outcome) {
    let production = check(b, p, r, q);
    assert_eq!(production.outcome, expected, "{}", production.reason);
    assert_eq!(
        veri_adam_reference::check(b, p, r, q),
        expected,
        "reference mismatch"
    );
}
fn resign_decision(b: &mut Bundle) {
    b.decision.signature = sign(&b.decision.payload.canonical_bytes(), &[6; 32]);
    let hash = b.decision.payload.id();
    for (i, v) in b.ballots.iter_mut().enumerate() {
        v.payload.decision_hash = hash.clone();
        v.payload.advisory = b.decision.payload.advisory;
        v.signature = sign(&v.payload.canonical_bytes(), &[(i + 3) as u8; 32]);
    }
}

#[test]
fn threshold_and_quorum_cases() {
    for mode in [CrewMode::NominalThree, CrewMode::DegradedTwo] {
        for (readings, expected_advisory) in [
            ([799, 800], false),
            ([800, 800], true),
            ([0, 1600], true),
            ([0, 0], false),
            ([40_000, 40_000], true),
        ] {
            let (p, r, q, b) = demo::fixture(mode, readings);
            assert_eq!(b.decision.payload.advisory, expected_advisory);
            assert_outcome(&p, &r, &q, &b, Outcome::VerifiedCurrent);
        }
    }
}

#[test]
fn expiration_boundary_and_history() {
    let (p, r, mut q, b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    q.now_ms = 10_999;
    assert_outcome(&p, &r, &q, &b, Outcome::VerifiedCurrent);
    q.now_ms = 11_000;
    assert_outcome(&p, &r, &q, &b, Outcome::Expired);
    q.now_ms = 11_001;
    assert_outcome(&p, &r, &q, &b, Outcome::Expired);
    q.kind = CheckKind::Historical;
    assert_outcome(&p, &r, &q, &b, Outcome::VerifiedHistorical);
}

#[test]
fn missing_and_replaced_heads_never_pass_as_current() {
    let (p, r, mut q, b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    q.heads.remove("sensor-a");
    assert_outcome(&p, &r, &q, &b, Outcome::Unconfirmed);
    q.heads.insert("sensor-a".into(), "00".repeat(32));
    assert_outcome(&p, &r, &q, &b, Outcome::Unconfirmed);
    q.kind = CheckKind::Historical;
    assert_outcome(&p, &r, &q, &b, Outcome::VerifiedHistorical);
}

#[test]
fn unrelated_heads_do_not_invalidate() {
    let (p, r, mut q, b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    q.heads.insert("unrelated".into(), "00".repeat(32));
    assert_outcome(&p, &r, &q, &b, Outcome::VerifiedCurrent);
}

#[test]
fn qualifying_replacement_requires_a_new_authenticated_bundle() {
    let (p, r, mut q, original) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    let mut renewed = original.clone();
    let m = &mut renewed.measurements[0];
    m.payload.sequence = 2;
    m.payload.captured_at_ms = 2_000;
    m.payload.co2_ppm = 950;
    m.signature = sign(&m.payload.canonical_bytes(), &[1; 32]);
    q.heads.insert(m.payload.signer_id.clone(), m.payload.id());
    // The old proposal still has valid historical signatures but is no longer current.
    assert_outcome(&p, &r, &q, &original, Outcome::Unconfirmed);
    // Updating a checkpoint alone does not authenticate a new proposal.
    assert_outcome(&p, &r, &q, &renewed, Outcome::Rejected);
    renewed.decision.payload.measurement_ids = renewed
        .measurements
        .iter()
        .map(|m| m.payload.id())
        .collect();
    renewed.decision.payload.measurement_ids.sort();
    resign_decision(&mut renewed);
    assert_outcome(&p, &r, &q, &renewed, Outcome::VerifiedCurrent);
}

#[test]
fn signatures_and_bindings_attack_corpus() {
    let (p, r, q, original) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    let mut attacks = Vec::new();
    let mut b = original.clone();
    b.measurements[0].payload.co2_ppm += 1;
    attacks.push(b);
    let mut b = original.clone();
    b.measurements[0].payload.captured_at_ms += 1;
    attacks.push(b);
    let mut b = original.clone();
    b.measurements[0].signature = "00".repeat(64);
    attacks.push(b);
    let mut b = original.clone();
    b.measurements[0].payload.signer_id = "impostor".into();
    attacks.push(b);
    let mut b = original.clone();
    b.measurements[0].payload.event_id = "old-event".into();
    attacks.push(b);
    let mut b = original.clone();
    b.measurements[0].payload.nonce = "00".repeat(32);
    attacks.push(b);
    let mut b = original.clone();
    b.ballots[0].payload.advisory = false;
    attacks.push(b);
    let mut b = original.clone();
    b.ballots[0].payload.decision_hash = "00".repeat(32);
    attacks.push(b);
    let mut b = original.clone();
    b.ballots[0].payload.event_id = "old-event".into();
    attacks.push(b);
    let mut b = original.clone();
    b.ballots[0].payload.nonce = "00".repeat(32);
    attacks.push(b);
    let mut b = original.clone();
    b.ballots[0].signature = b.measurements[0].signature.clone();
    attacks.push(b);
    let mut b = original.clone();
    b.ballots[0].payload.voter_id = "sensor-a".into();
    attacks.push(b);
    let mut b = original.clone();
    b.ballots[1] = b.ballots[0].clone();
    attacks.push(b);
    let mut b = original.clone();
    b.decision.signature = "ff".repeat(64);
    attacks.push(b);
    let mut b = original.clone();
    b.decision.payload.registry_version += 1;
    attacks.push(b);
    let mut b = original.clone();
    b.decision.payload.advisory = false;
    resign_decision(&mut b);
    attacks.push(b);
    let mut b = original.clone();
    b.measurements[1] = b.measurements[0].clone();
    attacks.push(b);
    let mut b = original.clone();
    b.measurements.pop();
    attacks.push(b);
    let mut b = original.clone();
    b.decision.payload.measurement_ids[0] = "00".repeat(32);
    resign_decision(&mut b);
    attacks.push(b);
    let mut b = original.clone();
    b.measurements[0].payload.protocol_version = 2;
    attacks.push(b);
    for (i, b) in attacks.iter().enumerate() {
        assert_eq!(
            check(b, &p, &r, &q).outcome,
            Outcome::Rejected,
            "attack {i}"
        );
        assert_eq!(
            veri_adam_reference::check(b, &p, &r, &q),
            Outcome::Rejected,
            "reference attack {i}"
        );
    }
}

#[test]
fn signed_but_invalid_readings_are_rejected() {
    for ppm in [40_001, u32::MAX] {
        let (p, r, q, mut b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
        b.measurements[0].payload.co2_ppm = ppm;
        b.measurements[0].signature = sign(&b.measurements[0].payload.canonical_bytes(), &[1; 32]);
        assert_outcome(&p, &r, &q, &b, Outcome::Rejected);
    }
}

#[test]
fn no_silent_quorum_reduction() {
    for mode in [CrewMode::NominalThree, CrewMode::DegradedTwo] {
        let (p, r, q, mut b) = demo::fixture(mode, [900, 850]);
        b.ballots.truncate(1);
        assert_outcome(&p, &r, &q, &b, Outcome::Unconfirmed);
        b.ballots.clear();
        assert_outcome(&p, &r, &q, &b, Outcome::Unconfirmed);
    }
    let (p, r, q, mut b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    b.ballots.truncate(2);
    assert_outcome(&p, &r, &q, &b, Outcome::VerifiedCurrent);
}

#[test]
fn authenticated_disagreement_is_counted_correctly() {
    let (p, r, q, mut b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    b.ballots[0].payload.advisory = false;
    b.ballots[0].signature = sign(&b.ballots[0].payload.canonical_bytes(), &[3; 32]);
    assert_outcome(&p, &r, &q, &b, Outcome::VerifiedCurrent);
    b.ballots[1].payload.advisory = false;
    b.ballots[1].signature = sign(&b.ballots[1].payload.canonical_bytes(), &[4; 32]);
    assert_outcome(&p, &r, &q, &b, Outcome::Unconfirmed);
}

#[test]
fn altered_trusted_configuration_is_not_accepted() {
    let (p, r, q, b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    let mut changed = p.clone();
    changed.threshold_ppm = 1000;
    assert_outcome(&changed, &r, &q, &b, Outcome::Rejected);
    let mut changed = r.clone();
    changed.members[0].revoked = true;
    assert_outcome(&p, &changed, &q, &b, Outcome::Rejected);
    let mut changed = r.clone();
    changed.version += 1;
    assert_outcome(&p, &changed, &q, &b, Outcome::Rejected);
    let mut changed = q.clone();
    changed.event_id = "another-event".into();
    assert_outcome(&p, &r, &changed, &b, Outcome::Rejected);
    let mut changed = q.clone();
    changed.nonce = "12".repeat(32);
    assert_outcome(&p, &r, &changed, &b, Outcome::Rejected);
    let mut changed = r.clone();
    changed.members[1].public_key = changed.members[0].public_key.clone();
    assert_outcome(&p, &changed, &q, &b, Outcome::Rejected);
    let mut changed = r.clone();
    changed.members[4].public_key = "00".repeat(32);
    assert_outcome(&p, &changed, &q, &b, Outcome::Rejected);
}

#[test]
fn future_capture_and_expiry_overflow_rejected() {
    let (mut p, r, mut q, b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    q.now_ms = 999;
    assert_outcome(&p, &r, &q, &b, Outcome::Rejected);
    // Build a consistently bound and signed policy whose timestamp + TTL overflows.
    p.ttl_ms = u64::MAX;
    let mut b = b.clone();
    for (i, m) in b.measurements.iter_mut().enumerate() {
        m.payload.policy_hash = p.id();
        m.signature = sign(&m.payload.canonical_bytes(), &[(i + 1) as u8; 32]);
    }
    b.decision.payload.policy_hash = p.id();
    b.decision.payload.measurement_ids = b.measurements.iter().map(|m| m.payload.id()).collect();
    b.decision.payload.measurement_ids.sort();
    resign_decision(&mut b);
    q.now_ms = 2000;
    assert_outcome(&p, &r, &q, &b, Outcome::Rejected);
}

#[test]
fn forged_expired_records_still_rejected_not_merely_expired() {
    let (p, r, mut q, mut b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    q.now_ms = 12_000;
    b.measurements[0].payload.co2_ppm += 1;
    assert_outcome(&p, &r, &q, &b, Outcome::Rejected);
}

#[test]
fn public_wire_format_rejects_unknown_and_duplicate_fields() {
    let (_, _, _, b) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    let mut value = serde_json::to_value(&b).unwrap();
    value["unchecked_override"] = true.into();
    assert!(serde_json::from_value::<Bundle>(value).is_err());
    let text =
        r#"{"event_id":"a","event_id":"b","nonce":"x","now_ms":0,"kind":"current","heads":{}}"#;
    assert!(serde_json::from_str::<Request>(text).is_err());
}

#[test]
fn deterministic_matched_sweep() {
    for seed in 0..64u32 {
        let (p, r, mut q, b) = demo::fixture(
            if seed % 2 == 0 {
                CrewMode::NominalThree
            } else {
                CrewMode::DegradedTwo
            },
            [seed * 31, seed * 17],
        );
        for now in [999, 1000, 2000, 10_999, 11_000, 12_000] {
            q.now_ms = now;
            assert_eq!(
                check(&b, &p, &r, &q).outcome,
                veri_adam_reference::check(&b, &p, &r, &q),
                "seed {seed}, time {now}"
            );
        }
    }
}
