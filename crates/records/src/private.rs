//! Real weighted-fusion predicate proof. It proves both signed committed inputs
//! bounded 0..40000 and the exact registered weighted comparison, NOT model
//! execution, Sensor temporal reasoning, physical truth or complete governance.
use crate::*;
use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek::{ristretto::CompressedRistretto, scalar::Scalar};
use merlin::Transcript;
use std::{collections::BTreeSet, sync::OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivateProof {
    pub statement: String,
    pub proposal_id: String,
    pub proof_hex: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimReport {
    pub accepted: bool,
    pub reason: String,
    pub private_fusion_verified: bool,
    pub authenticated_crew_classification: Option<Classification>,
    pub full_decision_processing_verified: bool,
    pub action_released: bool,
}
fn generators() -> &'static BulletproofGens {
    static G: OnceLock<BulletproofGens> = OnceLock::new();
    G.get_or_init(|| BulletproofGens::new(64, 8))
}
fn transcript(r: &Record) -> Transcript {
    let mut t = Transcript::new(b"verifiable-adam/weighted-private-fusion/v1");
    t.append_message(
        b"proposal-id",
        canonical::id("proposal", &r.proposal.payload).as_bytes(),
    );
    t
}
struct Inputs {
    points: Vec<CompressedRistretto>,
    observations: Vec<Observation>,
    weights: Vec<u64>,
    total: u64,
}
/// Authenticate public inputs before either proving or checking. Currentness is
/// independent of the proof and uses the same trusted source heads and time.
fn inputs(r: &Record, t: &Trust, q: &Request) -> Result<Inputs, String> {
    t.validate()?;
    q.context.validate()?;
    let p = &r.proposal.payload;
    if p.protocol_version != 2
        || p.proof_statement != PRIVATE_STATEMENT
        || p.context != q.context
        || p.registry_hash != t.registry.id()
        || p.policy_hash != t.policy_id()
        || p.calibration_hash != t.calibration_id()
        || p.coordinator_id != t.coordinator
    {
        return Err("private statement/context/trust mismatch".into());
    }
    veri_adam_protocol::verify(
        &canonical::bytes("proposal", p),
        &r.proposal.signature,
        t.registry.key(&p.coordinator_id, Role::Coordinator)?,
    )?;
    if r.observations.len() > 4128 || r.ballots.len() > 3 {
        return Err("private record exceeds bounds".into());
    }
    let mut ids = Vec::new();
    let mut fusion = Vec::new();
    let mut seen = BTreeSet::new();
    for o in &r.observations {
        let a = &o.payload;
        veri_adam_protocol::verify(
            &canonical::bytes("observation", a),
            &o.signature,
            t.registry.key(&a.source_id, Role::Source)?,
        )?;
        let id = canonical::id("observation", a);
        ids.push(id.clone());
        if a.context != p.context
            || a.calibration_hash != p.calibration_hash
            || a.commitment_scheme != COMMITMENT_SCHEME
        {
            return Err("private support binding mismatch".into());
        }
        let point = CompressedRistretto(decode::<32>(&a.commitment)?);
        point
            .decompress()
            .ok_or("invalid signed commitment point")?;
        if a.purpose == Purpose::Fusion {
            if !seen.insert(&a.source_id)
                || a.sequence.is_none_or(|v| v == 0)
                || !["synthetic", "replay", "physical"].contains(&a.data_kind.as_str())
                || a.captured_at_ms > p.context.started_at_ms
            {
                return Err("invalid private fusion support".into());
            }
            let expiry = a
                .captured_at_ms
                .checked_add(t.policy.evidence_ttl_ms)
                .ok_or("expiry overflow")?;
            if p.completed_at_ms >= expiry
                || q.current && (q.now_ms >= expiry || q.heads.get(&a.source_id) != Some(&id))
            {
                return Err("private support expired or replaced".into());
            }
            fusion.push((a.clone(), point));
        }
    }
    ids.sort();
    if ids != p.observation_ids || !ids.windows(2).all(|w| w[0] < w[1]) || fusion.len() != 2 {
        return Err("private proof requires exactly two distinct bound fusion inputs".into());
    }
    if p.completed_at_ms < p.context.started_at_ms
        || q.now_ms < p.completed_at_ms
        || p.context
            .started_at_ms
            .checked_add(t.policy.event_budget_ms)
            != Some(p.deadline_ms)
        || p.completed_at_ms >= p.deadline_ms
    {
        return Err("private event deadline mismatch".into());
    }
    let registered: Vec<_> = t
        .calibration
        .weights
        .iter()
        .map(|w| Weight {
            source_id: w.source_id.clone(),
            units: w.weight_units,
        })
        .collect();
    if p.weights != registered {
        return Err("private registered weight mismatch".into());
    }
    fusion.sort_by(|a, b| a.0.source_id.cmp(&b.0.source_id));
    let mut weights = Vec::new();
    for (a, _) in &fusion {
        weights.push(u64::from(
            t.calibration
                .weights
                .iter()
                .find(|w| w.source_id == a.source_id)
                .ok_or("unregistered proof source")?
                .weight_units,
        ));
    }
    let total = weights.iter().sum();
    Ok(Inputs {
        points: fusion.iter().map(|(_, c)| *c).collect(),
        observations: fusion.into_iter().map(|(a, _)| a).collect(),
        weights,
        total,
    })
}
fn derived(i: &Inputs, p: &Proposal, t: &Trust) -> Vec<CompressedRistretto> {
    let g = PedersenGens::default();
    let max = Scalar::from(40000u64) * g.B;
    let a = i.points[0].decompress().expect("validated point");
    let b = i.points[1].decompress().expect("validated point");
    let n = Scalar::from(i.weights[0]) * a + Scalar::from(i.weights[1]) * b;
    let threshold = u64::from(t.policy.threshold_ppm) * i.total;
    let difference = if p.fusion_classification == Classification::Anomaly {
        n - Scalar::from(threshold) * g.B
    } else {
        Scalar::from(threshold - 1) * g.B - n
    };
    vec![
        a.compress(),
        (max - a).compress(),
        b.compress(),
        (max - b).compress(),
        difference.compress(),
        (Scalar::from(40000 * i.total) * g.B - difference).compress(),
        g.commit(Scalar::ZERO, Scalar::ZERO).compress(),
        g.commit(Scalar::ZERO, Scalar::ZERO).compress(),
    ]
}
pub fn prove(r: &Record, d: &Disclosure, t: &Trust, q: &Request) -> Result<PrivateProof, String> {
    let i = inputs(r, t, q)?;
    let mut values = Vec::new();
    let mut blindings = Vec::new();
    let mut n = 0u64;
    let mut n_blinding = Scalar::ZERO;
    for (j, a) in i.observations.iter().enumerate() {
        let id = canonical::id("observation", a);
        let o = d
            .openings
            .iter()
            .find(|o| o.observation_id == id)
            .ok_or("private witness missing")?;
        if o.co2_ppm > 40000 || crate::commitment(o.co2_ppm, &o.blinding)? != a.commitment {
            return Err("private witness does not open signed input".into());
        }
        let blind =
            Option::<Scalar>::from(Scalar::from_canonical_bytes(decode::<32>(&o.blinding)?))
                .ok_or("noncanonical witness blinding")?;
        values.extend([u64::from(o.co2_ppm), 40000 - u64::from(o.co2_ppm)]);
        blindings.extend([blind, -blind]);
        n += i.weights[j] * u64::from(o.co2_ppm);
        n_blinding += Scalar::from(i.weights[j]) * blind;
    }
    let threshold = u64::from(t.policy.threshold_ppm) * i.total;
    let (diff, blind) = if r.proposal.payload.fusion_classification == Classification::Anomaly {
        (
            n.checked_sub(threshold)
                .ok_or("claimed exceedance is false")?,
            n_blinding,
        )
    } else {
        (
            (threshold - 1)
                .checked_sub(n)
                .ok_or("claimed below-threshold class is false")?,
            -n_blinding,
        )
    };
    let upper = 40000 * i.total;
    values.extend([
        diff,
        upper
            .checked_sub(diff)
            .ok_or("weighted difference outside domain")?,
        0,
        0,
    ]);
    blindings.extend([blind, -blind, Scalar::ZERO, Scalar::ZERO]);
    let (proof, commitments) = RangeProof::prove_multiple(
        generators(),
        &PedersenGens::default(),
        &mut transcript(r),
        &values,
        &blindings,
        64,
    )
    .map_err(|e| e.to_string())?;
    if commitments != derived(&i, &r.proposal.payload, t) {
        return Err("proof did not bind derived signed commitments".into());
    }
    Ok(PrivateProof {
        statement: PRIVATE_STATEMENT.into(),
        proposal_id: canonical::id("proposal", &r.proposal.payload),
        proof_hex: hex::encode(proof.to_bytes()),
    })
}
pub fn check(r: &Record, proof: &PrivateProof, t: &Trust, q: &Request) -> ClaimReport {
    match check_inner(r,proof,t,q) {
        Ok(crew)=>ClaimReport{accepted:true,reason:"bounded weighted fusion proved from two authenticated commitments; crew ballots authenticated; temporal/model/full-governance processing is not privately proved".into(),private_fusion_verified:true,authenticated_crew_classification:crew,full_decision_processing_verified:false,action_released:false},
        Err(reason)=>ClaimReport{accepted:false,reason,private_fusion_verified:false,authenticated_crew_classification:None,full_decision_processing_verified:false,action_released:false}
    }
}
fn check_inner(
    r: &Record,
    proof: &PrivateProof,
    t: &Trust,
    q: &Request,
) -> Result<Option<Classification>, String> {
    let i = inputs(r, t, q)?;
    let p = &r.proposal.payload;
    let id = canonical::id("proposal", p);
    if proof.statement != PRIVATE_STATEMENT
        || proof.proposal_id != id
        || proof.proof_hex.len() > 4096
    {
        return Err("proof statement/event substitution or size mismatch".into());
    }
    let bytes = hex::decode(&proof.proof_hex).map_err(|_| "malformed proof bytes")?;
    RangeProof::from_bytes(&bytes)
        .map_err(|e| e.to_string())?
        .verify_multiple(
            generators(),
            &PedersenGens::default(),
            &mut transcript(r),
            &derived(&i, p, t),
            64,
        )
        .map_err(|e| e.to_string())?;
    let eligible = &p.eligible_roles;
    if ![
        vec![
            VotingRole::Sensor,
            VotingRole::Aggregator,
            VotingRole::Decision,
        ],
        vec![VotingRole::Sensor, VotingRole::Decision],
    ]
    .contains(eligible)
        || p.quorum_rule != QUORUM_RULE
        || p.quorum as usize != eligible.len() / 2 + 1
    {
        return Err("private crew formation/quorum mismatch".into());
    }
    let mut seen = BTreeSet::new();
    let mut yes = 0usize;
    for b in &r.ballots {
        let a = &b.payload;
        if a.proposal_id != id
            || !eligible.contains(&a.role)
            || !seen.insert(a.role)
            || t.voters.get(&a.role) != Some(&a.voter_id)
        {
            return Err("private ballot binding mismatch".into());
        }
        veri_adam_protocol::verify(
            &canonical::bytes("ballot", a),
            &b.signature,
            t.registry.key(&a.voter_id, a.role.into())?,
        )?;
        yes += usize::from(a.classification == Classification::Anomaly);
    }
    let no = r.ballots.len() - yes;
    let needed = eligible.len() / 2 + 1;
    let crew = if yes >= needed {
        Some(Classification::Anomaly)
    } else if no >= needed {
        Some(Classification::Normal)
    } else {
        None
    };
    if p.final_decision.as_ref().map(|v| v.classification) != crew {
        return Err("private final tally mismatch".into());
    }
    Ok(crew)
}
