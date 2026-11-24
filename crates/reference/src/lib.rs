//! Independent, deliberately straightforward acceptance oracle.
//! Shares schemas, encodings, and crypto primitives, not production decision logic.
use std::collections::BTreeSet;
use veri_adam_protocol::*;

pub fn check(b: &Bundle, p: &Policy, r: &Registry, q: &Request) -> Outcome {
    // Structural validation is intentionally repeated instead of calling Policy::validate.
    if r.version == 0 || r.members.is_empty() || r.members.len() > MAX_RECORDS {
        return Outcome::Rejected;
    }
    let mut registry_ids = BTreeSet::new();
    let mut registry_keys = BTreeSet::new();
    for m in &r.members {
        if identifier(&m.signer_id).is_err()
            || identifier(&m.node_id).is_err()
            || validate_public_key(&m.public_key).is_err()
            || !registry_ids.insert(&m.signer_id)
            || !registry_keys.insert(&m.public_key)
        {
            return Outcome::Rejected;
        }
    }
    if p.protocol_version != VERSION
        || p.policy_version == 0
        || p.ttl_ms == 0
        || p.threshold_ppm > 40_000
        || p.sources.is_empty()
        || p.sources.len() > MAX_RECORDS
    {
        return Outcome::Rejected;
    }
    let crew_size = match p.crew_mode {
        CrewMode::NominalThree => 3,
        CrewMode::DegradedTwo => 2,
    };
    if p.voters.len() != crew_size {
        return Outcome::Rejected;
    }
    for (list, role) in [(&p.sources, Role::Sensor), (&p.voters, Role::Voter)] {
        let unique: BTreeSet<_> = list.iter().collect();
        if unique.len() != list.len() {
            return Outcome::Rejected;
        }
        if list.iter().any(|id| {
            identifier(id).is_err()
                || !r
                    .members
                    .iter()
                    .any(|m| m.signer_id == *id && m.role == role && !m.revoked)
        }) {
            return Outcome::Rejected;
        }
    }
    let d = &b.decision.payload;
    if context(VERSION, &q.event_id, &q.nonce).is_err()
        || d.validate().is_err()
        || d.event_id != q.event_id
        || d.nonce != q.nonce
        || d.policy_hash != p.id()
        || d.registry_version != r.version
        || d.coordinator_id != p.coordinator
        || identifier(&p.coordinator).is_err()
    {
        return Outcome::Rejected;
    }
    let coordinator = r
        .members
        .iter()
        .find(|m| m.signer_id == p.coordinator && m.role == Role::Coordinator && !m.revoked);
    if coordinator
        .is_none_or(|m| verify(&d.canonical_bytes(), &b.decision.signature, &m.public_key).is_err())
    {
        return Outcome::Rejected;
    }
    if b.measurements.len() != p.sources.len() || b.ballots.len() > p.voters.len() {
        return Outcome::Rejected;
    }
    let mut seen = Vec::new();
    let mut actual_ids = Vec::new();
    let mut readings = Vec::new();
    let mut all_fresh = true;
    let mut all_heads = true;
    for sm in &b.measurements {
        let m = &sm.payload;
        if m.validate().is_err()
            || m.event_id != q.event_id
            || m.nonce != q.nonce
            || m.policy_hash != p.id()
            || !p.sources.contains(&m.signer_id)
            || seen.contains(&m.signer_id)
            || m.captured_at_ms > q.now_ms
        {
            return Outcome::Rejected;
        }
        let key = r
            .members
            .iter()
            .find(|x| x.signer_id == m.signer_id && x.role == Role::Sensor && !x.revoked);
        if key.is_none_or(|k| verify(&m.canonical_bytes(), &sm.signature, &k.public_key).is_err()) {
            return Outcome::Rejected;
        }
        let Some(expiry) = m.captured_at_ms.checked_add(p.ttl_ms) else {
            return Outcome::Rejected;
        };
        if q.now_ms >= expiry {
            all_fresh = false;
        }
        if q.heads.get(&m.signer_id) != Some(&m.id()) {
            all_heads = false;
        }
        readings.push(u64::from(m.co2_ppm));
        seen.push(m.signer_id.clone());
        actual_ids.push(m.id());
    }
    actual_ids.sort();
    if actual_ids != d.measurement_ids {
        return Outcome::Rejected;
    }
    let expected =
        readings.iter().sum::<u64>() >= u64::from(p.threshold_ppm) * readings.len() as u64;
    if expected != d.advisory {
        return Outcome::Rejected;
    }
    let mut ballot_ids = BTreeSet::new();
    let mut opinions = Vec::new();
    for sb in &b.ballots {
        let v = &sb.payload;
        if v.validate().is_err()
            || v.event_id != q.event_id
            || v.nonce != q.nonce
            || v.decision_hash != d.id()
            || !p.voters.contains(&v.voter_id)
            || !ballot_ids.insert(&v.voter_id)
        {
            return Outcome::Rejected;
        }
        let key = r
            .members
            .iter()
            .find(|m| m.signer_id == v.voter_id && m.role == Role::Voter && !m.revoked);
        if key.is_none_or(|k| verify(&v.canonical_bytes(), &sb.signature, &k.public_key).is_err()) {
            return Outcome::Rejected;
        }
        opinions.push(v.advisory);
    }
    if opinions.iter().filter(|v| **v == expected).count() < 2 {
        return Outcome::Unconfirmed;
    }
    match q.kind {
        CheckKind::Historical => Outcome::VerifiedHistorical,
        CheckKind::Current if !all_fresh => Outcome::Expired,
        CheckKind::Current if !all_heads => Outcome::Unconfirmed,
        CheckKind::Current => Outcome::VerifiedCurrent,
    }
}
