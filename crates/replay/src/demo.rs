//! A declared development corpus, not a statistical performance experiment.
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
fn renew(s: &mut Snapshot, indexes: &[usize], at: u64) {
    for &i in indexes {
        let m = &mut s.3.measurements[i];
        m.payload.sequence += 1;
        m.payload.captured_at_ms = at;
        m.signature = sign(&m.payload.canonical_bytes(), &[(i + 1) as u8; 32]);
        s.2.heads
            .insert(m.payload.signer_id.clone(), m.payload.id());
    }
    s.3.decision.payload.measurement_ids =
        s.3.measurements.iter().map(|m| m.payload.id()).collect();
    s.3.decision.payload.measurement_ids.sort();
    s.3.decision.signature = sign(&s.3.decision.payload.canonical_bytes(), &[6; 32]);
    for (i, v) in s.3.ballots.iter_mut().enumerate() {
        v.payload.decision_hash = s.3.decision.payload.id();
        v.signature = sign(&v.payload.canonical_bytes(), &[(i + 3) as u8; 32]);
    }
}

pub fn workload() -> Workload {
    use Outcome::*;
    let base = veri_adam_protocol::demo::fixture(CrewMode::NominalThree, [900, 850]);
    let mut steps = vec![
        step("cold-current", &base, VerifiedCurrent),
        step("stable-repeat", &base, VerifiedCurrent),
    ];
    let mut s = base.clone();
    s.2.heads.insert("unrelated-source".into(), "00".repeat(32));
    steps.push(step("unrelated-update", &s, VerifiedCurrent));
    s.2.now_ms = 10_999;
    steps.push(step("before-expiry", &s, VerifiedCurrent));
    s.2.now_ms = 11_000;
    steps.push(step("exact-expiry", &s, Expired));
    s.2.kind = CheckKind::Historical;
    steps.push(step("historical-after-expiry", &s, VerifiedHistorical));
    s.2.now_ms = 999;
    steps.push(step("time-before-capture", &s, Rejected));
    steps.push(step("restore-valid-check-time", &base, VerifiedCurrent));
    s = base.clone();
    s.2.heads.remove("sensor-a");
    steps.push(step("missing-head", &s, Unconfirmed));
    s.2.heads.insert("sensor-a".into(), "11".repeat(32));
    steps.push(step("replaced-head-old-bundle", &s, Unconfirmed));
    s = base.clone();
    renew(&mut s, &[0], 2_000);
    steps.push(step("authenticated-replacement", &s, VerifiedCurrent));
    steps.push(step("replacement-repeat", &s, VerifiedCurrent));
    let mut old = base.clone();
    old.2.heads = s.2.heads.clone();
    steps.push(step("old-bundle-after-replacement", &old, Unconfirmed));
    s.2.now_ms = 11_000;
    steps.push(step("remaining-source-expires", &s, Expired));
    renew(&mut s, &[0, 1], 12_000);
    s.2.now_ms = 12_500;
    steps.push(step("all-sources-renewed", &s, VerifiedCurrent));
    let mut tampered = s.clone();
    tampered.3.measurements[0].payload.co2_ppm += 1;
    steps.push(step("tamper-after-cache-warmup", &tampered, Rejected));
    steps.push(step(
        "tamper-repeat-no-negative-signature-cache",
        &tampered,
        Rejected,
    ));
    let mut unknown = s.clone();
    unknown.3.measurements[0].payload.signer_id = "unknown".into();
    steps.push(step("unauthorized-source", &unknown, Rejected));
    let mut changed = base.clone();
    changed.0.threshold_ppm = 1_000;
    steps.push(step("policy-content-change", &changed, Rejected));
    changed = base.clone();
    changed.1.members[0].revoked = true;
    steps.push(step("role-revocation", &changed, Rejected));
    changed = base.clone();
    changed.1.members[0].public_key = public_key(&[9; 32]);
    steps.push(step("key-change-with-same-version", &changed, Rejected));
    steps.push(step(
        "restore-trusted-fixture-snapshot",
        &base,
        VerifiedCurrent,
    ));
    changed = base.clone();
    changed.2.event_id = "different-event".into();
    steps.push(step("event-replay", &changed, Rejected));
    changed = base.clone();
    changed.2.nonce = "12".repeat(32);
    steps.push(step("nonce-replay", &changed, Rejected));
    changed = base.clone();
    changed.3.ballots.truncate(1);
    steps.push(step("insufficient-quorum", &changed, Unconfirmed));
    steps.push(step("insufficient-quorum-repeat", &changed, Unconfirmed));
    changed = base.clone();
    changed.3.ballots[1] = changed.3.ballots[0].clone();
    steps.push(step("duplicate-ballot", &changed, Rejected));
    let mut restart = step("cache-restart-cold-check", &base, VerifiedCurrent);
    restart.reset_cache = true;
    steps.push(restart);
    changed = base.clone();
    changed.2.heads.remove("sensor-b");
    steps.push(step(
        "dropout-without-quorum-reduction",
        &changed,
        Unconfirmed,
    ));
    let mut degraded = veri_adam_protocol::demo::fixture(CrewMode::DegradedTwo, [799, 800]);
    steps.push(step(
        "explicit-degraded-no-advisory",
        &degraded,
        VerifiedCurrent,
    ));
    degraded.3.ballots.truncate(1);
    steps.push(step("degraded-missing-voter", &degraded, Unconfirmed));
    Workload {
        version: VERSION,
        label: "b2-currentness-v1".into(),
        data_kind: "synthetic".into(),
        cache_config: Config::default(),
        steps,
    }
}
