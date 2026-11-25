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
pub fn private_check(
    record: &str,
    proof: &str,
    trust: &str,
    request: &str,
) -> Result<bool, String> {
    let record: Record = read(record)?;
    let proof: private::PrivateProof = read(proof)?;
    let trust: Trust = read(trust)?;
    let request: Request = read(request)?;
    let report = private::check(&record, &proof, &trust, &request);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    Ok(report.accepted)
}
pub fn audit_check(directory: &str, trust: &str, pin: &str) -> Result<bool, String> {
    let trust: Trust = read(trust)?;
    let pin: Option<Signed<audit::Checkpoint>> = if pin == "-" { None } else { Some(read(pin)?) };
    let checkpoint = audit::verify(Path::new(directory), &trust, pin.as_ref())?;
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"verified":true,"checkpoint":checkpoint,"independent_pin_supplied":pin.is_some(),"rollback_protection":"requires an independently retained latest checkpoint"})).map_err(|e|e.to_string())?);
    Ok(true)
}
pub fn release_check(
    release: &str,
    record: &str,
    trust: &str,
    directory: &str,
    pin: &str,
) -> Result<bool, String> {
    let release: Signed<audit::Release> = read(release)?;
    let record: Record = read(record)?;
    let trust: Trust = read(trust)?;
    let pin: Option<Signed<audit::Checkpoint>> = if pin == "-" { None } else { Some(read(pin)?) };
    audit::verify_release(
        &release,
        &record,
        &trust,
        Path::new(directory),
        pin.as_ref(),
    )?;
    println!(
        "{}",
        serde_json::json!({"authorization_history_verified":true,"currentness_verified":false,"full_decision_processing_verified":false})
    );
    Ok(true)
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

pub fn agent_run(output: &str, mode: &str) -> Result<bool, String> {
    if !["rules", "ollama", "ollama-history"].contains(&mode) {
        return Err("mode must be rules, ollama or ollama-history".into());
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
    write(dir, "trust.json", &trust)?;
    let export = dir.join("private-export");
    std::fs::create_dir(&export).map_err(|e| e.to_string())?;
    write(&export, "trust.json", &trust)?;
    write(dir, "calibration-input.json", &calibration_input)?;
    if mode == "ollama-history" {
        write(
            dir,
            "history-preparation.json",
            &history_call("init", None, 10000)?,
        )?;
    }
    let reasoner: Box<dyn Reasoner + Send> = if mode.starts_with("ollama") {
        let r = OllamaReasoner {
            model: "qwen3:1.7b".into(),
            adapter: Path::new(env!("CARGO_MANIFEST_DIR")).join(if mode == "ollama-history" {
                "../../scripts/adam_reasoner_history.py"
            } else {
                "../../scripts/adam_reasoner.py"
            }),
        };
        println!("Preparing local Qwen before agent evidence capture...");
        write(dir, "model-preparation.json", &r.prepare()?)?;
        Box::new(r)
    } else {
        Box::new(RuleReasoner { confidence: 0.8 })
    };
    let audit_keys = Keyring(BTreeMap::from([(
        trust.coordinator.clone(),
        keys.0
            .get(&trust.coordinator)
            .ok_or("coordinator key missing")?
            .clone(),
    )]));
    let mut journal = audit::Journal::open(&dir.join("audit"), &trust, &audit_keys, None)?;
    let mut runtime = veri_adam_agents::weighted::Runtime::new(trust.clone(), keys, reasoner)?;
    let mut rounds = Vec::new();
    for (i, (name, a, b)) in [
        ("weighted-normal", 790, 830),
        ("weighted-anomaly", 900, 500),
        ("temporal-spike", 700, 400),
        ("degraded-crew", 900, 850),
        ("missing-ballot", 790, 900),
        ("tampered-source", 900, 850),
    ]
    .into_iter()
    .enumerate()
    {
        let start = 100_000 + i as u64 * 60_000;
        let mut event = fixtures::event(name, start, &[("sensor-a", a), ("sensor-b", b)]);
        event.nonce = fresh_nonce()?;
        if i == 2 {
            event.temporal_window = vec![PastReading {
                evidence_id: "replay-prior-300".into(),
                source_id: "sensor-a".into(),
                captured_at_ms: start - 5000,
                co2_ppm: 300,
            }];
        }
        if i == 3 {
            event.availability.aggregator = false;
        }
        if i == 4 {
            event.missing_ballots.insert(VotingRole::Decision);
        }
        let processing = std::time::Instant::now();
        let mut round = runtime.round(event, i == 5)?;
        if let (Some(record), Some(disclosure), Some(request)) =
            (&round.record, &round.disclosure, &round.request)
        {
            let began = std::time::Instant::now();
            match private::prove(record, disclosure, &trust, request) {
                Ok(proof) => {
                    let mut proof_request = request.clone();
                    proof_request.now_ms = proof_request
                        .now_ms
                        .saturating_add(began.elapsed().as_millis() as u64);
                    round.private_verification =
                        Some(private::check(record, &proof, &trust, &proof_request));
                    round.private_proof = Some(proof);
                }
                Err(error) => {
                    println!("{name}: private predicate withheld: {error}");
                }
            }
        }
        println!(
            "{name}: {}",
            round
                .verification
                .as_ref()
                .map(|r| r.outcome.as_str())
                .unwrap_or("withheld_before_decision")
        );
        let case = dir.join(name);
        std::fs::create_dir(&case).map_err(|e| e.to_string())?;
        let mut required_history_ack = mode != "ollama-history";
        if mode == "ollama-history"
            && round
                .verification
                .as_ref()
                .is_some_and(|v| v.outcome == "verified_current_awaiting_recording")
            && let (Some(record), Some(disclosure)) = (&round.record, &round.disclosure)
        {
            let elapsed = processing.elapsed().as_millis() as u64;
            let payload = serde_json::json!({"record":record,"disclosure":disclosure,"resolved_at_ms":round.event.started_at_ms+elapsed,"remaining_ms":trust.policy.event_budget_ms.saturating_sub(elapsed)});
            match history_call(
                "record",
                Some(payload),
                trust.policy.event_budget_ms.saturating_sub(elapsed),
            ) {
                Ok(ack) => {
                    required_history_ack = ack["recording_acknowledged"] == true;
                    write(&case, "history-recording-ack.json", &ack)?;
                }
                Err(error) => {
                    write(
                        &case,
                        "history-recording-failed.json",
                        &serde_json::json!({"error":error}),
                    )?;
                }
            }
        }
        let now = round
            .event
            .started_at_ms
            .saturating_add(processing.elapsed().as_millis() as u64);
        for invalidation in &round.invalidations {
            journal.append(audit::Event {
                kind: audit::Kind::DependencyInvalidation,
                context: Context::from(&round.event),
                proposal_id: Some(invalidation.claim_id.clone()),
                outcome: "support_changed_or_expired".into(),
                at_ms: now,
            })?;
        }
        let checkpoint = journal.append(audit::Event {
            kind: audit::Kind::Attempt,
            context: Context::from(&round.event),
            proposal_id: round
                .record
                .as_ref()
                .map(|r| canonical::id("proposal", &r.proposal.payload)),
            outcome: round
                .verification
                .as_ref()
                .map(|r| r.outcome.clone())
                .unwrap_or_else(|| "withheld_before_decision".into()),
            at_ms: now,
        })?;
        write(&case, "recording-ack.json", &checkpoint)?;
        if required_history_ack
            && round
                .private_verification
                .as_ref()
                .is_some_and(|p| p.accepted)
            && let (Some(record), Some(disclosure), Some(request)) =
                (&round.record, &round.disclosure, &round.request)
        {
            let clock = MonotonicClock::new(
                round
                    .event
                    .started_at_ms
                    .saturating_add(processing.elapsed().as_millis() as u64),
            );
            match journal.authorize(record, disclosure, request, &clock) {
                Ok(release) => {
                    write(&case, "advisory-authorization.json", &release)?;
                    println!("{name}: advisory authorization durably recorded");
                }
                Err(error) => {
                    write(
                        &case,
                        "release-withheld.json",
                        &serde_json::json!({"reason":error}),
                    )?;
                }
            }
        }
        write(&case, "round.json", &round)?;
        if let Some(record) = &round.record {
            write(&case, "record.json", record)?;
        }
        if let Some(disclosure) = &round.disclosure {
            write(&case, "authorized-disclosure.json", disclosure)?;
        }
        if let Some(request) = &round.request {
            write(&case, "request.json", request)?;
        }
        if let Some(proof) = &round.private_proof {
            write(&case, "private-proof.json", proof)?;
        }
        if let Some(report) = &round.private_verification {
            write(&case, "private-verification.json", report)?;
        }
        if round.private_proof.is_some() {
            let public_case = export.join(name);
            std::fs::create_dir(&public_case).map_err(|e| e.to_string())?;
            for file in [
                "record.json",
                "private-proof.json",
                "request.json",
                "private-verification.json",
                "advisory-authorization.json",
            ] {
                if case.join(file).exists() {
                    std::fs::copy(case.join(file), public_case.join(file))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        rounds.push(round);
    }
    write(dir, "agent-run.json", &rounds)?;
    let final_checkpoint = journal.checkpoint.clone();
    drop(journal);
    audit::verify(&dir.join("audit"), &trust, Some(&final_checkpoint))?;
    write(dir, "latest-checkpoint.json", &final_checkpoint)?;
    let exported_audit = export.join("audit");
    std::fs::create_dir(&exported_audit).map_err(|e| e.to_string())?;
    for name in ["events.jsonl", "checkpoint.json"] {
        std::fs::copy(dir.join("audit").join(name), exported_audit.join(name))
            .map_err(|e| e.to_string())?;
    }
    write(&export, "latest-checkpoint.json", &final_checkpoint)?;
    write(
        dir,
        "verification-work.json",
        &serde_json::json!({"ordinary":runtime.ordinary.work,"dependency_cache":runtime.dependencies.cache.work,"dependency_maintenance":runtime.dependencies.work,"claim":"development operation counts; no efficiency advantage asserted"}),
    )?;
    write(
        dir,
        "manifest.json",
        &serde_json::json!({"kind":"weighted-authenticated-agent-runtime","mode":mode,"data_kind":"synthetic","keys":"OS-random ephemeral; each role key owned by its worker","ui_updated":false,"pi_deployment":false,"action_released":false,"private_fusion_proof_implemented":true,"private_full_decision_processing_proved":false}),
    )?;
    Ok(true)
}

fn history_call(
    operation: &str,
    payload: Option<serde_json::Value>,
    remaining_ms: u64,
) -> Result<serde_json::Value, String> {
    use std::{
        io::{Read, Write},
        process::{Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    if remaining_ms == 0 {
        return Err("no history-service budget remains".into());
    }
    let mut child = Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/history_store.py"))
        .arg(operation)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(payload) = payload {
        child
            .stdin
            .take()
            .ok_or("history stdin unavailable")?
            .write_all(&serde_json::to_vec(&payload).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    } else {
        drop(child.stdin.take());
    }
    let stdout = child.stdout.take().ok_or("history stdout unavailable")?;
    let stderr = child.stderr.take().ok_or("history stderr unavailable")?;
    let output = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(65537).read_to_end(&mut bytes).map(|_| bytes)
    });
    let error = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.take(8193).read_to_end(&mut bytes).map(|_| bytes)
    });
    let stop = Instant::now() + Duration::from_millis(remaining_ms);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if Instant::now() >= stop {
            let _ = child.kill();
            let _ = child.wait();
            return Err("history acknowledgment missed event deadline".into());
        }
        thread::sleep(Duration::from_millis(5));
    };
    let out = output
        .join()
        .map_err(|_| "history reader panicked")?
        .map_err(|e| e.to_string())?;
    let err = error
        .join()
        .map_err(|_| "history stderr reader panicked")?
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(String::from_utf8_lossy(&err).chars().take(600).collect());
    }
    if out.len() > 65536 {
        return Err("history acknowledgment exceeded bounds".into());
    }
    serde_json::from_slice(&out).map_err(|e| e.to_string())
}
