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
                "report_schema_version": 2,
                "strategies": ["b1", "b2", "b3"],
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
            "All outcomes match expected, B1, B2, B3 and reference: {}",
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

fn simulate(config: veri_adam_simulation::Config, output: &str) -> Result<bool, String> {
    let simulation = veri_adam_simulation::run(config)?;
    let html = veri_adam_simulation::viewer::html(&simulation)?;
    let dir = new_directory(output)?;
    write(dir, "config.json", &simulation.config)?;
    write(dir, "workload.json", &simulation.workload)?;
    write(dir, "report.json", &simulation.verification)?;
    write(dir, "simulation.json", &simulation)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("simulation.html"))
        .map_err(|e| e.to_string())?;
    file.write_all(html.as_bytes()).map_err(|e| e.to_string())?;
    write(
        dir,
        "manifest.json",
        &serde_json::json!({
            "software_version":env!("CARGO_PKG_VERSION"),"simulation_schema_version":1,"report_schema_version":2,
            "run_kind":"virtual_time_simulation","data_kind":"synthetic","public_test_keys":true,
            "strategies":["b1","b2","b3"],"host_timing_measured":false,"energy_measured":false,
            "model":"prescribed_co2_and_light_v1","workload_sha256":digest(&serde_json::to_vec(&simulation.workload).map_err(|e|e.to_string())?),
            "config_sha256":digest(&serde_json::to_vec(&simulation.config).map_err(|e|e.to_string())?),
            "hash_encoding":"compact serde_json serialization of typed input"
        }),
    )?;
    println!(
        "Simulation viewer: {}",
        dir.join("simulation.html").display()
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&simulation.summary).map_err(|e| e.to_string())?
    );
    println!(
        "Verification checks: {}; all expected/B1/B2/B3/reference outcomes agree: {}",
        simulation.verification.summary.checks, simulation.verification.all_matched
    );
    Ok(simulation.verification.all_matched)
}
fn run(args: &[String]) -> Result<bool, String> {
    match args {
        [cmd, output, mode] if cmd == "agent-simulate" => {
            let mut reasoner: Box<dyn veri_adam_agents::Reasoner> = match mode.as_str() {
                "rules" => Box::new(veri_adam_agents::RuleReasoner),
                "ollama" => Box::new(veri_adam_agents::OllamaReasoner {
                    model: "qwen3:1.7b".into(),
                    adapter: Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../scripts/ollama_agent.py"),
                }),
                _ => return Err("agent mode must be rules or ollama".into()),
            };
            let dir = new_directory(output)?;
            println!(
                "Preparing {mode} before evidence capture (cold Ollama loading may take up to two minutes)."
            );
            let preparation = reasoner.prepare()?;
            if let Some(preparation) = &preparation {
                write(dir, "model-preparation.json", preparation)?;
            }
            println!(
                "Running message-driven agents ({mode}); local model inference can take up to 20 s per round."
            );
            let run = veri_adam_agents::showcase(reasoner, mode.clone())?;
            write(dir, "agent-run.json", &run)?;
            let html = veri_adam_agents::html(&run)?;
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dir.join("simulation.html"))
                .map_err(|e| e.to_string())?
                .write_all(html.as_bytes())
                .map_err(|e| e.to_string())?;
            write(
                dir,
                "manifest.json",
                &serde_json::json!({
                    "software_version":env!("CARGO_PKG_VERSION"),"run_kind":"local_message_driven_agents",
                    "data_kind":run.data_kind,"public_test_keys":true,"zkp_implemented":false,
                    "mode":mode,"agent_run_sha256":digest(&serde_json::to_vec(&run).map_err(|e|e.to_string())?),
                    "model_preparation":preparation,
                    "hash_encoding":"compact serde_json serialization of typed agent run",
                    "wall_time_affects_currentness":true,"pi_benchmark":false
                }),
            )?;
            for round in &run.rounds {
                println!(
                    "{}: {} ({} signed assessments, {} reasoner responses) — {}",
                    round.name,
                    round.state,
                    round.assessments.len(),
                    round.inferences.len(),
                    round.reason
                );
                for absent in &round.absent {
                    println!("  {absent}");
                }
            }
            println!("Viewer: {}", dir.join("simulation.html").display());
            Ok(run.all_reference_outcomes_match
                && (mode != "ollama" || run.rounds.iter().any(|r| !r.inferences.is_empty())))
        }
        [cmd, dir] if cmd == "simulate" => simulate(veri_adam_simulation::Config::default(), dir),
        [cmd, dir, scenario] if cmd == "simulate" => simulate(
            veri_adam_simulation::Config {
                scenario: veri_adam_simulation::Scenario::parse(scenario)?,
                ..Default::default()
            },
            dir,
        ),
        [cmd, dir, scenario, seed] if cmd == "simulate" => simulate(
            veri_adam_simulation::Config {
                scenario: veri_adam_simulation::Scenario::parse(scenario)?,
                seed: seed
                    .parse()
                    .map_err(|_| "seed must be an unsigned 64-bit integer")?,
                ..Default::default()
            },
            dir,
        ),
        [cmd, input, dir] if cmd == "simulate-config" => simulate(read(input)?, dir),
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
        [cmd, dir] if cmd == "dependency-demo" => {
            replay(veri_adam_replay::dependencies_demo::workload(), Some(dir))
        }
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
        "veri-adam: experimental signed-event verifier\n\n  demo NEW_DIRECTORY\n  verify BUNDLE POLICY REGISTRY REQUEST\n  replay-demo NEW_DIRECTORY\n  dependency-demo NEW_DIRECTORY\n  replay WORKLOAD [NEW_DIRECTORY]\n  simulate NEW_DIRECTORY [SCENARIO [SEED]]\n  simulate-config CONFIG NEW_DIRECTORY\n  agent-simulate NEW_DIRECTORY ollama|rules\n\nScenarios: stable, dropout, partition, delay, tamper, replay, false-reading, mixed.\nPolicy, registry and request are trusted verifier inputs, separate from the bundle.\nReplay workloads contain trusted snapshots and expected outcomes; use trusted files.\nExit: 0 verified/all checks match; 1 verification withheld or mismatch; 2 input/operational error.\nSimulation uses virtual time, toy environmental data and public test keys. No live sensors, actuation, production keys or ZKP."
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
