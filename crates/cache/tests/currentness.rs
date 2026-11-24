use veri_adam_cache::{Cache, CachedReport, Config};
use veri_adam_protocol::*;

type Snapshot = (Policy, Registry, Request, Bundle);
fn check(cache: &mut Cache, s: &Snapshot) -> CachedReport {
    let report = cache.check(&s.3, &s.0, &s.1, &s.2);
    let b1 = veri_adam_verification::check(&s.3, &s.0, &s.1, &s.2);
    let reference = veri_adam_reference::check(&s.3, &s.0, &s.1, &s.2);
    assert_eq!(report.verification.outcome, reference);
    assert_eq!(report.verification.outcome, b1.outcome);
    assert_eq!(report.verification.reason, b1.reason);
    assert_eq!(report.verification.decision_id, b1.decision_id);
    assert_eq!(
        report.verification.support_expires_at_ms,
        b1.support_expires_at_ms
    );
    report
}
fn base() -> Snapshot {
    demo::fixture(CrewMode::NominalThree, [900, 850])
}
fn renew(s: &mut Snapshot) {
    let m = &mut s.3.measurements[0];
    m.payload.sequence += 1;
    m.payload.captured_at_ms = 2_000;
    m.signature = sign(&m.payload.canonical_bytes(), &[1; 32]);
    s.2.heads
        .insert(m.payload.signer_id.clone(), m.payload.id());
    s.3.decision.payload.measurement_ids =
        s.3.measurements.iter().map(|m| m.payload.id()).collect();
    s.3.decision.payload.measurement_ids.sort();
    s.3.decision.signature = sign(&s.3.decision.payload.canonical_bytes(), &[6; 32]);
    for (i, v) in s.3.ballots.iter_mut().enumerate() {
        v.payload.decision_hash = s.3.decision.payload.id();
        v.signature = sign(&v.payload.canonical_bytes(), &[(i + 3) as u8; 32]);
    }
}

#[test]
fn stable_and_unrelated_updates_reuse_complete_checks() {
    let mut cache = Cache::default();
    let mut s = base();
    let cold = check(&mut cache, &s);
    assert_eq!(cold.verification.work.signature_checks, 6);
    assert_eq!(cold.signature_entries, 6);
    assert_eq!(cold.decision_entries, 1);
    for i in 0..5 {
        s.2.now_ms = 2_000 + i;
        s.2.heads.insert(format!("unrelated-{i}"), "00".repeat(32));
        let warm = check(&mut cache, &s);
        assert_eq!(warm.cache_work.decision_hits, 1);
        assert_eq!(warm.verification.work.signature_checks, 0);
        assert_eq!(warm.cache_work.checkpoint_comparisons, 2);
        assert!(warm.cache_work.key_bytes_hashed > 0);
    }
}

#[test]
fn cache_hits_recheck_expiry_history_and_future_time() {
    let mut cache = Cache::default();
    let mut s = base();
    check(&mut cache, &s);
    for (now, kind, expected) in [
        (10_999, CheckKind::Current, Outcome::VerifiedCurrent),
        (11_000, CheckKind::Current, Outcome::Expired),
        (12_000, CheckKind::Historical, Outcome::VerifiedHistorical),
        (999, CheckKind::Historical, Outcome::Rejected),
        (999, CheckKind::Current, Outcome::Rejected),
        (2_000, CheckKind::Current, Outcome::VerifiedCurrent),
    ] {
        s.2.now_ms = now;
        s.2.kind = kind;
        let out = check(&mut cache, &s);
        assert_eq!(out.cache_work.decision_hits, 1);
        assert_eq!(out.verification.outcome, expected);
    }
}

#[test]
fn historical_warmup_cannot_grant_current_support() {
    let mut s = base();
    let mut cache = Cache::default();
    s.2.now_ms = 11_000;
    s.2.kind = CheckKind::Historical;
    assert_eq!(
        check(&mut cache, &s).verification.outcome,
        Outcome::VerifiedHistorical
    );
    s.2.kind = CheckKind::Current;
    assert_eq!(check(&mut cache, &s).verification.outcome, Outcome::Expired);
    s.2.now_ms = 2_000;
    s.2.heads.clear();
    assert_eq!(
        check(&mut cache, &s).verification.outcome,
        Outcome::Unconfirmed
    );
}

#[test]
fn missing_or_replaced_heads_do_not_inherit_cached_acceptance() {
    let mut s = base();
    let mut cache = Cache::default();
    check(&mut cache, &s);
    s.2.heads.remove("sensor-a");
    assert_eq!(
        check(&mut cache, &s).verification.outcome,
        Outcome::Unconfirmed
    );
    s.2.heads.insert("sensor-a".into(), "ff".repeat(32));
    assert_eq!(
        check(&mut cache, &s).verification.outcome,
        Outcome::Unconfirmed
    );
    s.2.heads = base().2.heads;
    assert_eq!(
        check(&mut cache, &s).verification.outcome,
        Outcome::VerifiedCurrent
    );
}

#[test]
fn replacement_and_quorum_changes_reuse_unchanged_signatures() {
    let mut s = base();
    let mut cache = Cache::default();
    check(&mut cache, &s);
    renew(&mut s);
    let renewed = check(&mut cache, &s);
    assert_eq!(renewed.verification.outcome, Outcome::VerifiedCurrent);
    assert_eq!(renewed.cache_work.decision_hits, 0);
    assert_eq!(renewed.verification.work.signature_checks, 5);
    assert_eq!(renewed.verification.work.signature_cache_hits, 1);
    // Dropping the optional third ballot changes the bundle, but not its signatures.
    s.3.ballots.truncate(2);
    let two = check(&mut cache, &s);
    assert_eq!(two.verification.outcome, Outcome::VerifiedCurrent);
    assert_eq!(two.verification.work.signature_checks, 0);
    assert_eq!(two.verification.work.signature_cache_hits, 5);
    s.3.ballots.truncate(1);
    let one = check(&mut cache, &s);
    assert_eq!(one.verification.outcome, Outcome::Unconfirmed);
    assert_eq!(
        check(&mut cache, &s).verification.outcome,
        Outcome::Unconfirmed
    );
}

#[test]
fn full_configuration_content_is_bound_even_without_a_version_bump() {
    let original = base();
    let mut cache = Cache::default();
    check(&mut cache, &original);
    for change in 0..6 {
        let mut s = original.clone();
        match change {
            0 => s.0.threshold_ppm += 1,
            1 => s.0.ttl_ms += 1,
            2 => s.1.members[0].revoked = true,
            3 => s.1.members[0].public_key = public_key(&[9; 32]),
            4 => s.2.event_id = "replayed-event".into(),
            _ => s.2.nonce = "12".repeat(32),
        }
        let out = check(&mut cache, &s);
        assert_eq!(out.cache_work.decision_hits, 0);
        assert_eq!(out.verification.outcome, Outcome::Rejected);
    }
    assert_eq!(check(&mut cache, &original).cache_work.decision_hits, 1);
}

#[test]
fn warm_cache_never_authenticates_altered_content_or_signatures() {
    let original = base();
    let mut cache = Cache::default();
    check(&mut cache, &original);
    for change in 0..7 {
        let mut s = original.clone();
        match change {
            0 => s.3.measurements[0].payload.co2_ppm += 1,
            1 => s.3.measurements[0].signature = "00".repeat(64),
            2 => s.3.decision.signature = "00".repeat(64),
            3 => s.3.ballots[0].signature = "00".repeat(64),
            4 => s.3.ballots[0].payload.advisory = false,
            5 => s.3.ballots[1] = s.3.ballots[0].clone(),
            _ => s.3.decision.payload.measurement_ids[0] = "00".repeat(32),
        }
        for _ in 0..2 {
            let out = check(&mut cache, &s);
            assert_eq!(out.verification.outcome, Outcome::Rejected);
            assert_eq!(out.cache_work.decision_hits, 0);
            assert_eq!(out.cache_work.decision_insertions, 0);
        }
    }
}

#[test]
fn cache_is_bounded_and_eviction_changes_work_not_acceptance() {
    let mut cache = Cache::new(Config {
        signature_capacity: 1,
        decision_capacity: 1,
    });
    let original = base();
    let mut changed = original.clone();
    renew(&mut changed);
    check(&mut cache, &original);
    let out = check(&mut cache, &changed);
    assert_eq!(out.signature_entries, 1);
    assert_eq!(out.decision_entries, 1);
    assert_eq!(out.cache_work.decision_evictions, 1);
    assert!(out.cache_work.signature_evictions > 0);
    let old = check(&mut cache, &original);
    assert_eq!(old.cache_work.decision_hits, 0);
    assert_eq!(old.verification.outcome, Outcome::VerifiedCurrent);
}

#[test]
fn disabled_decision_cache_still_reuses_individual_signatures() {
    let mut cache = Cache::new(Config {
        signature_capacity: 32,
        decision_capacity: 0,
    });
    let s = base();
    check(&mut cache, &s);
    let warm = check(&mut cache, &s);
    assert_eq!(warm.verification.work.signature_checks, 0);
    assert_eq!(warm.verification.work.signature_cache_hits, 6);
    assert_eq!(warm.verification.work.measurements_evaluated, 2);
    assert_eq!(warm.decision_entries, 0);
}

#[test]
fn disabled_caches_and_restart_are_cold() {
    let mut cache = Cache::new(Config {
        signature_capacity: 0,
        decision_capacity: 0,
    });
    let s = base();
    for _ in 0..2 {
        let out = check(&mut cache, &s);
        assert_eq!(out.verification.work.signature_checks, 6);
        assert_eq!(out.signature_entries, 0);
        assert_eq!(out.decision_entries, 0);
    }
    let mut cache = Cache::default();
    check(&mut cache, &s);
    cache.clear();
    let out = check(&mut cache, &s);
    assert_eq!(out.verification.work.signature_checks, 6);
    assert_eq!(out.cache_work.decision_hits, 0);
}

#[test]
fn record_order_does_not_change_acceptance_or_require_new_signatures() {
    let mut s = base();
    let mut cache = Cache::default();
    check(&mut cache, &s);
    s.3.measurements.reverse();
    s.3.ballots.reverse();
    s.1.members.reverse();
    let out = check(&mut cache, &s);
    assert_eq!(out.verification.outcome, Outcome::VerifiedCurrent);
    assert_eq!(out.verification.work.signature_checks, 0);
    assert_eq!(out.cache_work.decision_hits, 1);
}

#[test]
fn matched_stateful_sweep_with_small_caches() {
    for capacity in [0, 1, 8] {
        let mut cache = Cache::new(Config {
            signature_capacity: capacity,
            decision_capacity: capacity,
        });
        for seed in 0..32u32 {
            let mut s = demo::fixture(
                if seed % 2 == 0 {
                    CrewMode::NominalThree
                } else {
                    CrewMode::DegradedTwo
                },
                [seed * 31, seed * 17],
            );
            for now in [999, 1000, 2000, 10_999, 11_000, 12_000] {
                s.2.now_ms = now;
                check(&mut cache, &s);
                check(&mut cache, &s);
            }
            s.2.heads.clear();
            check(&mut cache, &s);
            s.2.kind = CheckKind::Historical;
            check(&mut cache, &s);
            s.3.measurements[0].signature = "00".repeat(64);
            check(&mut cache, &s);
        }
    }
}
