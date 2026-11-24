use veri_adam_cache::{Cache, Config};
use veri_adam_dependencies::{DependencyCache, Report};
use veri_adam_protocol::*;

type Snapshot = (Policy, Registry, Request, Bundle);
fn base() -> Snapshot {
    demo::fixture(CrewMode::NominalThree, [900, 850])
}
fn compare(b3: &mut DependencyCache, b2: &mut Cache, s: &Snapshot) -> Report {
    let out = b3.check(&s.3, &s.0, &s.1, &s.2);
    let ordinary = b2.check(&s.3, &s.0, &s.1, &s.2);
    let b1 = veri_adam_verification::check(&s.3, &s.0, &s.1, &s.2);
    let reference = veri_adam_reference::check(&s.3, &s.0, &s.1, &s.2);
    for report in [&ordinary.verification, &b1] {
        assert_eq!(out.verification.outcome, report.outcome);
        assert_eq!(out.verification.reason, report.reason);
        assert_eq!(out.verification.decision_id, report.decision_id);
        assert_eq!(
            out.verification.support_expires_at_ms,
            report.support_expires_at_ms
        );
    }
    assert_eq!(out.verification.outcome, reference);
    assert_eq!(
        out.verification.work.signature_checks,
        ordinary.verification.work.signature_checks
    );
    assert_eq!(
        out.verification.work.signature_cache_hits,
        ordinary.verification.work.signature_cache_hits
    );
    assert_eq!(
        out.cache_work.key_bytes_hashed,
        ordinary.cache_work.key_bytes_hashed
    );
    assert_eq!(
        out.cache_work.decision_hits,
        ordinary.cache_work.decision_hits
    );
    assert_eq!(out.signature_entries, ordinary.signature_entries);
    assert_eq!(out.decision_entries, ordinary.decision_entries);
    assert!(out.indexed_claims <= out.decision_entries);
    assert!(out.dependency_links <= out.indexed_claims * MAX_RECORDS);
    out
}

fn disjoint_pair() -> (Snapshot, Snapshot) {
    let mut a = base();
    let mut b = a.clone();
    b.1.version = 2;
    b.0.sources = vec!["sensor-c".into(), "sensor-d".into()];
    b.2.event_id = "disjoint-event".into();
    b.2.nonce = "cd".repeat(32);
    for i in 0..2 {
        b.1.members[i].signer_id = b.0.sources[i].clone();
        b.1.members[i].public_key = public_key(&[(i + 7) as u8; 32]);
        let m = &mut b.3.measurements[i];
        m.payload.signer_id = b.0.sources[i].clone();
        m.payload.event_id = b.2.event_id.clone();
        m.payload.nonce = b.2.nonce.clone();
        m.payload.policy_hash = b.0.id();
        m.signature = sign(&m.payload.canonical_bytes(), &[(i + 7) as u8; 32]);
    }
    let d = &mut b.3.decision.payload;
    d.event_id = b.2.event_id.clone();
    d.nonce = b.2.nonce.clone();
    d.registry_version = 2;
    d.policy_hash = b.0.id();
    d.measurement_ids = b.3.measurements.iter().map(|m| m.payload.id()).collect();
    d.measurement_ids.sort();
    b.3.decision.signature = sign(&d.canonical_bytes(), &[6; 32]);
    for (i, v) in b.3.ballots.iter_mut().enumerate() {
        v.payload.event_id = b.2.event_id.clone();
        v.payload.nonce = b.2.nonce.clone();
        v.payload.decision_hash = d.id();
        v.signature = sign(&v.payload.canonical_bytes(), &[(i + 3) as u8; 32]);
    }
    let heads =
        a.3.measurements
            .iter()
            .chain(&b.3.measurements)
            .map(|m| (m.payload.signer_id.clone(), m.payload.id()))
            .collect();
    a.2.heads = heads;
    b.2.heads = a.2.heads.clone();
    (a, b)
}

#[test]
fn stable_queries_reuse_index_but_snapshot_scans_are_visible() {
    let s = base();
    let mut b3 = DependencyCache::default();
    let mut b2 = Cache::default();
    assert_eq!(compare(&mut b3, &mut b2, &s).indexed_claims, 0);
    let admission = compare(&mut b3, &mut b2, &s);
    assert_eq!(admission.indexed_claims, 1);
    assert_eq!(admission.dependency_links, 2);
    assert_eq!(admission.cache_work.checkpoint_comparisons, 2);
    let hit = compare(&mut b3, &mut b2, &s);
    assert_eq!(hit.dependency_work.metadata_hits, 1);
    assert_eq!(hit.cache_work.checkpoint_comparisons, 0);
    assert_eq!(hit.dependency_work.snapshot_entries_examined, 4);
    assert_eq!(hit.dependency_work.dependency_links_visited, 0);
}

#[test]
fn changed_support_updates_only_affected_certificates() {
    let (a, mut b) = disjoint_pair();
    let mut variant = a.clone();
    variant.3.ballots.truncate(2);
    let mut b3 = DependencyCache::default();
    let mut b2 = Cache::default();
    for s in [&a, &b, &variant] {
        compare(&mut b3, &mut b2, s);
        compare(&mut b3, &mut b2, s);
    }
    b.2.heads.insert("sensor-a".into(), "ff".repeat(32));
    let unaffected = compare(&mut b3, &mut b2, &b);
    assert_eq!(unaffected.verification.outcome, Outcome::VerifiedCurrent);
    assert_eq!(unaffected.dependency_work.dependency_links_visited, 2);
    assert_eq!(unaffected.dependency_work.mismatch_changes, 2);
    assert_eq!(unaffected.indexed_claims, 3);
    variant.2.heads = b.2.heads.clone();
    assert_eq!(
        compare(&mut b3, &mut b2, &variant).verification.outcome,
        Outcome::Unconfirmed
    );
    variant.2.heads = a.2.heads.clone();
    assert_eq!(
        compare(&mut b3, &mut b2, &variant).verification.outcome,
        Outcome::VerifiedCurrent
    );
}

#[test]
fn unrelated_changes_visit_no_dependency_links() {
    let mut s = base();
    let mut b3 = DependencyCache::default();
    let mut b2 = Cache::default();
    compare(&mut b3, &mut b2, &s);
    compare(&mut b3, &mut b2, &s);
    s.2.heads.insert("unrelated".into(), "ef".repeat(32));
    let out = compare(&mut b3, &mut b2, &s);
    assert_eq!(out.dependency_work.snapshot_changes, 1);
    assert_eq!(out.dependency_work.dependency_links_visited, 0);
    assert_eq!(out.cache_work.checkpoint_comparisons, 0);
    assert_eq!(out.observed_heads, 3);
}

#[test]
fn missing_heads_can_recover_without_reauthenticating_records() {
    let original = base();
    let mut s = original.clone();
    s.2.heads.clear();
    let mut b3 = DependencyCache::default();
    let mut b2 = Cache::default();
    assert_eq!(
        compare(&mut b3, &mut b2, &s).verification.outcome,
        Outcome::Unconfirmed
    );
    assert_eq!(compare(&mut b3, &mut b2, &s).indexed_claims, 1);
    s.2.heads = original.2.heads;
    let out = compare(&mut b3, &mut b2, &s);
    assert_eq!(out.verification.outcome, Outcome::VerifiedCurrent);
    assert_eq!(out.dependency_work.dependency_links_visited, 2);
    assert_eq!(out.dependency_work.mismatch_changes, 2);
    assert_eq!(out.verification.work.signature_checks, 0);
}

#[test]
fn expiry_future_time_and_historical_checks_preserve_reference_semantics() {
    let mut s = base();
    let mut b3 = DependencyCache::default();
    let mut b2 = Cache::default();
    compare(&mut b3, &mut b2, &s);
    compare(&mut b3, &mut b2, &s);
    for (now, kind) in [
        (11_000, CheckKind::Current),
        (12_000, CheckKind::Historical),
        (999, CheckKind::Current),
        (999, CheckKind::Historical),
        (2_000, CheckKind::Current),
    ] {
        s.2.now_ms = now;
        s.2.kind = kind;
        compare(&mut b3, &mut b2, &s);
    }
    s.2.kind = CheckKind::Historical;
    s.2.heads.clear();
    let historical = compare(&mut b3, &mut b2, &s);
    assert_eq!(historical.dependency_work.snapshot_entries_examined, 0);
    s.2.kind = CheckKind::Current;
    let current = compare(&mut b3, &mut b2, &s);
    assert_eq!(current.verification.outcome, Outcome::Unconfirmed);
    assert_eq!(current.dependency_work.dependency_links_visited, 2);
}

#[test]
fn eviction_removes_forward_and_reverse_metadata() {
    let (a, b) = disjoint_pair();
    let config = Config {
        signature_capacity: 32,
        decision_capacity: 1,
    };
    let mut b3 = DependencyCache::new(config);
    let mut b2 = Cache::new(config);
    compare(&mut b3, &mut b2, &a);
    compare(&mut b3, &mut b2, &a);
    let evicted = compare(&mut b3, &mut b2, &b);
    assert_eq!(evicted.dependency_work.metadata_entries_removed, 1);
    assert_eq!(evicted.dependency_work.dependency_links_removed, 2);
    assert_eq!(evicted.indexed_claims, 0);
    assert_eq!(evicted.tracked_sources, 0);
    let admitted = compare(&mut b3, &mut b2, &b);
    assert_eq!(admitted.indexed_claims, 1);
    assert_eq!(admitted.tracked_sources, 2);
    let old = compare(&mut b3, &mut b2, &a);
    assert_eq!(old.indexed_claims, 0);
    assert!(old.dependency_work.cleanup_entries_examined > 0);
}

#[test]
fn restart_clears_index_and_exposes_cleanup_counts() {
    let s = base();
    let mut b3 = DependencyCache::default();
    let mut b2 = Cache::default();
    compare(&mut b3, &mut b2, &s);
    compare(&mut b3, &mut b2, &s);
    b3.clear();
    b2.clear();
    let out = compare(&mut b3, &mut b2, &s);
    assert_eq!(out.indexed_claims, 0);
    assert_eq!(out.verification.work.signature_checks, 6);
    assert_eq!(out.dependency_work.reset_claims_cleared, 1);
    assert_eq!(out.dependency_work.reset_links_cleared, 2);
    assert_eq!(out.dependency_work.reset_heads_cleared, 2);
}

#[test]
fn new_content_or_configuration_cannot_reuse_indexed_authorization() {
    let original = base();
    let mut b3 = DependencyCache::default();
    let mut b2 = Cache::default();
    compare(&mut b3, &mut b2, &original);
    compare(&mut b3, &mut b2, &original);
    for i in 0..8 {
        let mut s = original.clone();
        match i {
            0 => s.3.measurements[0].payload.co2_ppm += 1,
            1 => s.3.measurements[0].signature = "00".repeat(64),
            2 => s.1.members[0].revoked = true,
            3 => s.1.members[0].public_key = public_key(&[9; 32]),
            4 => s.0.threshold_ppm += 1,
            5 => s.3.ballots[1] = s.3.ballots[0].clone(),
            6 => s.2.event_id = "other-event".into(),
            _ => s.2.nonce = "ef".repeat(32),
        }
        let out = compare(&mut b3, &mut b2, &s);
        assert_eq!(out.verification.outcome, Outcome::Rejected);
        assert_eq!(out.dependency_work.metadata_hits, 0);
        assert_eq!(out.dependency_work.metadata_entries_inserted, 0);
    }
}

#[test]
fn missing_quorum_never_becomes_current_through_metadata() {
    let mut s = base();
    s.3.ballots.truncate(1);
    let mut b3 = DependencyCache::default();
    let mut b2 = Cache::default();
    for _ in 0..3 {
        let out = compare(&mut b3, &mut b2, &s);
        assert_eq!(out.verification.outcome, Outcome::Unconfirmed);
        assert_eq!(out.indexed_claims, 0);
    }
}

#[test]
fn transport_reordering_preserves_the_existing_dependency_binding() {
    let mut s = base();
    let mut b3 = DependencyCache::default();
    let mut b2 = Cache::default();
    compare(&mut b3, &mut b2, &s);
    compare(&mut b3, &mut b2, &s);
    s.3.measurements.reverse();
    s.3.ballots.reverse();
    s.1.members.reverse();
    let out = compare(&mut b3, &mut b2, &s);
    assert_eq!(out.dependency_work.metadata_hits, 1);
    assert_eq!(out.dependency_work.metadata_entries_inserted, 0);
    assert_eq!(out.cache_work.checkpoint_comparisons, 0);
}

#[test]
fn stateful_sweep_preserves_b1_b2_and_reference_outcomes() {
    for capacity in [0, 1, 8] {
        let config = Config {
            signature_capacity: capacity,
            decision_capacity: capacity,
        };
        let mut b3 = DependencyCache::new(config);
        let mut b2 = Cache::new(config);
        for seed in 0..24u32 {
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
                compare(&mut b3, &mut b2, &s);
                compare(&mut b3, &mut b2, &s);
            }
            s.2.now_ms = 2000;
            s.2.heads.clear();
            compare(&mut b3, &mut b2, &s);
            s.2.kind = CheckKind::Historical;
            compare(&mut b3, &mut b2, &s);
            s.3.measurements[0].signature = "00".repeat(64);
            compare(&mut b3, &mut b2, &s);
        }
    }
}
