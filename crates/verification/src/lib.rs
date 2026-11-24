//! B1: authenticate every record and independently recompute the public decision.
use serde::Serialize;
use std::collections::BTreeSet;
use veri_adam_protocol::*;

#[derive(Debug, Default, Serialize)]
pub struct Work {
    pub signature_checks: u64,
    pub measurements_evaluated: u64,
    pub ballots_evaluated: u64,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub outcome: Outcome,
    pub reason: String,
    pub decision_id: Option<String>,
    pub support_expires_at_ms: Option<u64>,
    pub work: Work,
}

pub fn check(bundle: &Bundle, policy: &Policy, registry: &Registry, request: &Request) -> Report {
    let mut report = Report {
        outcome: Outcome::Rejected,
        reason: String::new(),
        decision_id: None,
        support_expires_at_ms: None,
        work: Work::default(),
    };
    match evaluate(bundle, policy, registry, request, &mut report) {
        Ok((outcome, reason)) => {
            report.outcome = outcome;
            report.reason = reason.into();
        }
        Err(reason) => {
            report.reason = reason;
        }
    }
    report
}

fn evaluate(
    b: &Bundle,
    p: &Policy,
    reg: &Registry,
    req: &Request,
    report: &mut Report,
) -> Result<(Outcome, &'static str), String> {
    p.validate(reg)?;
    context(VERSION, &req.event_id, &req.nonce)?;
    let d = &b.decision.payload;
    d.validate()?;
    if d.event_id != req.event_id || d.nonce != req.nonce {
        return Err("unexpected_event_or_nonce".into());
    }
    if d.policy_hash != p.id()
        || d.registry_version != reg.version
        || d.coordinator_id != p.coordinator
    {
        return Err("policy_or_registry_binding_mismatch".into());
    }
    if b.measurements.len() != p.sources.len() || b.ballots.len() > p.voters.len() {
        return Err("unexpected_record_count".into());
    }
    let coordinator = reg
        .member(&d.coordinator_id, Role::Coordinator)
        .ok_or("unauthorized_coordinator")?;
    report.work.signature_checks += 1;
    verify(
        &d.canonical_bytes(),
        &b.decision.signature,
        &coordinator.public_key,
    )?;
    let mut sources = BTreeSet::new();
    let mut ids = Vec::new();
    let mut total = 0u64;
    let mut horizon = u64::MAX;
    let mut expired = false;
    let mut changed = false;
    for signed in &b.measurements {
        let m = &signed.payload;
        m.validate()?;
        if m.event_id != d.event_id || m.nonce != d.nonce || m.policy_hash != d.policy_hash {
            return Err("measurement_context_mismatch".into());
        }
        if !p.sources.contains(&m.signer_id) || !sources.insert(&m.signer_id) {
            return Err("duplicate_or_unexpected_source".into());
        }
        let member = reg
            .member(&m.signer_id, Role::Sensor)
            .ok_or("unauthorized_sensor")?;
        report.work.signature_checks += 1;
        verify(&m.canonical_bytes(), &signed.signature, &member.public_key)?;
        if m.captured_at_ms > req.now_ms {
            return Err("future_measurement".into());
        }
        let end = m
            .captured_at_ms
            .checked_add(p.ttl_ms)
            .ok_or("expiry_overflow")?;
        horizon = horizon.min(end);
        expired |= req.now_ms >= end;
        let id = m.id();
        changed |= req.heads.get(&m.signer_id) != Some(&id);
        ids.push(id);
        total += u64::from(m.co2_ppm);
        report.work.measurements_evaluated += 1;
    }
    ids.sort();
    if ids != d.measurement_ids {
        return Err("support_digest_mismatch".into());
    }
    // Compare exact integer sums, without floating point or mean-rounding errors.
    if d.advisory != (total >= u64::from(p.threshold_ppm) * p.sources.len() as u64) {
        return Err("classification_mismatch".into());
    }
    let mut voters = BTreeSet::new();
    let mut matching = 0;
    for signed in &b.ballots {
        let ballot = &signed.payload;
        ballot.validate()?;
        if ballot.event_id != d.event_id
            || ballot.nonce != d.nonce
            || ballot.decision_hash != d.id()
        {
            return Err("ballot_context_mismatch".into());
        }
        if !p.voters.contains(&ballot.voter_id) || !voters.insert(&ballot.voter_id) {
            return Err("duplicate_or_unexpected_voter".into());
        }
        let member = reg
            .member(&ballot.voter_id, Role::Voter)
            .ok_or("unauthorized_voter")?;
        report.work.signature_checks += 1;
        verify(
            &ballot.canonical_bytes(),
            &signed.signature,
            &member.public_key,
        )?;
        matching += usize::from(ballot.advisory == d.advisory);
        report.work.ballots_evaluated += 1;
    }
    report.decision_id = Some(d.id());
    report.support_expires_at_ms = Some(horizon);
    // Quorum is never lowered when a ballot is missing.
    if matching < 2 {
        return Ok((Outcome::Unconfirmed, "insufficient_matching_ballots"));
    }
    if req.kind == CheckKind::Historical {
        return Ok((
            Outcome::VerifiedHistorical,
            "historical_processing_verified",
        ));
    }
    if expired {
        return Ok((Outcome::Expired, "support_expired"));
    }
    if changed {
        return Ok((Outcome::Unconfirmed, "missing_or_replaced_trusted_head"));
    }
    Ok((Outcome::VerifiedCurrent, "current_support_verified"))
}
