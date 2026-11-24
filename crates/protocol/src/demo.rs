//! Public deterministic synthetic fixtures. NEVER use these known keys for real devices.
use super::*;

pub fn fixture(mode: CrewMode, co2: [u32; 2]) -> (Policy, Registry, Request, Bundle) {
    let voter_count = if mode == CrewMode::NominalThree { 3 } else { 2 };
    let identities = [
        ("sensor-a", "pi-a", Role::Sensor),
        ("sensor-b", "pi-b", Role::Sensor),
        ("voter-a", "pi-a", Role::Voter),
        ("voter-b", "pi-b", Role::Voter),
        ("voter-c", "pi-a", Role::Voter),
        ("coordinator", "pi-b", Role::Coordinator),
    ];
    let registry = Registry {
        version: 1,
        members: identities
            .iter()
            .enumerate()
            .map(|(i, (id, node, role))| Member {
                signer_id: (*id).into(),
                node_id: (*node).into(),
                role: role.clone(),
                public_key: public_key(&[(i + 1) as u8; 32]),
                revoked: false,
            })
            .collect(),
    };
    let policy = Policy {
        protocol_version: VERSION,
        policy_version: 1,
        threshold_ppm: 800,
        ttl_ms: 10_000,
        sources: vec!["sensor-a".into(), "sensor-b".into()],
        voters: ["voter-a", "voter-b", "voter-c"][..voter_count]
            .iter()
            .map(|s| (*s).into())
            .collect(),
        coordinator: "coordinator".into(),
        crew_mode: mode,
    };
    let event_id = "synthetic-event-001".to_string();
    let nonce = "ab".repeat(32);
    let measurements: Vec<_> = co2
        .iter()
        .enumerate()
        .map(|(i, ppm)| {
            let payload = Measurement {
                protocol_version: VERSION,
                event_id: event_id.clone(),
                nonce: nonce.clone(),
                signer_id: policy.sources[i].clone(),
                sequence: 1,
                captured_at_ms: 1_000,
                co2_ppm: *ppm,
                temperature_millic: 24_000,
                humidity_millipercent: 50_000,
                light_millilux: 120_000,
                calibration_version: 1,
                policy_hash: policy.id(),
            };
            SignedMeasurement {
                signature: sign(&payload.canonical_bytes(), &[(i + 1) as u8; 32]),
                payload,
            }
        })
        .collect();
    let mut ids: Vec<_> = measurements.iter().map(|m| m.payload.id()).collect();
    ids.sort();
    let payload = Decision {
        protocol_version: VERSION,
        event_id: event_id.clone(),
        nonce: nonce.clone(),
        policy_hash: policy.id(),
        registry_version: registry.version,
        coordinator_id: policy.coordinator.clone(),
        measurement_ids: ids,
        advisory: u64::from(co2[0]) + u64::from(co2[1]) >= u64::from(policy.threshold_ppm) * 2,
    };
    let decision = SignedDecision {
        signature: sign(&payload.canonical_bytes(), &[6; 32]),
        payload,
    };
    let ballots = policy
        .voters
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let payload = Ballot {
                protocol_version: VERSION,
                event_id: event_id.clone(),
                nonce: nonce.clone(),
                decision_hash: decision.payload.id(),
                voter_id: id.clone(),
                advisory: decision.payload.advisory,
            };
            SignedBallot {
                signature: sign(&payload.canonical_bytes(), &[(i + 3) as u8; 32]),
                payload,
            }
        })
        .collect();
    let heads = measurements
        .iter()
        .map(|m| (m.payload.signer_id.clone(), m.payload.id()))
        .collect();
    let request = Request {
        event_id,
        nonce,
        now_ms: 2_000,
        kind: CheckKind::Current,
        heads,
    };
    (
        policy,
        registry,
        request,
        Bundle {
            decision,
            measurements,
            ballots,
        },
    )
}
