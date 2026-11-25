use super::{new_directory, read, write};
use std::{fs::OpenOptions, io::Write, path::Path};
use veri_adam_decision::{calibration::*, reasoner::*, *};

pub fn calibrate_file(input: &str, output: &str) -> Result<bool, String> {
    let data: CalibrationData = read(input)?;
    let profile = calibrate(&data)?;
    let dir = new_directory(output)?;
    write(dir, "calibration-input.json", &data)?;
    write(dir, "calibration-profile.json", &profile)?;
    println!(
        "Calibration profile: {}",
        dir.join("calibration-profile.json").display()
    );
    println!(
        "Synthetic: {}; profile hash: {}",
        profile.synthetic,
        profile.id()
    );
    Ok(true)
}
pub fn evaluate_file(event: &str, policy: &str, profile: &str) -> Result<bool, String> {
    let event: EventInput = read(event)?;
    let policy: ApplicationPolicy = read(policy)?;
    let profile: CalibrationProfile = read(profile)?;
    let clock = MonotonicClock::new(event.started_at_ms);
    let mut reasoner = RuleReasoner {
        confidence: policy.fallback_confidence,
    };
    let record =
        DecisionEngine::new().evaluate(&event, &policy, &profile, &mut reasoner, &clock)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?
    );
    Ok(record.status == StageStatus::ApprovedAwaitingRecording)
}
pub fn run(output: &str, mode: &str) -> Result<bool, String> {
    if !["rules", "ollama"].contains(&mode) {
        return Err("ADAM decision mode must be rules or ollama".into());
    }
    let policy = fixtures::policy();
    policy.validate()?;
    let data = fixtures::calibration_data();
    let profile = calibrate(&data)?;
    let dir = new_directory(output)?;
    write(dir, "policy.json", &policy)?;
    write(dir, "calibration-input.json", &data)?;
    write(dir, "calibration-profile.json", &profile)?;
    let mut reasoner: Box<dyn Reasoner> = if mode == "ollama" {
        let r = OllamaReasoner {
            model: "qwen3:1.7b".into(),
            adapter: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/adam_reasoner.py"),
        };
        println!(
            "Preparing local Qwen before observations; cold loading can take up to two minutes."
        );
        write(dir, "model-preparation.json", &r.prepare()?)?;
        Box::new(r)
    } else {
        Box::new(RuleReasoner {
            confidence: policy.fallback_confidence,
        })
    };
    let mut engine = DecisionEngine::new();
    let mut records = vec![];
    let mut inputs = vec![];
    let mut start = 1000;
    for (index, (name, values)) in [
        ("weighted-normal", [790, 830]),
        ("weighted-anomaly", [900, 500]),
        ("temporal-spike", [700, 400]),
        ("degraded-crew", [900, 850]),
        ("missing-ballot", [790, 900]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut event = fixtures::event(
            name,
            start,
            &[("sensor-a", values[0]), ("sensor-b", values[1])],
        );
        if index == 2 {
            event.temporal_window = vec![PastReading {
                evidence_id: "synthetic-prior-300".into(),
                source_id: "sensor-a".into(),
                captured_at_ms: start - 1,
                co2_ppm: 300,
            }];
        }
        if index == 3 {
            event.availability.aggregator = false;
        }
        if index == 4 {
            event.missing_ballots.insert(VotingRole::Decision);
        }
        println!("Processing {name} through the weighted decision stage...");
        let clock = MonotonicClock::new(start);
        let record = engine.evaluate(&event, &policy, &profile, reasoner.as_mut(), &clock)?;
        println!(
            "  {:?}; fusion {:?}; crew {:?}; action released: {}",
            record.status,
            record.fusion.as_ref().map(|f| f.threshold_class),
            record.final_classification,
            record.action_released
        );
        start = record.completed_at_ms + 1000;
        inputs.push(event);
        records.push(record);
    }
    write(dir, "events.json", &inputs)?;
    write(dir, "decision-stage-run.json", &records)?;
    // Save individual cases for independent CLI evaluation with explicit trusted policy/profile.
    for (index, event) in inputs.iter().enumerate() {
        write(dir, &format!("event-{}.json", index + 1), event)?;
    }
    let metadata = serde_json::json!({"software_version":env!("CARGO_PKG_VERSION"),"run_kind":"paper_derived_adam_decision_stage",
        "data_kind":"synthetic","calibration_hash":profile.id(),"policy_hash":policy.id(),"mode":mode,
        "records_sha256":veri_adam_protocol::digest(&serde_json::to_vec(&records).map_err(|e|e.to_string())?),
        "source":"ADAM_EAAI-3.pdf sections 3.2.3, 3.3, 3.4.2, 3.4.5, 3.5",
        "requirements":["REQ-01","REQ-03","REQ-04"],"fully_spec_compliant":false,
        "authenticated_v2_integration":false,"semantic_vector_retrieval":false,"zkp":false,"durable_audit":false,"pi_validation":false});
    write(dir, "manifest.json", &metadata)?;
    let mut note = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("README.txt"))
        .map_err(|e| e.to_string())?;
    note.write_all(b"This is a shared ADAM decision-stage implementation, not a completed authenticated event system.\nSynthetic calibration and configurable application actions are development fixtures.\nPolicy approval does not release an action: signed-v2, durable recording, private proofs and dependency integration remain unfinished.\nThe original v1 signed fixtures are preserved separately.\n").map_err(|e|e.to_string())?;
    println!(
        "Records saved to {}. No action released; cryptographic v2 and durable recording are not yet integrated.",
        dir.display()
    );
    Ok(true)
}
