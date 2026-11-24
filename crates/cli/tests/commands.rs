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
