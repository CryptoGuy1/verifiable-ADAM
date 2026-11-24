use veri_adam_protocol::Outcome;
use veri_adam_simulation::{Config, Scenario, run, viewer};

fn scenario(s: Scenario) -> veri_adam_simulation::Simulation {
    let simulation = run(Config {
        scenario: s,
        ..Default::default()
    })
    .unwrap();
    assert!(simulation.verification.all_matched);
    let n = &simulation.summary;
    assert_eq!(
        n.created_rounds + n.missed_source_rounds,
        n.attempted_rounds
    );
    assert_eq!(
        n.delivered_legitimate_rounds
            + n.partition_dropped_rounds
            + n.pending_legitimate_rounds_at_end,
        n.created_rounds
    );
    assert_eq!(
        n.timely_verified_rounds + n.late_or_withheld_rounds,
        n.attempted_rounds
    );
    simulation
}

#[test]
fn stable_run_has_no_faults_and_all_rounds_meet_deadline() {
    let s = scenario(Scenario::Stable);
    assert_eq!(s.summary.attempted_rounds, 12);
    assert_eq!(s.summary.timely_verified_rounds, 12);
    assert_eq!(s.summary.late_or_withheld_rounds, 0);
    assert_eq!(s.summary.periodic_checks_without_a_claim, 1);
    assert_eq!(s.verification.summary.checks, 72);
    assert!(
        s.workload
            .steps
            .iter()
            .all(|step| step.expected == Outcome::VerifiedCurrent)
    );
}

#[test]
fn mixed_run_preserves_failure_denominators_and_independent_expectations() {
    let s = scenario(Scenario::Mixed);
    assert_eq!(s.summary.missed_source_rounds, 2);
    assert_eq!(s.summary.partition_dropped_rounds, 2);
    assert_eq!(s.summary.delivered_legitimate_rounds, 8);
    assert_eq!(s.summary.timely_verified_rounds, 6);
    assert_eq!(s.summary.injected_tamper_attempts, 1);
    assert_eq!(s.summary.injected_replay_attempts, 1);
    assert_eq!(s.verification.summary.checks, 70);
    for outcome in [
        Outcome::VerifiedCurrent,
        Outcome::Expired,
        Outcome::Unconfirmed,
        Outcome::Rejected,
    ] {
        assert!(s.workload.steps.iter().any(|step| step.expected == outcome));
    }
}

#[test]
fn seed_and_configuration_reproduce_identical_artifacts() {
    let a = run(Config::default()).unwrap();
    let b = run(Config::default()).unwrap();
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&b).unwrap()
    );
    let c = run(Config {
        seed: 43,
        ..Default::default()
    })
    .unwrap();
    assert_ne!(
        serde_json::to_vec(&a.samples).unwrap(),
        serde_json::to_vec(&c.samples).unwrap()
    );
}

#[test]
fn dropout_and_partition_expire_old_claims_without_fabricating_samples() {
    let dropout = scenario(Scenario::Dropout);
    assert_eq!(
        dropout
            .samples
            .iter()
            .filter(|s| s.measured_a_ppm.is_none())
            .count(),
        2
    );
    assert_eq!(dropout.summary.created_rounds, 10);
    assert!(
        dropout
            .workload
            .steps
            .iter()
            .any(|s| s.expected == Outcome::Expired)
    );
    for step in &dropout.workload.steps {
        for m in &step.bundle.measurements {
            assert_ne!(m.payload.captured_at_ms, 15_000);
            assert_ne!(m.payload.captured_at_ms, 20_000);
        }
    }
    let partition = scenario(Scenario::Partition);
    assert_eq!(partition.summary.partition_dropped_rounds, 2);
    assert_eq!(partition.summary.created_rounds, 12);
    assert!(
        partition
            .workload
            .steps
            .iter()
            .any(|s| s.expected == Outcome::Expired)
    );
}

#[test]
fn delayed_packets_do_not_roll_back_trusted_heads_or_active_rounds() {
    let s = scenario(Scenario::Delay);
    assert_eq!(s.summary.delivered_legitimate_rounds, 12);
    assert_eq!(s.summary.timely_verified_rounds, 10);
    assert!(
        s.workload
            .steps
            .iter()
            .any(|s| s.expected == Outcome::Expired)
    );
    assert!(
        s.workload
            .steps
            .iter()
            .any(|s| s.expected == Outcome::Unconfirmed)
    );
    let active: Vec<_> = s
        .workload
        .steps
        .iter()
        .filter(|s| s.label.starts_with("periodic-"))
        .map(|s| s.bundle.measurements[0].payload.sequence)
        .collect();
    assert!(active.windows(2).all(|p| p[0] <= p[1]));
}

#[test]
fn tamper_and_cross_event_replay_are_rejected() {
    for fault in [Scenario::Tamper, Scenario::Replay] {
        let s = scenario(fault);
        let rejected: Vec<_> = s
            .verification
            .steps
            .iter()
            .filter(|s| s.b1.outcome == Outcome::Rejected)
            .collect();
        assert_eq!(rejected.len(), 1);
        assert_eq!(s.summary.timely_verified_rounds, 12);
        if fault == Scenario::Replay {
            assert_eq!(rejected[0].b1.reason, "unexpected_event_or_nonce");
        }
    }
}

#[test]
fn signed_false_readings_can_pass_protocol_checks_but_disagree_with_model_truth() {
    let s = scenario(Scenario::FalseReading);
    assert!(s.summary.authentic_false_reading_rounds > 0);
    assert!(s.summary.model_classification_disagreements > 0);
    assert!(
        s.verification
            .steps
            .iter()
            .all(|s| s.b1.outcome == Outcome::VerifiedCurrent)
    );
}

#[test]
fn invalid_and_excessive_configurations_fail_explicitly() {
    assert!(
        run(Config {
            sample_interval_ms: 0,
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        run(Config {
            check_interval_ms: 0,
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        run(Config {
            duration_ms: u64::MAX,
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        run(Config {
            duration_ms: 300_000,
            sample_interval_ms: 1_000,
            check_interval_ms: 250,
            ..Default::default()
        })
        .unwrap_err()
        .contains("1 MiB")
    );
    assert!(Scenario::parse("unknown").is_err());
}

#[test]
fn saved_workload_is_replayable_and_keeps_declared_expected_outcomes() {
    let s = scenario(Scenario::Mixed);
    let text = serde_json::to_string_pretty(&s.workload).unwrap();
    assert!(text.len() as u64 <= veri_adam_protocol::MAX_FILE_BYTES);
    let workload = serde_json::from_str(&text).unwrap();
    let replay = veri_adam_replay::run(&workload).unwrap();
    assert_eq!(
        serde_json::to_value(&replay).unwrap(),
        serde_json::to_value(&s.verification).unwrap()
    );
}

#[test]
fn standalone_viewer_escapes_embedded_data_and_requires_no_external_scripts() {
    let mut s = scenario(Scenario::Stable);
    s.trace[0].detail = "</script><script>INJECTED</script>".into();
    let html = viewer::html(&s).unwrap();
    assert!(!html.contains("<script>INJECTED"));
    assert!(html.contains("\\u003c/script\\u003e"));
    assert!(!html.contains("__SIMULATION_DATA__"));
    assert!(!html.contains("<script src="));
    assert!(html.contains("id=\"seek\""));
}
