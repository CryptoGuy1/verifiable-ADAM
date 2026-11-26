use std::collections::BTreeMap;
use veri_adam_decision::{calibration::calibrate, fixtures, reasoner::rule_recommendation, *};
use veri_adam_records::{canonical, checker, *};

struct FixedClock(u64);
impl Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.0
    }
}
struct Chosen(Classification);
impl Reasoner for Chosen {
    fn infer(
        &mut self,
        c: &ReasoningContext,
        _: Option<&str>,
        _: u64,
    ) -> Result<ReasonerResponse, String> {
        let mut r = rule_recommendation(c, 0.8);
        r.classification = self.0;
        let a = c.application_policy.alignment(self.0);
        r.severity = a.severity;
        r.recommended_action = a.action.clone();
        Ok(ReasonerResponse {
            backend: "test-response-not-inference".into(),
            raw_response: serde_json::to_string(&r).unwrap(),
            metadata: serde_json::json!({}),
        })
    }
}
struct Case {
    keys: Keyring,
    trust: Trust,
    record: Record,
    disclosure: Disclosure,
    request: Request,
}
fn fixture(
    a: u32,
    b: u32,
    choice: Classification,
    spike: bool,
    degraded: bool,
    missing: bool,
) -> Case {
    let (keys, registry) = Keyring::local().unwrap();
    let trust = Trust {
        registry,
        policy: fixtures::policy(),
        calibration: calibrate(&fixtures::calibration_data()).unwrap(),
        voters: BTreeMap::from([
            (VotingRole::Sensor, "sensor-role".into()),
            (VotingRole::Aggregator, "aggregator-role".into()),
            (VotingRole::Decision, "decision-role".into()),
        ]),
        coordinator: "coordinator".into(),
    };
    let mut event = fixtures::event("event-1", 10000, &[("sensor-a", a), ("sensor-b", b)]);
    if spike {
        event.temporal_window.push(PastReading {
            evidence_id: "prior-1".into(),
            source_id: "sensor-a".into(),
            captured_at_ms: 5000,
            co2_ppm: 300,
        });
    }
    if degraded {
        event.availability.aggregator = false;
    }
    if missing {
        event.missing_ballots.insert(VotingRole::Decision);
    }
    let stage = DecisionEngine::new()
        .evaluate(
            &event,
            &trust.policy,
            &trust.calibration,
            &mut Chosen(choice),
            &FixedClock(10000),
        )
        .unwrap();
    let (record, disclosure) = attest(&event, &stage, &trust, &keys).unwrap();
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
        context: Context::from(&event),
        now_ms: 10000,
        current: true,
        heads,
    };
    Case {
        keys,
        trust,
        record,
        disclosure,
        request,
    }
}
fn report(c: &Case) -> checker::Report {
    checker::check(&c.record, Some(&c.disclosure), &c.trust, &c.request)
}
fn verified(c: &Case) {
    let r = report(c);
    assert!(
        r.outcome.starts_with("verified_"),
        "{}: {}",
        r.outcome,
        r.reason
    );
    assert!(!r.private_proof_verified && !r.action_released);
}
fn resigned(c: &mut Case) {
    let id = canonical::id("proposal", &c.record.proposal.payload);
    c.record.proposal = c
        .keys
        .signed("coordinator", "proposal", c.record.proposal.payload.clone())
        .unwrap();
    for b in &mut c.record.ballots {
        b.payload.proposal_id = id.clone();
        *b = c
            .keys
            .signed(&b.payload.voter_id, "ballot", b.payload.clone())
            .unwrap();
    }
}
fn repack(c: &mut Case) {
    let (record, disclosure) =
        attest(&c.disclosure.event, &c.disclosure.stage, &c.trust, &c.keys).unwrap();
    c.request.heads = record
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
    c.record = record;
    c.disclosure = disclosure;
}

#[test]
fn v2_b2_b3_match_independent_outcomes_and_ignore_unrelated_updates() {
    let c = fixture(900, 500, Classification::Anomaly, false, false, false);
    let mut b2 = reuse::OrdinaryCache::new(4);
    let mut b3 = reuse::Dependencies::new(4);
    b3.update(&c.trust, &c.request.heads, 10000);
    assert_eq!(
        b2.check(&c.record, &c.disclosure, &c.trust, &c.request)
            .outcome,
        report(&c).outcome
    );
    assert_eq!(
        b3.check(&c.record, &c.disclosure, &c.trust, &c.request)
            .outcome,
        report(&c).outcome
    );
    let mut q = c.request.clone();
    q.heads.insert("unrelated-light".into(), "ff".repeat(32));
    assert!(b3.update(&c.trust, &q.heads, 10001).is_empty());
    assert_eq!(
        b2.check(&c.record, &c.disclosure, &c.trust, &q)
            .signatures_verified,
        0
    );
    assert_eq!(
        b3.check(&c.record, &c.disclosure, &c.trust, &q)
            .signatures_verified,
        0
    );
    q.heads.insert("sensor-a".into(), "aa".repeat(32));
    let affected = b3.update(&c.trust, &q.heads, 10002);
    assert_eq!(affected.len(), 1);
    assert_eq!(b3.active_claims(), 0);
    for (time, current) in [(10002, true), (40000, true), (40000, false)] {
        q.now_ms = time;
        q.current = current;
        let independent = checker::check(&c.record, Some(&c.disclosure), &c.trust, &q);
        assert_eq!(
            b2.check(&c.record, &c.disclosure, &c.trust, &q).outcome,
            independent.outcome
        );
        assert_eq!(
            b3.check(&c.record, &c.disclosure, &c.trust, &q).outcome,
            independent.outcome
        );
    }
    let mut bad = c.record.clone();
    bad.ballots[0].signature = "00".repeat(64);
    assert_eq!(
        b2.check(&bad, &c.disclosure, &c.trust, &c.request).outcome,
        "rejected"
    );
    assert_eq!(
        b3.check(&bad, &c.disclosure, &c.trust, &c.request).outcome,
        "rejected"
    );
}

#[test]
fn v2_dependency_expiry_and_configuration_changes_withdraw_registered_claims() {
    let c = fixture(900, 500, Classification::Anomaly, false, false, false);
    let mut b3 = reuse::Dependencies::new(2);
    b3.update(&c.trust, &c.request.heads, 10000);
    b3.check(&c.record, &c.disclosure, &c.trust, &c.request);
    assert_eq!(b3.update(&c.trust, &c.request.heads, 40000).len(), 1);
    b3.check(&c.record, &c.disclosure, &c.trust, &c.request);
    let mut t = c.trust.clone();
    t.policy.version += 1;
    assert_eq!(b3.update(&t, &c.request.heads, 10001).len(), 1);
    assert_eq!(
        b3.check(&c.record, &c.disclosure, &t, &c.request).outcome,
        "rejected"
    );
}

fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "adam-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
#[test]
fn audit_acknowledgment_precedes_at_most_once_release_and_survives_reopen() {
    let c = fixture(900, 500, Classification::Anomaly, false, false, false);
    let path = scratch("audit");
    let mut journal = audit::Journal::open(&path, &c.trust, &c.keys, None).unwrap();
    journal
        .append(audit::Event {
            kind: audit::Kind::Attempt,
            context: c.request.context.clone(),
            proposal_id: Some(canonical::id("proposal", &c.record.proposal.payload)),
            outcome: "verified_current_awaiting_recording".into(),
            at_ms: 10000,
        })
        .unwrap();
    let release = journal
        .authorize(&c.record, &c.disclosure, &c.request, &FixedClock(10001))
        .unwrap();
    assert_eq!(release.payload.checkpoint.payload.sequence, 2);
    assert!(
        journal
            .authorize(&c.record, &c.disclosure, &c.request, &FixedClock(10002))
            .is_err()
    );
    let pin = journal.checkpoint.clone();
    drop(journal);
    audit::verify(&path, &c.trust, Some(&pin)).unwrap();
    audit::verify_release(&release, &c.record, &c.trust, &path, Some(&pin)).unwrap();
    let mut journal = audit::Journal::open(&path, &c.trust, &c.keys, Some(&pin)).unwrap();
    assert!(
        journal
            .authorize(&c.record, &c.disclosure, &c.request, &FixedClock(10003))
            .is_err()
    );
    drop(journal);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn historical_context_requires_pinned_keys_valid_processing_and_prior_durable_resolution() {
    let c = fixture(900, 500, Classification::Anomaly, false, false, false);
    let path = scratch("history-authentication");
    let mut journal = audit::Journal::open(&path, &c.trust, &c.keys, None).unwrap();
    let previous = journal.checkpoint.payload.tip.clone();
    let event = audit::Event {
        kind: audit::Kind::Attempt,
        context: c.request.context.clone(),
        proposal_id: Some(canonical::id("proposal", &c.record.proposal.payload)),
        outcome: "verified_current_awaiting_recording".into(),
        at_ms: 10001,
    };
    let checkpoint = journal.append(event.clone()).unwrap();
    let stored = history::StoredEvent {
        record: c.record.clone(),
        disclosure: c.disclosure.clone(),
        trust: c.trust.clone(),
        resolution: history::Resolution {
            entry: audit::Entry {
                sequence: checkpoint.payload.sequence,
                previous,
                event,
            },
            checkpoint,
        },
    };
    let pin = c.trust.registry.id();
    let view = history::verify(&stored, &pin, 10002).unwrap();
    assert_eq!(view.event_id, "event-1");
    assert_eq!(view.event_record["trigger"]["co2_ppm"], 900);
    assert!(history::verify(&stored, &pin, 10001).is_err()); // Exactly at resolution is not prior.
    assert!(history::verify(&stored, &"00".repeat(32), 10002).is_err());
    let mut forged = stored.clone();
    forged
        .disclosure
        .stage
        .selected_recommendation
        .as_mut()
        .unwrap()
        .recommendation
        .reasoning = "forged memory".into();
    assert!(history::verify(&forged, &pin, 10002).is_err());
    let mut forged = stored;
    forged.resolution.entry.event.at_ms = 10000;
    assert!(history::verify(&forged, &pin, 10002).is_err());
    drop(journal);
    std::fs::remove_dir_all(path).unwrap();
}
#[test]
fn audit_detects_corruption_truncation_and_rollback_relative_to_external_pin() {
    let c = fixture(900, 500, Classification::Anomaly, false, false, false);
    let path = scratch("audit-attacks");
    let mut journal = audit::Journal::open(&path, &c.trust, &c.keys, None).unwrap();
    let old = journal.checkpoint.clone();
    journal
        .authorize(&c.record, &c.disclosure, &c.request, &FixedClock(10001))
        .unwrap();
    let latest = journal.checkpoint.clone();
    drop(journal);
    let original = std::fs::read(path.join("events.jsonl")).unwrap();
    std::fs::write(path.join("events.jsonl"), b"").unwrap();
    assert!(audit::verify(&path, &c.trust, None).is_err());
    std::fs::write(
        path.join("checkpoint.json"),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    assert!(audit::verify(&path, &c.trust, Some(&latest)).is_err());
    assert!(audit::verify(&path, &c.trust, None).is_ok()); // Whole-state rollback requires an external pin.
    std::fs::write(path.join("events.jsonl"), original).unwrap();
    std::fs::write(
        path.join("checkpoint.json"),
        serde_json::to_vec(&latest).unwrap(),
    )
    .unwrap();
    let mut entry: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path.join("events.jsonl")).unwrap()).unwrap();
    entry["payload"]["event"]["outcome"] = "forged".into();
    std::fs::write(path.join("events.jsonl"), format!("{}\n", entry)).unwrap();
    assert!(audit::verify(&path, &c.trust, Some(&latest)).is_err());
    std::fs::remove_dir_all(path).unwrap();
}
#[test]
fn late_recording_withholds_release_and_complete_signed_crash_tail_recovers_safely() {
    let c = fixture(900, 500, Classification::Anomaly, false, false, false);
    let path = scratch("audit-crash");
    let mut journal = audit::Journal::open(&path, &c.trust, &c.keys, None).unwrap();
    let before = journal.checkpoint.clone();
    struct LateAck(std::cell::Cell<bool>);
    impl Clock for LateAck {
        fn now_ms(&self) -> u64 {
            if self.0.replace(true) { 40000 } else { 10001 }
        }
    }
    assert!(
        journal
            .authorize(
                &c.record,
                &c.disclosure,
                &c.request,
                &LateAck(std::cell::Cell::new(false))
            )
            .is_err()
    );
    assert_eq!(journal.checkpoint.payload.sequence, 1); // Durable reservation, no release.
    drop(journal);
    // Simulate power loss after the signed entry fsync but before checkpoint replacement.
    std::fs::write(
        path.join("checkpoint.json"),
        serde_json::to_vec(&before).unwrap(),
    )
    .unwrap();
    assert!(audit::Journal::open(&path, &c.trust, &c.keys, Some(&before)).is_err());
    audit::Journal::recover(&path, &c.trust, &c.keys, Some(&before)).unwrap();
    let mut journal = audit::Journal::open(&path, &c.trust, &c.keys, Some(&before)).unwrap();
    assert!(
        journal
            .authorize(&c.record, &c.disclosure, &c.request, &FixedClock(10002))
            .is_err()
    );
    drop(journal);
    std::fs::remove_dir_all(path).unwrap();
}

fn reevaluate(c: &mut Case, reasoner: &mut dyn Reasoner) {
    c.disclosure.stage = DecisionEngine::new()
        .evaluate(
            &c.disclosure.event,
            &c.trust.policy,
            &c.trust.calibration,
            reasoner,
            &FixedClock(10000),
        )
        .unwrap();
    repack(c);
}
struct FailureOrRepair {
    repair: bool,
}
impl Reasoner for FailureOrRepair {
    fn infer(
        &mut self,
        c: &ReasoningContext,
        prior: Option<&str>,
        budget: u64,
    ) -> Result<ReasonerResponse, String> {
        if !self.repair {
            return Err("injected inference failure".into());
        }
        if prior.is_none() {
            return Ok(ReasonerResponse {
                backend: "malformed-test-response".into(),
                raw_response: "{".into(),
                metadata: serde_json::json!({}),
            });
        }
        Chosen(Classification::Anomaly).infer(c, prior, budget)
    }
}
#[test]
fn repair_and_deterministic_fallback_are_independently_checked() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    reevaluate(&mut c, &mut FailureOrRepair { repair: true });
    verified(&c);
    assert_eq!(c.disclosure.stage.attempts.len(), 2);
    assert!(!c.record.proposal.payload.degraded);
    reevaluate(&mut c, &mut FailureOrRepair { repair: false });
    verified(&c);
    assert!(c.record.proposal.payload.degraded);
    assert_eq!(c.record.proposal.payload.selected.confidence, 0.5);
    c.disclosure
        .stage
        .local_recommendation
        .as_mut()
        .unwrap()
        .recommendation
        .confidence = 0.9;
    repack(&mut c);
    assert!(report(&c).reason.contains("fallback"));
}
#[test]
fn competing_recommendation_severity_and_provenance_are_checked() {
    let mut c = fixture(790, 830, Classification::Normal, false, false, false);
    let mut competitor = c.disclosure.stage.local_recommendation.clone().unwrap();
    competitor.source_event_id = "competing-event".into();
    competitor.recommendation.classification = Classification::Anomaly;
    competitor.recommendation.severity = Severity::High;
    competitor.recommendation.recommended_action = "VentilationAdvisory".into();
    c.disclosure.event.competing.push(competitor);
    reevaluate(&mut c, &mut Chosen(Classification::Normal));
    verified(&c);
    // The two numerical roles resolve NORMAL; selected reasoner provenance stays ANOMALY.
    assert_eq!(
        c.record.proposal.payload.selected.classification,
        Classification::Anomaly
    );
    assert_eq!(
        c.record
            .proposal
            .payload
            .final_decision
            .as_ref()
            .unwrap()
            .classification,
        Classification::Normal
    );
    c.disclosure.stage.selected_recommendation = c.disclosure.stage.local_recommendation.clone();
    repack(&mut c);
    assert!(report(&c).reason.contains("selection"));
}

#[test]
fn signed_invented_reasoner_output_and_undisclosed_history_are_not_trusted() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.disclosure
        .stage
        .local_recommendation
        .as_mut()
        .unwrap()
        .recommendation
        .confidence = 0.99;
    repack(&mut c);
    assert!(report(&c).reason.contains("recorded reasoner response"));
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.disclosure
        .stage
        .sensor_assessment
        .as_mut()
        .unwrap()
        .baseline_evidence_ids
        .push("hidden-history".into());
    repack(&mut c);
    assert_eq!(report(&c).outcome, "unconfirmed");
}

#[test]
fn decision_at_deadline_is_rejected_even_with_authentic_signatures() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.disclosure.stage.completed_at_ms = 40000;
    c.request.now_ms = 40000;
    repack(&mut c);
    assert!(report(&c).reason.contains("deadline"));
}

#[test]
fn signed_policy_success_lie_is_rejected() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.record.proposal.payload.governance_accepted = false;
    resigned(&mut c);
    assert!(report(&c).reason.contains("governance verdict"));
}
#[test]
fn signed_weighted_record_verifies_all_six_signatures() {
    let c = fixture(790, 830, Classification::Normal, false, false, false);
    verified(&c);
    assert_eq!(report(&c).signatures_verified, 6);
    assert_eq!(
        report(&c).fusion_classification,
        Some(Classification::Normal)
    ); // Equal weighting would cross 800.
}
#[test]
fn authentic_majority_anomaly_can_disagree_with_fusion() {
    let c = fixture(700, 400, Classification::Anomaly, true, false, false);
    verified(&c);
    let r = report(&c);
    assert_eq!(r.fusion_classification, Some(Classification::Normal));
    assert_eq!(r.crew_classification, Some(Classification::Anomaly));
}
#[test]
fn authentic_majority_normal_can_disagree_with_fusion() {
    let c = fixture(700, 1300, Classification::Normal, false, false, false);
    verified(&c);
    let r = report(&c);
    assert_eq!(r.fusion_classification, Some(Classification::Anomaly));
    assert_eq!(r.crew_classification, Some(Classification::Normal));
}
#[test]
fn degraded_requires_both_formed_voters() {
    verified(&fixture(
        900,
        850,
        Classification::Anomaly,
        false,
        true,
        false,
    ));
    let c = fixture(900, 850, Classification::Anomaly, false, true, true);
    assert_eq!(report(&c).outcome, "unconfirmed");
    assert!(report(&c).reason.contains("quorum"));
}
#[test]
fn ordinary_missing_ballot_does_not_shrink_quorum() {
    let c = fixture(790, 900, Classification::Anomaly, false, false, true);
    assert_eq!(report(&c).outcome, "unconfirmed");
}
#[test]
fn measurement_tamper_and_wrong_opening_fail() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.record.observations[0].payload.captured_at_ms -= 1;
    assert_eq!(report(&c).outcome, "rejected");
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.disclosure.openings[0].co2_ppm += 1;
    assert!(report(&c).reason.contains("opening mismatch"));
}
#[test]
fn event_nonce_replay_and_ballot_substitution_fail() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.request.context.nonce = "ff".repeat(32);
    assert!(report(&c).reason.contains("challenge"));
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.record.ballots[0].payload.proposal_id = "00".repeat(32);
    assert_eq!(report(&c).outcome, "rejected");
}
#[test]
fn forged_final_and_weight_summaries_fail_even_when_resigned() {
    let mut c = fixture(790, 830, Classification::Normal, false, false, false);
    c.record
        .proposal
        .payload
        .final_decision
        .as_mut()
        .unwrap()
        .classification = Classification::Anomaly;
    resigned(&mut c);
    assert!(report(&c).reason.contains("final decision"));
    let mut c = fixture(790, 830, Classification::Normal, false, false, false);
    c.record.proposal.payload.weights[0].units = 500000;
    resigned(&mut c);
    assert!(report(&c).reason.contains("weights mismatch"));
}
#[test]
fn duplicate_ballot_and_role_impersonation_fail() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.record.ballots[1] = c.record.ballots[0].clone();
    assert_eq!(report(&c).outcome, "rejected");
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.record.ballots[0].payload.voter_id = "coordinator".into();
    assert_eq!(report(&c).outcome, "rejected");
}
#[test]
fn registry_revocation_and_policy_change_invalidate_record() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.trust.registry.members[0].revoked = true;
    assert_eq!(report(&c).outcome, "rejected");
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.trust.policy.threshold_ppm = 1000;
    assert!(report(&c).reason.contains("configuration"));
}
#[test]
fn expiry_and_replacement_are_currentness_not_historical_rejection() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.request.now_ms = 40000;
    assert_eq!(report(&c).outcome, "unconfirmed");
    c.request.current = false;
    verified(&c);
    c.request.current = true;
    c.request.now_ms = 10000;
    c.request.heads.insert("sensor-a".into(), "ff".repeat(32));
    assert_eq!(report(&c).outcome, "unconfirmed");
    c.request.current = false;
    verified(&c);
}

#[test]
fn invalid_signatures_are_rejected_even_when_support_is_stale_or_replaced() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.request.now_ms = 40000;
    c.record.ballots[0].signature = "00".repeat(64);
    assert_eq!(report(&c).outcome, "rejected");
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.request.heads.clear();
    c.record.observations[1].signature = "00".repeat(64);
    assert_eq!(report(&c).outcome, "rejected");
}
#[test]
fn no_disclosure_never_claims_private_proof() {
    let c = fixture(900, 500, Classification::Anomaly, false, false, false);
    let r = checker::check(&c.record, None, &c.trust, &c.request);
    assert_eq!(r.outcome, "unconfirmed");
    assert!(!r.private_proof_verified && !r.action_released);
}
#[test]
fn raw_values_and_guessable_trace_hash_are_absent_from_public_record() {
    let c = fixture(790, 830, Classification::Normal, false, false, false);
    let json = serde_json::to_string(&c.record).unwrap();
    for forbidden in [
        "co2_ppm",
        "blinding",
        "reasoning",
        "contributing_factors",
        &c.disclosure.event.readings[0].evidence_id,
        &c.disclosure
            .stage
            .selected_reasoner_output_hash
            .clone()
            .unwrap(),
    ] {
        assert!(!json.contains(forbidden), "leaked {forbidden}");
    }
    let (again, _) = attest(&c.disclosure.event, &c.disclosure.stage, &c.trust, &c.keys).unwrap();
    assert_ne!(
        c.record.observations[0].payload.commitment,
        again.observations[0].payload.commitment
    );
    assert_ne!(
        c.record.proposal.payload.trace_commitment,
        again.proposal.payload.trace_commitment
    );
}
#[test]
fn altered_private_reasoner_text_breaks_trace_binding() {
    let mut c = fixture(900, 500, Classification::Anomaly, false, false, false);
    c.disclosure
        .stage
        .selected_recommendation
        .as_mut()
        .unwrap()
        .recommendation
        .reasoning = "changed text".into();
    assert!(report(&c).reason.contains("trace commitment"));
}
#[test]
fn signed_bad_numerical_trace_is_recomputed_not_trusted() {
    let mut c = fixture(790, 830, Classification::Normal, false, false, false);
    c.disclosure.stage.fusion.as_mut().unwrap().weighted_sum += 10000000;
    let (record, disclosure) =
        attest(&c.disclosure.event, &c.disclosure.stage, &c.trust, &c.keys).unwrap();
    c.request.heads = record
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
    c.record = record;
    c.disclosure = disclosure;
    assert!(report(&c).reason.contains("fusion transcript"));
}
#[test]
fn canonical_bytes_are_domain_separated_and_transport_order_independent() {
    let a: serde_json::Value = serde_json::from_str("{\"b\":2,\"a\":1}").unwrap();
    let b: serde_json::Value = serde_json::from_str("{\"a\":1,\"b\":2}").unwrap();
    assert_eq!(
        canonical::bytes("vector", &a),
        canonical::bytes("vector", &b)
    );
    assert_ne!(
        canonical::bytes("vector", &a),
        canonical::bytes("ballot", &a)
    );
    assert_eq!(
        hex::encode(canonical::bytes("vector", &a)),
        "00000013766572692d6164616d2f766563746f722f7632080000000200000001610300000000000000010000000162030000000000000002"
    );
}
