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
