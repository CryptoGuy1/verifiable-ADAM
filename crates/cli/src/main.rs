use serde::{Serialize, de::DeserializeOwned};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    process::ExitCode,
};
use veri_adam_protocol::*;

fn read<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let f = File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut bytes = Vec::new();
    f.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(format!("{path}: exceeds 1 MiB input limit"));
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("{path}: {e}"))
}
fn write<T: Serialize>(dir: &Path, name: &str, value: &T) -> Result<(), String> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(name))
        .map_err(|e| e.to_string())?;
    let data = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    f.write_all(&data)
        .and_then(|_| f.write_all(b"\n"))
        .map_err(|e| e.to_string())
}

fn new_directory(path: &str) -> Result<&Path, String> {
    let dir = Path::new(path);
    if let Some(parent) = dir.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::create_dir(dir).map_err(|e| format!("Use a new output directory: {e}"))?;
    Ok(dir)
}

fn replay(workload: veri_adam_replay::Workload, output: Option<&str>) -> Result<bool, String> {
    let report = veri_adam_replay::run(&workload)?;
    if let Some(output) = output {
        let dir = new_directory(output)?;
        write(dir, "workload.json", &workload)?;
        write(dir, "report.json", &report)?;
        let normalized = serde_json::to_vec(&workload).map_err(|e| e.to_string())?;
        write(
            dir,
            "manifest.json",
            &serde_json::json!({
                "software_version": env!("CARGO_PKG_VERSION"),
                "workload_sha256": digest(&normalized),
                "hash_encoding": "compact serde_json serialization of the typed workload",
                "data_kind": workload.data_kind,
                "run_kind": "software_counter_replay",
                "timing_measured": false,
                "energy_measured": false
            }),
        )?;
        println!("Replay files: {}", dir.display());
        println!(
            "{}",
            serde_json::to_string_pretty(&report.summary).map_err(|e| e.to_string())?
        );
        println!(
            "All outcomes match expected, B1, B2 and reference: {}",
            report.all_matched
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
    }
    Ok(report.all_matched)
}
fn run(args: &[String]) -> Result<bool, String> {
    match args {
        [cmd, dir] if cmd == "demo" => {
            let dir = new_directory(dir)?;
            let (policy, registry, request, bundle) =
                demo::fixture(CrewMode::NominalThree, [900, 850]);
            write(dir, "policy.json", &policy)?;
            write(dir, "registry.json", &registry)?;
            write(dir, "request.json", &request)?;
            write(dir, "bundle.json", &bundle)?;
            let report = veri_adam_verification::check(&bundle, &policy, &registry, &request);
            let reference = veri_adam_reference::check(&bundle, &policy, &registry, &request);
            write(dir, "report.json", &report)?;
            if report.outcome != reference {
                return Err("reference disagreement".into());
            }
            // Demonstrate a fresh authentic historical event that is no longer current.
            let mut expired = request.clone();
            expired.now_ms = 11_000;
            write(dir, "request-expired.json", &expired)?;
            expired.kind = CheckKind::Historical;
            write(dir, "request-historical.json", &expired)?;
            let mut tampered = bundle.clone();
            tampered.measurements[0].payload.co2_ppm += 1;
            write(dir, "bundle-tampered.json", &tampered)?;
            println!(
                "Synthetic demo only; public test keys, not real sensor data.\nFiles: {}\nOutcome: {:?}\nReference agrees: true",
                dir.display(),
                report.outcome
            );
            Ok(true)
        }
        [cmd, dir] if cmd == "replay-demo" => replay(veri_adam_replay::demo::workload(), Some(dir)),
        [cmd, input] if cmd == "replay" => replay(read(input)?, None),
        [cmd, input, dir] if cmd == "replay" => replay(read(input)?, Some(dir)),
        [cmd, bundle, policy, registry, request] if cmd == "verify" => {
            let b = read(bundle)?;
            let p = read(policy)?;
            let r = read(registry)?;
            let q = read(request)?;
            let report = veri_adam_verification::check(&b, &p, &r, &q);
            let reference = veri_adam_reference::check(&b, &p, &r, &q);
            if reference != report.outcome {
                return Err(format!(
                    "reference disagreement: {reference:?} vs {:?}",
                    report.outcome
                ));
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            );
            Ok(matches!(
                report.outcome,
                Outcome::VerifiedCurrent | Outcome::VerifiedHistorical
            ))
        }
        [cmd] if cmd == "--help" || cmd == "help" => {
            help();
            Ok(true)
        }
        [] => {
            help();
            Ok(true)
        }
        _ => Err("Invalid arguments. Run veri-adam --help".into()),
    }
}
fn help() {
    println!(
        "veri-adam: experimental signed-event verifier\n\n  demo NEW_DIRECTORY\n  verify BUNDLE POLICY REGISTRY REQUEST\n  replay-demo NEW_DIRECTORY\n  replay WORKLOAD [NEW_DIRECTORY]\n\nPolicy, registry and request are trusted verifier inputs, separate from the bundle.\nReplay workloads contain trusted snapshots and expected outcomes; use trusted files.\nExit: 0 verified/all replay checks match; 1 verification withheld or replay mismatch; 2 input/operational error.\nB2 uses in-memory caches across replay steps. No live sensing, actuation, production keys, B3 or ZKP."
    );
}
fn main() -> ExitCode {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
