//! Authenticate a resolved historical event before exposing its summary to the
//! reasoner. The registry is pinned by the receiving actor, not the database.
use crate::*;
use veri_adam_decision::Candidate;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    pub checkpoint: Signed<audit::Checkpoint>,
    pub entry: audit::Entry,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvent {
    pub record: Record,
    pub disclosure: Disclosure,
    pub trust: Trust,
    pub resolution: Resolution,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub proposal_id: String,
    pub event_id: String,
    pub resolved_at_ms: u64,
    pub data_kind: String,
    pub candidate: Candidate,
    pub event_record: serde_json::Value,
}
pub fn verify(s: &StoredEvent, pinned_registry: &str, before: u64) -> Result<View, String> {
    decode::<32>(pinned_registry)?;
    s.trust.validate()?;
    let t = &s.trust;
    let p = &s.record.proposal.payload;
    let cp = &s.resolution.checkpoint;
    let e = &s.resolution.entry;
    if t.registry.id() != pinned_registry
        || cp.payload.registry_hash != pinned_registry
        || cp.payload.coordinator_id != t.coordinator
    {
        return Err("historical registry differs from the receiving actor's trust pin".into());
    }
    veri_adam_protocol::verify(
        &canonical::bytes("audit-checkpoint", &cp.payload),
        &cp.signature,
        t.registry.key(&t.coordinator, Role::Coordinator)?,
    )?;
    let proposal_id = canonical::id("proposal", p);
    if e.sequence == 0
        || cp.payload.sequence != e.sequence
        || cp.payload.tip != canonical::id("audit-entry", e)
        || e.event.context != p.context
        || e.event.proposal_id.as_ref() != Some(&proposal_id)
        || e.event.kind != audit::Kind::Attempt
        || e.event.outcome != "verified_current_awaiting_recording"
        || e.event.at_ms >= before
    {
        return Err("historical resolution acknowledgment/context/time mismatch".into());
    }
    let q = Request {
        context: p.context.clone(),
        now_ms: e.event.at_ms,
        current: true,
        heads: s
            .record
            .observations
            .iter()
            .filter(|o| o.payload.purpose == Purpose::Fusion)
            .map(|o| {
                (
                    o.payload.source_id.clone(),
                    canonical::id("observation", &o.payload),
                )
            })
            .collect(),
    };
    if checker::check(&s.record, Some(&s.disclosure), t, &q).outcome
        != "verified_current_awaiting_recording"
    {
        return Err(
            "historical event failed independent evidence/quorum/policy verification".into(),
        );
    }
    let d = &s.disclosure;
    let stage = &d.stage;
    let primary = d
        .event
        .readings
        .iter()
        .find(|v| v.source_id == d.event.trigger_source)
        .ok_or("historical trigger absent")?;
    Ok(View {
        proposal_id,
        event_id: p.context.event_id.clone(),
        resolved_at_ms: e.event.at_ms,
        data_kind: primary.data_kind.clone(),
        candidate: stage
            .selected_recommendation
            .clone()
            .ok_or("historical recommendation absent")?,
        event_record: serde_json::json!({"trigger":primary,"fusion":stage.fusion,"selected_recommendation":stage.selected_recommendation,"local_recommendation":stage.local_recommendation,"crew_support":stage.crew_support,"final_decision":stage.final_decision,"resolved_at_ms":e.event.at_ms}),
    })
}
