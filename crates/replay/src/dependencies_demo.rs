//! Multi-claim synthetic topology: two disjoint source pairs and two ballot
//! variants of claim A. Four logical sources are not four physical sensors.
use super::*;

type Snapshot = (Policy, Registry, Request, Bundle);
fn step(label: &str, s: &Snapshot, expected: Outcome) -> Step {
    Step {
        label: label.into(),
        reset_cache: false,
        policy: s.0.clone(),
        registry: s.1.clone(),
        request: s.2.clone(),
        bundle: s.3.clone(),
        expected,
    }
}
fn resign(s: &mut Snapshot, sensor_seeds: [u8; 2]) {
    for (i, m) in s.3.measurements.iter_mut().enumerate() {
        m.payload.policy_hash = s.0.id();
        m.payload.event_id = s.2.event_id.clone();
        m.payload.nonce = s.2.nonce.clone();
        m.signature = sign(&m.payload.canonical_bytes(), &[sensor_seeds[i]; 32]);
    }
    let d = &mut s.3.decision.payload;
    d.policy_hash = s.0.id();
    d.registry_version = s.1.version;
    d.event_id = s.2.event_id.clone();
    d.nonce = s.2.nonce.clone();
    d.measurement_ids = s.3.measurements.iter().map(|m| m.payload.id()).collect();
    d.measurement_ids.sort();
    s.3.decision.signature = sign(&d.canonical_bytes(), &[6; 32]);
    for (i, b) in s.3.ballots.iter_mut().enumerate() {
        b.payload.event_id = s.2.event_id.clone();
        b.payload.nonce = s.2.nonce.clone();
        b.payload.decision_hash = d.id();
        b.signature = sign(&b.payload.canonical_bytes(), &[(i + 3) as u8; 32]);
    }
}

pub fn workload() -> Workload {
    use Outcome::*;
    let mut a = veri_adam_protocol::demo::fixture(CrewMode::NominalThree, [900, 850]);
    a.1.version = 2;
    for (id, node, seed) in [("sensor-c", "pi-a", 7), ("sensor-d", "pi-b", 8)] {
        a.1.members.push(Member {
            signer_id: id.into(),
            node_id: node.into(),
            role: Role::Sensor,
            public_key: public_key(&[seed; 32]),
            revoked: false,
        });
    }
    a.2.event_id = "multi-claim-a".into();
    resign(&mut a, [1, 2]);
    let mut b = a.clone();
    b.0.sources = vec!["sensor-c".into(), "sensor-d".into()];
    b.2.event_id = "multi-claim-b".into();
    b.2.nonce = "cd".repeat(32);
    b.3.measurements[0].payload.signer_id = "sensor-c".into();
    b.3.measurements[1].payload.signer_id = "sensor-d".into();
    resign(&mut b, [7, 8]);
    let heads: std::collections::BTreeMap<_, _> =
        a.3.measurements
            .iter()
            .chain(&b.3.measurements)
            .map(|m| (m.payload.signer_id.clone(), m.payload.id()))
            .collect();
    a.2.heads = heads.clone();
    b.2.heads = heads.clone();
    let mut variant = a.clone();
    variant.3.ballots.truncate(2);
    let mut steps = vec![
        step("claim-a-cold", &a, VerifiedCurrent),
        step("claim-a-index-admission", &a, VerifiedCurrent),
        step("claim-b-cold", &b, VerifiedCurrent),
        step("claim-b-index-admission", &b, VerifiedCurrent),
        step("claim-a-two-ballot-variant-cold", &variant, VerifiedCurrent),
        step("claim-a-variant-index-admission", &variant, VerifiedCurrent),
    ];
    for i in 0..3 {
        steps.push(step(&format!("stable-claim-b-{i}"), &b, VerifiedCurrent));
    }
    let mut renewed = a.clone();
    renewed.3.measurements[0].payload.captured_at_ms = 2_000;
    renewed.3.measurements[0].payload.sequence = 2;
    resign(&mut renewed, [1, 2]);
    let new_id = renewed.3.measurements[0].payload.id();
    a.2.heads.insert("sensor-a".into(), new_id.clone());
    b.2.heads = a.2.heads.clone();
    variant.2.heads = a.2.heads.clone();
    renewed.2.heads = a.2.heads.clone();
    steps.push(step(
        "support-a-change-check-unaffected-b",
        &b,
        VerifiedCurrent,
    ));
    steps.push(step("old-a-unconfirmed", &a, Unconfirmed));
    steps.push(step("old-a-variant-unconfirmed", &variant, Unconfirmed));
    steps.push(step("renewed-a-cold", &renewed, VerifiedCurrent));
    steps.push(step("renewed-a-index-admission", &renewed, VerifiedCurrent));
    b.2.heads.insert("unrelated-source".into(), "ee".repeat(32));
    steps.push(step(
        "unrelated-update-visits-no-claim",
        &b,
        VerifiedCurrent,
    ));
    b.2.heads.remove("sensor-a");
    steps.push(step("source-a-dropout-unaffected-b", &b, VerifiedCurrent));
    renewed.2.heads = b.2.heads.clone();
    steps.push(step("source-a-dropout-withholds-a", &renewed, Unconfirmed));
    a.2.heads = heads.clone();
    b.2.heads = heads;
    variant.2.heads = a.2.heads.clone();
    steps.push(step("trusted-old-snapshot-restored-a", &a, VerifiedCurrent));
    steps.push(step("trusted-old-snapshot-restored-b", &b, VerifiedCurrent));
    renewed.2.heads = a.2.heads.clone();
    steps.push(step(
        "renewed-a-not-current-at-old-head",
        &renewed,
        Unconfirmed,
    ));
    a.2.now_ms = 11_000;
    steps.push(step("expiry-a", &a, Expired));
    a.2.kind = CheckKind::Historical;
    steps.push(step("historical-a", &a, VerifiedHistorical));
    a.2.kind = CheckKind::Current;
    a.2.now_ms = 999;
    steps.push(step("future-a", &a, Rejected));
    let mut revoked = b.clone();
    revoked.1.members[1].revoked = true;
    // The shared registry revokes sensor-b, which is not required by B's policy.
    steps.push(step(
        "unrelated-revocation-b-remains-authorized",
        &revoked,
        VerifiedCurrent,
    ));
    revoked.1.members[6].revoked = true;
    steps.push(step(
        "required-role-revocation-rejects-b",
        &revoked,
        Rejected,
    ));
    let mut restart = step("restart-b-cold", &b, VerifiedCurrent);
    restart.reset_cache = true;
    steps.push(restart);
    steps.push(step("restart-b-index-admission", &b, VerifiedCurrent));
    Workload {
        version: VERSION,
        label: "b3-multi-claim-v1".into(),
        data_kind: "synthetic".into(),
        cache_config: Config::default(),
        steps,
    }
}
