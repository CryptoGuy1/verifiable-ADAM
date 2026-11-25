use std::{
    fs,
    path::Path,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_veri-adam"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn connected_agents_private_claim_and_durable_authorization_are_real_and_ui_is_absent() {
    let root = std::env::temp_dir().join(format!(
        "adam-connected-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = Scratch(root.clone());
    let path = root.to_str().unwrap();
    let output = cli(&["adam-agent-run", path, "rules"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rounds: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("agent-run.json")).unwrap()).unwrap();
    assert_eq!(rounds.as_array().unwrap().len(), 6);
    for round in &rounds.as_array().unwrap()[..4] {
        assert_eq!(
            round["verification"]["outcome"],
            "verified_current_awaiting_recording"
        );
        assert_eq!(
            round["private_verification"]["private_fusion_verified"],
            true
        );
        assert_eq!(
            round["private_verification"]["full_decision_processing_verified"],
            false
        );
        assert_eq!(
            round["ordinary_verification"]["outcome"],
            round["verification"]["outcome"]
        );
        assert_eq!(
            round["dependency_verification"]["outcome"],
            round["verification"]["outcome"]
        );
    }
    assert_eq!(rounds[4]["verification"]["outcome"], "unconfirmed");
    assert!(rounds[5]["record"].is_null());
    assert!(
        root.join("weighted-anomaly/advisory-authorization.json")
            .exists()
    );
    assert!(
        !root
            .join("missing-ballot/advisory-authorization.json")
            .exists()
    );
    assert!(!root.join("simulation.html").exists());
    let public =
        fs::read_to_string(root.join("private-export/weighted-normal/record.json")).unwrap();
    assert!(
        !public.contains("co2_ppm")
            && !public.contains("raw_response")
            && !public.contains("blinding")
    );
    assert!(
        !root
            .join("private-export/weighted-normal/authorized-disclosure.json")
            .exists()
    );
    let out = cli(&[
        "adam-private-check",
        root.join("weighted-normal/record.json").to_str().unwrap(),
        root.join("weighted-normal/private-proof.json")
            .to_str()
            .unwrap(),
        root.join("trust.json").to_str().unwrap(),
        root.join("weighted-normal/request.json").to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = cli(&[
        "adam-audit-check",
        root.join("audit").to_str().unwrap(),
        root.join("trust.json").to_str().unwrap(),
        root.join("latest-checkpoint.json").to_str().unwrap(),
    ]);
    assert!(out.status.success());
}

#[test]
fn authenticated_weighted_run_is_independently_checkable_and_never_claims_release() {
    let root = std::env::temp_dir().join(format!(
        "adam-auth-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = Scratch(root.clone());
    let path = root.to_str().unwrap();
    assert_eq!(
        cli(&["adam-auth-run", path, "invalid"]).status.code(),
        Some(2)
    );
    assert!(!root.exists());
    let out = cli(&["adam-auth-run", path, "rules"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for i in 0..4 {
        assert_eq!(
            manifest["cases"][i]["independent_verification"]["outcome"],
            "verified_current_awaiting_recording"
        );
    }
    assert_eq!(
        manifest["cases"][4]["independent_verification"]["outcome"],
        "unconfirmed"
    );
    assert_eq!(manifest["action_released"], false);
    let record = root.join("weighted-normal/record.json");
    let disclosure = root.join("weighted-normal/authorized-disclosure.json");
    let request = root.join("weighted-normal/request.json");
    let trust = root.join("trust.json");
    let out = cli(&[
        "adam-check",
        record.to_str().unwrap(),
        disclosure.to_str().unwrap(),
        trust.to_str().unwrap(),
        request.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = cli(&[
        "adam-check",
        record.to_str().unwrap(),
        "-",
        trust.to_str().unwrap(),
        request.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["private_proof_verified"], false);
    let original = fs::read(&record).unwrap();
    assert_eq!(
        cli(&["adam-auth-run", path, "ollama"]).status.code(),
        Some(2)
    );
    assert_eq!(original, fs::read(record).unwrap());
}

#[test]
fn weighted_decision_stage_uses_calibration_and_does_not_claim_authenticated_release() {
    let root = std::env::temp_dir().join(format!(
        "adam-stage-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = Scratch(root.clone());
    let path = root.to_str().unwrap();
    assert_eq!(cli(&["adam-run", path, "unknown"]).status.code(), Some(2));
    assert!(!root.exists());
    let output = cli(&["adam-run", path, "rules"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(root.join("decision-stage-run.json")).unwrap();
    let records: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(records.as_array().unwrap().len(), 5);
    assert_eq!(records[0]["fusion"]["fused_ppm"], 798.0);
    assert_eq!(records[0]["final_classification"], "NORMAL");
    assert_eq!(
        records[1]["final_decision"]["recommended_action"],
        "VentilationAdvisory"
    );
    assert_eq!(records[2]["sensor_assessment"]["classification"], "ANOMALY");
    assert_eq!(
        records[3]["eligible_roles"],
        serde_json::json!(["sensor", "decision"])
    );
    assert_eq!(records[4]["status"], "unresolved");
    assert!(
        records
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["action_released"] == false && r["authenticated_record"] == false)
    );
    let evaluated = cli(&[
        "adam-evaluate",
        root.join("event-1.json").to_str().unwrap(),
        root.join("policy.json").to_str().unwrap(),
        root.join("calibration-profile.json").to_str().unwrap(),
    ]);
    assert!(evaluated.status.success());
    let r: serde_json::Value = serde_json::from_slice(&evaluated.stdout).unwrap();
    assert_eq!(r["fusion"]["fused_ppm"], 798.0);
    let c = root.join("recalibrated");
    assert!(
        cli(&[
            "adam-calibrate",
            root.join("calibration-input.json").to_str().unwrap(),
            c.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert_eq!(
        fs::read(root.join("calibration-profile.json")).unwrap(),
        fs::read(c.join("calibration-profile.json")).unwrap()
    );
    assert_eq!(cli(&["adam-run", path, "ollama"]).status.code(), Some(2));
    assert_eq!(
        bytes,
        fs::read(root.join("decision-stage-run.json")).unwrap()
    );
}

#[test]
fn agent_simulation_exports_real_ballots_and_preserves_existing_runs() {
    let root = std::env::temp_dir().join(format!(
        "veri-adam-agents-cli-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = Scratch(root.clone());
    let path = root.to_str().unwrap();
    assert_eq!(
        cli(&["agent-simulate", path, "invalid"]).status.code(),
        Some(2)
    );
    assert!(!root.exists());
    let out = cli(&["agent-simulate", path, "rules"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = fs::read(root.join("agent-run.json")).unwrap();
    let run: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(run["rounds"].as_array().unwrap().len(), 7);
    assert_eq!(
        run["rounds"][2]["bundle"]["ballots"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        run["rounds"][2]["bundle"]["ballots"][0]["payload"]["advisory"],
        false
    );
    assert_eq!(run["rounds"][2]["state"], "raised");
    assert!(
        fs::read_to_string(root.join("simulation.html"))
            .unwrap()
            .contains("What did the agents decide?")
    );
    // An existing directory must fail before model preparation or any overwrite.
    assert_eq!(
        cli(&["agent-simulate", path, "ollama"]).status.code(),
        Some(2)
    );
    assert_eq!(bytes, fs::read(root.join("agent-run.json")).unwrap());
}
struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn demo_verify_tampering_expiry_history_and_no_overwrite() {
    let root = std::env::temp_dir().join(format!(
        "veri-adam-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = Scratch(root.clone());
    let path = root.to_str().unwrap();
    assert!(cli(&["demo", path]).status.success());
    assert_eq!(cli(&["demo", path]).status.code(), Some(2));
    let verify = |bundle: &str, request: &str| {
        cli(&[
            "verify",
            root.join(bundle).to_str().unwrap(),
            root.join("policy.json").to_str().unwrap(),
            root.join("registry.json").to_str().unwrap(),
            root.join(request).to_str().unwrap(),
        ])
    };
    for (bundle, request, code, outcome) in [
        ("bundle.json", "request.json", 0, "verified_current"),
        ("bundle-tampered.json", "request.json", 1, "rejected"),
        ("bundle.json", "request-expired.json", 1, "expired"),
        (
            "bundle.json",
            "request-historical.json",
            0,
            "verified_historical",
        ),
    ] {
        let out = verify(bundle, request);
        assert_eq!(
            out.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["outcome"], outcome);
    }
    fs::write(root.join("oversize.json"), vec![b' '; 1_048_577]).unwrap();
    assert_eq!(
        verify("oversize.json", "request.json").status.code(),
        Some(2)
    );
    fs::write(root.join("bad.json"), "{broken").unwrap();
    assert_eq!(verify("bad.json", "request.json").status.code(), Some(2));
}

#[test]
fn frozen_demo_matches_expected_outcomes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/demo-v1");
    let expectations: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("expected.json")).unwrap()).unwrap();
    for case in expectations.as_array().unwrap() {
        let out = cli(&[
            "verify",
            root.join(case["bundle"].as_str().unwrap())
                .to_str()
                .unwrap(),
            root.join("policy.json").to_str().unwrap(),
            root.join("registry.json").to_str().unwrap(),
            root.join(case["request"].as_str().unwrap())
                .to_str()
                .unwrap(),
        ]);
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["outcome"], case["outcome"]);
        assert_eq!(
            out.status.code(),
            Some(case["exit_code"].as_i64().unwrap() as i32)
        );
    }
}

#[test]
fn replay_writes_reproducible_files_and_preserves_mismatches() {
    let root = std::env::temp_dir().join(format!(
        "veri-adam-replay-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let _guard = Scratch(root.clone());
    let first = root.join("first");
    let out = cli(&["replay-demo", first.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(first.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["all_matched"], true);
    assert_eq!(report["summary"]["checks"], 31);
    assert_eq!(
        cli(&["replay-demo", first.to_str().unwrap()]).status.code(),
        Some(2)
    );
    let workload = first.join("workload.json");
    let second = root.join("second");
    assert!(
        cli(&[
            "replay",
            workload.to_str().unwrap(),
            second.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert_eq!(
        fs::read(first.join("report.json")).unwrap(),
        fs::read(second.join("report.json")).unwrap()
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(first.join("manifest.json")).unwrap()).unwrap();
    let mut w: veri_adam_replay::Workload =
        serde_json::from_slice(&fs::read(&workload).unwrap()).unwrap();
    assert_eq!(
        manifest["workload_sha256"],
        veri_adam_protocol::digest(&serde_json::to_vec(&w).unwrap())
    );
    assert_eq!(manifest["timing_measured"], false);
    assert_eq!(manifest["report_schema_version"], 2);
    assert_eq!(
        manifest["strategies"],
        serde_json::json!(["b1", "b2", "b3"])
    );
    w.steps[0].expected = veri_adam_protocol::Outcome::Rejected;
    let bad = root.join("wrong-expected.json");
    fs::write(&bad, serde_json::to_vec(&w).unwrap()).unwrap();
    let out = cli(&["replay", bad.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["all_matched"], false);
    assert_eq!(report["summary"]["mismatches"], 1);
}

#[test]
fn dependency_demo_saves_all_strategies_and_index_costs() {
    let root = std::env::temp_dir().join(format!(
        "veri-adam-dependencies-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = Scratch(root.clone());
    let out = cli(&["dependency-demo", root.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["all_matched"], true);
    assert_eq!(report["summary"]["checks"], 27);
    assert!(
        report["summary"]["b3_dependency_work"]["snapshot_entries_examined"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(
        report["summary"]["b2_work"]["signature_checks"],
        report["summary"]["b3_work"]["signature_checks"]
    );
    assert_eq!(
        cli(&["dependency-demo", root.to_str().unwrap()])
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn simulation_saves_a_viewer_and_reproduces_from_config() {
    let root = std::env::temp_dir().join(format!(
        "veri-adam-sim-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let _guard = Scratch(root.clone());
    let first = root.join("first");
    let out = cli(&["simulate", first.to_str().unwrap(), "mixed", "42"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for file in [
        "config.json",
        "simulation.json",
        "simulation.html",
        "workload.json",
        "report.json",
        "manifest.json",
    ] {
        assert!(first.join(file).is_file());
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(first.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["data_kind"], "synthetic");
    assert_eq!(manifest["host_timing_measured"], false);
    assert_eq!(manifest["public_test_keys"], true);
    let second = root.join("second");
    assert!(
        cli(&[
            "simulate-config",
            first.join("config.json").to_str().unwrap(),
            second.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert_eq!(
        fs::read(first.join("simulation.json")).unwrap(),
        fs::read(second.join("simulation.json")).unwrap()
    );
    assert_eq!(
        cli(&["simulate", first.to_str().unwrap()]).status.code(),
        Some(2)
    );
    let out = cli(&["replay", first.join("workload.json").to_str().unwrap()]);
    assert!(out.status.success());
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["all_matched"], true);
}

#[test]
fn invalid_simulation_arguments_fail_before_creating_a_run() {
    let root = std::env::temp_dir().join(format!(
        "veri-adam-bad-sim-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert_eq!(
        cli(&["simulate", root.to_str().unwrap(), "unknown"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        cli(&["simulate", root.to_str().unwrap(), "mixed", "not-a-seed"])
            .status
            .code(),
        Some(2)
    );
    assert!(!root.exists());
}
