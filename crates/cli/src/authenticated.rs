use super::{new_directory, read, write};
use std::{collections::BTreeMap, path::Path};
use veri_adam_decision::{
    calibration::calibrate,
    fixtures,
    reasoner::{OllamaReasoner, RuleReasoner},
    *,
};
use veri_adam_records::{canonical, checker, *};

pub fn check(record: &str, disclosure: &str, trust: &str, request: &str) -> Result<bool, String> {
    let record: Record = read(record)?;
    let disclosure: Option<Disclosure> = if disclosure == "-" {
        None
    } else {
        Some(read(disclosure)?)
    };
    let trust: Trust = read(trust)?;
    let request: Request = read(request)?;
    let report = checker::check(&record, disclosure.as_ref(), &trust, &request);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    Ok(report.outcome.starts_with("verified_"))
}
pub fn run(output: &str, mode: &str) -> Result<bool, String> {
    if !["rules", "ollama"].contains(&mode) {
        return Err("mode must be rules or ollama".into());
    }
    let dir = new_directory(output)?;
    let (keys, registry) = Keyring::local()?;
    let calibration_input = fixtures::calibration_data();
    let trust = Trust {
        registry,
        policy: fixtures::policy(),
        calibration: calibrate(&calibration_input)?,
        voters: BTreeMap::from([
            (VotingRole::Sensor, "sensor-role".into()),
            (VotingRole::Aggregator, "aggregator-role".into()),
            (VotingRole::Decision, "decision-role".into()),
        ]),
        coordinator: "coordinator".into(),
    };
    trust.validate()?;
    write(dir, "trust.json", &trust)?;
    write(dir, "calibration-input.json", &calibration_input)?;
    let mut reasoner: Box<dyn Reasoner> = if mode == "ollama" {
        let reasoner = OllamaReasoner {
            model: "qwen3:1.7b".into(),
            adapter: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/adam_reasoner.py"),
        };
        println!("Preparing local Qwen before recording evidence...");
        write(dir, "model-preparation.json", &reasoner.prepare()?)?;
        Box::new(reasoner)
    } else {
        Box::new(RuleReasoner { confidence: 0.8 })
    };
    let mut results = Vec::new();
    let mut file_hashes = BTreeMap::new();
    for name in [
        "trust.json",
        "calibration-input.json",
        "model-preparation.json",
    ] {
        if dir.join(name).exists() {
            file_hashes.insert(
                name.to_owned(),
                veri_adam_protocol::digest(
                    &std::fs::read(dir.join(name)).map_err(|e| e.to_string())?,
                ),
            );
        }
    }
    for (i, (name, a, b)) in [
        ("weighted-normal", 790, 830),
        ("weighted-anomaly", 900, 500),
        ("temporal-spike", 700, 400),
        ("degraded-crew", 900, 850),
        ("missing-ballot", 790, 900),
    ]
    .into_iter()
    .enumerate()
    {
        let start = 100_000 + i as u64 * 60_000;
        let mut e = fixtures::event(name, start, &[("sensor-a", a), ("sensor-b", b)]);
        // Trusted replay challenge is fresh per run; not a network challenge service.
        e.nonce = canonical::id("local-challenge", &(trust.registry.id(), name));
        if i == 2 {
            e.temporal_window.push(PastReading {
                evidence_id: "prior-synthetic-300".into(),
                source_id: "sensor-a".into(),
                captured_at_ms: start - 5000,
                co2_ppm: 300,
            });
        }
        if i == 3 {
            e.availability.aggregator = false;
        }
        if i == 4 {
            e.missing_ballots.insert(VotingRole::Decision);
        }
        // Independent per-case engine: stateful history must be explicitly exported
        // as authenticated temporal support before crossing this protocol boundary.
        let stage = DecisionEngine::new().evaluate(
            &e,
            &trust.policy,
            &trust.calibration,
            reasoner.as_mut(),
            &MonotonicClock::new(start),
        )?;
        let (record, disclosure) = attest(&e, &stage, &trust, &keys)?;
        let heads = record
            .observations
            .iter()
            .filter(|o| o.payload.purpose == Purpose::Fusion)
            .map(|o| {
                (
                    o.payload.source_id.clone(),
                    canonical::id("observation", &o.payload),
                )
            })
            .collect();
        let request = Request {
            context: Context::from(&e),
            now_ms: stage.completed_at_ms,
            current: true,
            heads,
        };
        let report = checker::check(&record, Some(&disclosure), &trust, &request);
        let case_dir = dir.join(name);
        std::fs::create_dir(&case_dir).map_err(|e| e.to_string())?;
        write(&case_dir, "record.json", &record)?;
        write(&case_dir, "authorized-disclosure.json", &disclosure)?;
        write(&case_dir, "request.json", &request)?;
        write(&case_dir, "verification.json", &report)?;
        for file in [
            "record.json",
            "authorized-disclosure.json",
            "request.json",
            "verification.json",
        ] {
            file_hashes.insert(
                format!("{name}/{file}"),
                veri_adam_protocol::digest(
                    &std::fs::read(case_dir.join(file)).map_err(|e| e.to_string())?,
                ),
            );
        }
        println!("{name}: {}", report.outcome);
        results.push(serde_json::json!({"case":name,"stage_status":stage.status,"independent_verification":report}));
    }
    write(
        dir,
        "manifest.json",
        &serde_json::json!({"kind":"weighted-adam-authenticated-v2-local-development","software_version":env!("CARGO_PKG_VERSION"),"mode":mode,"data_kind":"synthetic","keys":"fresh OS randomness; ephemeral; not exported or device provisioned","registry_hash":trust.registry.id(),"files_sha256":file_hashes,"cases":results,"private_proof_implemented":false,"durable_recording_implemented":false,"action_released":false,"worker_runtime_migrated":false,"pi_deployment":false,"disclosure_warning":"authorized-disclosure.json contains readings, openings and full private reasoner text; never export to an external private verifier"}),
    )?;
    println!(
        "Saved signed records and independent reports to {}. No advisory was released.",
        dir.display()
    );
    Ok(true)
}
