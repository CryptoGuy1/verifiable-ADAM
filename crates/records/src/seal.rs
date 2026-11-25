use crate::*;
use bulletproofs::PedersenGens;
use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::SigningKey;
use rand_core::{OsRng, RngCore};
use veri_adam_protocol::sign;

/// Ephemeral local-run keys. No persistence, transport or device provisioning implied.
pub struct Keyring(pub BTreeMap<String, SigningKey>);
impl Keyring {
    pub fn local() -> Result<(Self, Registry), String> {
        let mut keys = BTreeMap::new();
        let mut members = Vec::new();
        for (id, role, node) in [
            ("sensor-a", Role::Source, "local-a"),
            ("sensor-b", Role::Source, "local-b"),
            ("sensor-role", Role::Sensor, "local-a"),
            ("aggregator-role", Role::Aggregator, "local-a"),
            ("decision-role", Role::Decision, "local-b"),
            ("coordinator", Role::Coordinator, "local-b"),
        ] {
            let mut seed = [0; 32];
            OsRng.try_fill_bytes(&mut seed).map_err(|e| e.to_string())?;
            let key = SigningKey::from_bytes(&seed);
            members.push(Member {
                id: id.into(),
                node_id: node.into(),
                role,
                public_key: hex::encode(key.verifying_key().to_bytes()),
                revoked: false,
            });
            keys.insert(id.into(), key);
        }
        Ok((
            Self(keys),
            Registry {
                version: 1,
                members,
            },
        ))
    }
    pub fn signed<T: Serialize + Clone>(
        &self,
        id: &str,
        kind: &str,
        payload: T,
    ) -> Result<Signed<T>, String> {
        let key = self.0.get(id).ok_or("missing local signing key")?;
        Ok(Signed {
            signature: sign(&canonical::bytes(kind, &payload), &key.to_bytes()),
            payload,
        })
    }
}
fn salt() -> Result<String, String> {
    let mut b = [0; 32];
    OsRng.try_fill_bytes(&mut b).map_err(|e| e.to_string())?;
    Ok(hex::encode(b))
}
pub fn fresh_nonce() -> Result<String, String> {
    salt()
}
pub fn commitment(ppm: u32, blinding: &str) -> Result<String, String> {
    if ppm > 40_000 {
        return Err("ppm outside committed domain".into());
    }
    let r = Option::<Scalar>::from(Scalar::from_canonical_bytes(decode::<32>(blinding)?))
        .ok_or("noncanonical commitment opening")?;
    Ok(hex::encode(
        PedersenGens::default()
            .commit(Scalar::from(u64::from(ppm)), r)
            .compress()
            .to_bytes(),
    ))
}
fn fresh_blinding() -> Result<String, String> {
    let mut wide = [0; 64];
    OsRng.try_fill_bytes(&mut wide).map_err(|e| e.to_string())?;
    Ok(hex::encode(
        Scalar::from_bytes_mod_order_wide(&wide).to_bytes(),
    ))
}
/// Construct evidence using only this source actor's key. Ordinals retain the
/// original event ordering; the coordinator cannot manufacture source signatures.
pub fn prepare_observations(
    event: &EventInput,
    trust: &Trust,
    keys: &Keyring,
    source_filter: Option<&str>,
) -> Result<(Vec<Signed<Observation>>, Vec<Opening>), String> {
    trust.validate()?;
    let context = Context::from(event);
    context.validate()?;
    let mut observations = Vec::new();
    let mut openings = Vec::new();
    let inputs = event
        .readings
        .iter()
        .map(|r| {
            (
                Purpose::Fusion,
                r.source_id.clone(),
                Some(r.sequence),
                r.captured_at_ms,
                r.data_kind.clone(),
                r.co2_ppm,
            )
        })
        .chain(event.temporal_window.iter().map(|r| {
            (
                Purpose::TemporalBaseline,
                r.source_id.clone(),
                None,
                r.captured_at_ms,
                "historical_baseline".into(),
                r.co2_ppm,
            )
        }));
    for (i, (purpose, source_id, sequence, captured_at_ms, data_kind, ppm)) in inputs.enumerate() {
        if source_filter.is_some_and(|id| id != source_id) {
            continue;
        }
        let blinding = fresh_blinding()?;
        let payload = Observation {
            context: context.clone(),
            source_id: source_id.clone(),
            purpose,
            ordinal: i as u32,
            sequence,
            captured_at_ms,
            data_kind,
            calibration_hash: trust.calibration_id(),
            commitment_scheme: COMMITMENT_SCHEME.into(),
            commitment: commitment(ppm, &blinding)?,
        };
        let observation_id = canonical::id("observation", &payload);
        observations.push(keys.signed(&source_id, "observation", payload)?);
        openings.push(Opening {
            observation_id,
            co2_ppm: ppm,
            blinding,
        });
    }
    Ok((observations, openings))
}
/// Assemble an unsigned proposal from evidence already signed before inference.
pub fn draft(
    event: &EventInput,
    stage: &StageRecord,
    trust: &Trust,
    observations: &[Signed<Observation>],
    openings: Vec<Opening>,
) -> Result<(Proposal, Disclosure), String> {
    trust.validate()?;
    let context = Context::from(event);
    context.validate()?;
    let fusion = stage
        .fusion
        .as_ref()
        .ok_or("stage has no fusion to attest")?;
    let selected = stage
        .selected_recommendation
        .as_ref()
        .ok_or("stage has no selected recommendation")?;
    let mut disclosure = Disclosure {
        event: event.clone(),
        stage: stage.clone(),
        openings,
        trace_salt: salt()?,
        reasoner_salt: salt()?,
    };
    // Keep the stage's truthful flags: authentication belongs to this outer protocol.
    disclosure.stage.authenticated_record = false;
    let trace_commitment = canonical::hidden(
        "trace-commitment",
        &disclosure.trace_salt,
        &(&disclosure.event, &disclosure.stage),
    );
    let selected_reasoner_commitment = canonical::hidden(
        "reasoner-commitment",
        &disclosure.reasoner_salt,
        &selected.recommendation,
    );
    let mut observation_ids: Vec<_> = observations
        .iter()
        .map(|o| canonical::id("observation", &o.payload))
        .collect();
    observation_ids.sort();
    let governance = stage.governance.as_ref();
    let proposal = Proposal {
        protocol_version: 2,
        context,
        coordinator_id: trust.coordinator.clone(),
        registry_hash: trust.registry.id(),
        policy_hash: trust.policy_id(),
        calibration_hash: trust.calibration_id(),
        observation_ids,
        weights: trust
            .calibration
            .weights
            .iter()
            .map(|w| Weight {
                source_id: w.source_id.clone(),
                units: w.weight_units,
            })
            .collect(),
        trace_commitment,
        selected_reasoner_commitment,
        eligible_roles: stage.eligible_roles.clone(),
        quorum_rule: QUORUM_RULE.into(),
        quorum: stage.quorum as u32,
        fusion_classification: fusion.threshold_class,
        selected: DecisionSummary::from(&selected.recommendation),
        final_decision: stage.final_decision.as_ref().map(DecisionSummary::from),
        degraded: stage.degraded_mode,
        governance_accepted: governance.is_some_and(|g| g.accepted),
        governance_reasons: governance.map(|g| g.reasons.clone()).unwrap_or_default(),
        stage_status: stage.status,
        completed_at_ms: stage.completed_at_ms,
        deadline_ms: stage.deadline_ms,
        proof_statement: "weighted-fusion-two-committed-co2/v1:reserved-unimplemented".into(),
    };
    Ok((proposal, disclosure))
}
/// Legacy local component helper, preserved; actual workers use prepare/draft and
/// their own isolated keys rather than asking the coordinator to sign ballots.
pub fn attest(
    event: &EventInput,
    stage: &StageRecord,
    trust: &Trust,
    keys: &Keyring,
) -> Result<(Record, Disclosure), String> {
    let (observations, openings) = prepare_observations(event, trust, keys, None)?;
    let (proposal, disclosure) = draft(event, stage, trust, &observations, openings)?;
    let proposal_id = canonical::id("proposal", &proposal);
    let ballots = stage
        .ballots
        .iter()
        .map(|b| {
            let voter_id = trust.voters.get(&b.role).ok_or("unassigned voter")?.clone();
            keys.signed(
                &voter_id,
                "ballot",
                Ballot {
                    proposal_id: proposal_id.clone(),
                    voter_id: voter_id.clone(),
                    role: b.role,
                    classification: b.classification,
                },
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((
        Record {
            proposal: keys.signed(&trust.coordinator, "proposal", proposal)?,
            observations,
            ballots,
        },
        disclosure,
    ))
}
