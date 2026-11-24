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
