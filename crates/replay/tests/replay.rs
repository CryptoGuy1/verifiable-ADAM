use veri_adam_cache::Config;
use veri_adam_replay::{Workload, demo, run};

#[test]
fn frozen_workload_matches_generator_and_reference() {
    let frozen: Workload =
        serde_json::from_str(include_str!("../../../workloads/b2-currentness-v1.json")).unwrap();
    assert_eq!(
        serde_json::to_value(&frozen).unwrap(),
        serde_json::to_value(demo::workload()).unwrap()
    );
    let report = run(&frozen).unwrap();
    assert!(report.all_matched);
    assert_eq!(report.summary.checks, 31);
    assert_eq!(report.summary.mismatches, 0);
    assert!(report.summary.b2_work.signature_checks < report.summary.b1_work.signature_checks);
    assert!(report.summary.b2_cache_work.decision_hits > 0);
    assert!(report.summary.b2_cache_work.key_bytes_hashed > 0);
    // Adding B3 must not handicap the already frozen conventional baseline.
    assert_eq!(report.summary.b1_work.signature_checks, 132);
    assert_eq!(report.summary.b2_work.signature_checks, 31);
    assert_eq!(report.summary.b3_work.signature_checks, 31);
}

#[test]
fn capacities_do_not_change_outcomes() {
    let mut w = demo::workload();
    for (signatures, decisions) in [(0, 0), (1, 1), (64, 0), (0, 64), (64, 64)] {
        w.cache_config = Config {
            signature_capacity: signatures,
            decision_capacity: decisions,
        };
        let result = run(&w).unwrap();
        assert!(result.all_matched);
        assert!(result.summary.peak_signature_entries <= signatures);
        assert!(result.summary.peak_decision_entries <= decisions);
        assert!(result.summary.b3_peak_indexed_claims <= decisions);
    }
}

#[test]
fn multi_claim_workload_is_frozen_and_updates_only_affected_links() {
    let w: Workload =
        serde_json::from_str(include_str!("../../../workloads/b3-multi-claim-v1.json")).unwrap();
    assert_eq!(
        serde_json::to_value(&w).unwrap(),
        serde_json::to_value(veri_adam_replay::dependencies_demo::workload()).unwrap()
    );
    let result = run(&w).unwrap();
    assert!(result.all_matched);
    assert_eq!(result.summary.checks, 27);
    assert_eq!(result.report_schema_version, 2);
    assert_eq!(
        result.summary.b2_work.signature_checks,
        result.summary.b3_work.signature_checks
    );
    assert!(
        result.summary.b3_cache_work.checkpoint_comparisons
            < result.summary.b2_cache_work.checkpoint_comparisons
    );
    assert!(result.summary.b3_dependency_work.snapshot_entries_examined > 0);
    let affected = result
        .steps
        .iter()
        .find(|s| s.label == "support-a-change-check-unaffected-b")
        .unwrap();
    assert_eq!(affected.b3.dependency_work.dependency_links_visited, 2);
    assert_eq!(
        affected.b3.verification.outcome,
        veri_adam_protocol::Outcome::VerifiedCurrent
    );
    let unrelated = result
        .steps
        .iter()
        .find(|s| s.label == "unrelated-update-visits-no-claim")
        .unwrap();
    assert_eq!(unrelated.b3.dependency_work.dependency_links_visited, 0);
}

#[test]
fn multi_claim_capacity_changes_preserve_all_outcomes() {
    let mut w = veri_adam_replay::dependencies_demo::workload();
    for (signatures, decisions) in [(0, 0), (1, 1), (64, 0), (0, 64), (64, 2), (64, 64)] {
        w.cache_config = Config {
            signature_capacity: signatures,
            decision_capacity: decisions,
        };
        let result = run(&w).unwrap();
        assert!(result.all_matched);
        assert!(result.summary.b3_peak_indexed_claims <= decisions);
        assert!(
            result.summary.b3_peak_dependency_links <= decisions * veri_adam_protocol::MAX_RECORDS
        );
        assert_eq!(
            result.summary.b2_work.signature_checks,
            result.summary.b3_work.signature_checks
        );
    }
}

#[test]
fn wrong_expectations_are_reported_not_overwritten() {
    let mut w = demo::workload();
    w.steps[0].expected = veri_adam_protocol::Outcome::Rejected;
    let result = run(&w).unwrap();
    assert!(!result.all_matched);
    assert_eq!(result.summary.mismatches, 1);
    assert!(!result.steps[0].matched);
}

#[test]
fn invalid_workloads_are_errors() {
    let mut w = demo::workload();
    w.version = 2;
    assert!(run(&w).is_err());
    w.version = 1;
    w.data_kind = "unlabeled".into();
    assert!(run(&w).is_err());
    w.data_kind = "synthetic".into();
    w.steps.clear();
    assert!(run(&w).is_err());
}
