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
fn run(args: &[String]) -> Result<bool, String> {
    match args {
        [cmd, dir] if cmd == "demo" => {
            let dir = Path::new(dir);
            if let Some(parent) = dir.parent().filter(|p| !p.as_os_str().is_empty()) {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::create_dir(dir).map_err(|e| format!("Use a new output directory: {e}"))?;
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
        "veri-adam: experimental signed-event verifier\n\n  demo NEW_DIRECTORY\n  verify BUNDLE POLICY REGISTRY REQUEST\n\nPolicy, registry and request are trusted verifier inputs, separate from the bundle.\nExit: 0 verified/success; 1 rejected, expired or unconfirmed; 2 input/operational error.\nNo physical actuation, live sensing, production keys, caching or ZKP in this milestone."
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
