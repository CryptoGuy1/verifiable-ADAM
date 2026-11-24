use std::time::Duration;
use veri_adam_agents::*;
use veri_adam_protocol::*;

struct Model {
    answer: bool,
    wrong_ids: bool,
    delay_ms: u64,
}
impl Reasoner for Model {
    fn label(&self) -> String {
        "test-model".into()
    }
    fn infer(&mut self, v: &EvidenceView) -> Result<Inference, String> {
        std::thread::sleep(Duration::from_millis(self.delay_ms));
        Ok(Inference {
            recommendation: Recommendation {
                advisory: self.answer,
                evidence_ids: if self.wrong_ids {
                    vec!["invented".into()]
                } else {
                    v.evidence_ids.clone()
                },
                explanation: "Test assessment with explicit evidence references.".into(),
            },
            metadata: serde_json::json!({"backend":"test"}),
        })
    }
}
#[test]
fn independent_dissent_is_preserved_in_signed_ballots() {
    let mut r = Runtime::new(Box::new(RuleReasoner));
    r.round("boundary", [Some(790), Some(830)], false, false, 2000)
        .unwrap();
    let run = r.finish("rules".into());
    let round = &run.rounds[0];
    assert_eq!(round.state, "raised");
    let b = round.bundle.as_ref().unwrap();
    assert!(
        !b.ballots
            .iter()
            .find(|b| b.payload.voter_id == "voter-a")
            .unwrap()
            .payload
            .advisory
    );
    assert!(
        b.ballots
            .iter()
            .find(|b| b.payload.voter_id == "voter-b")
            .unwrap()
            .payload
            .advisory
    );
    assert_eq!(round.reference_outcome, Some(Outcome::VerifiedCurrent));
    assert_eq!(
        round.expiry_check.as_ref().unwrap().outcome,
        Outcome::Expired
    );
    for assessment in &round.assessments {
        verify(
            &assessment.payload.canonical_bytes(),
            &assessment.signature,
            &run.registry
                .member(&assessment.payload.voter_id, Role::Voter)
                .unwrap()
                .public_key,
        )
        .unwrap();
    }
}
#[test]
fn majority_cannot_override_public_evidence_policy() {
    let mut r = Runtime::new(Box::new(Model {
        answer: true,
        wrong_ids: false,
        delay_ms: 0,
    }));
    r.round("wrong majority", [Some(850), Some(700)], false, false, 2000)
        .unwrap();
    let run = r.finish("test".into());
    let round = &run.rounds[0];
    assert_eq!(
        round
            .assessments
            .iter()
            .filter(|a| a.payload.advisory)
            .count(),
        2
    );
    assert_eq!(round.state, "withheld");
    assert_eq!(
        round.verification.as_ref().unwrap().outcome,
        Outcome::Rejected
    );
    assert_eq!(round.reason, "classification_mismatch");
}
#[test]
fn invented_evidence_abstains_without_fallback_and_quorum_is_not_lowered() {
    let mut r = Runtime::new(Box::new(Model {
        answer: true,
        wrong_ids: true,
        delay_ms: 0,
    }));
    r.round(
        "bad model evidence",
        [Some(790), Some(830)],
        false,
        false,
        2000,
    )
    .unwrap();
    let run = r.finish("test".into());
    let round = &run.rounds[0];
    assert_eq!(round.state, "withheld");
    assert!(round.bundle.is_none());
    assert_eq!(round.assessments.len(), 2);
    assert!(round.inferences.is_empty());
    assert!(round.absent[0].contains("invented evidence"));
}
#[test]
fn late_model_response_cannot_vote_or_extend_evidence_lifetime() {
    let mut r = Runtime::new(Box::new(Model {
        answer: true,
        wrong_ids: false,
        delay_ms: 100,
    }));
    r.round("late", [Some(790), Some(830)], false, false, 40)
        .unwrap();
    let run = r.finish("test".into());
    let round = &run.rounds[0];
    assert_eq!(round.state, "withheld");
    assert!(round.bundle.is_none());
    assert!(round.absent.iter().any(|s| s.contains("deadline missed")));
}

#[test]
fn stalled_model_does_not_prevent_two_other_voters_from_completing() {
    let mut r = Runtime::new(Box::new(Model {
        answer: false,
        wrong_ids: false,
        delay_ms: 700,
    }));
    r.round(
        "slow model, sufficient independent votes",
        [Some(900), Some(920)],
        false,
        false,
        500,
    )
    .unwrap();
    let run = r.finish("test".into());
    let round = &run.rounds[0];
    assert_eq!(round.state, "raised");
    assert_eq!(round.bundle.as_ref().unwrap().ballots.len(), 2);
    assert!(round.absent.iter().any(|s| s.contains("deadline missed")));
    assert!(round.completed_at_ms - round.started_at_ms <= 500);
}
#[test]
fn missing_or_tampered_sources_do_not_trigger_model_or_collective_release() {
    let mut r = Runtime::new(Box::new(RuleReasoner));
    r.round("missing", [None, Some(900)], false, false, 2000)
        .unwrap();
    r.round("tampered", [Some(900), Some(920)], false, true, 2000)
        .unwrap();
    let run = r.finish("rules".into());
    for round in &run.rounds {
        assert_eq!(round.state, "withheld");
        assert!(round.assessments.is_empty());
        assert!(round.bundle.is_none());
    }
    assert_eq!(run.rounds[1].reason, "invalid signature");
}
#[test]
fn showcase_keeps_absence_denominators_and_low_co2_is_a_valid_clear_result() {
    let run = showcase(Box::new(RuleReasoner), "rules".into()).unwrap();
    assert_eq!(run.rounds.len(), 7);
    assert_eq!(run.rounds[0].state, "clear");
    assert_eq!(run.rounds[1].state, "raised");
    assert_eq!(run.rounds[5].state, "withheld");
    assert_eq!(
        run.events.last().unwrap().advisory_state.as_deref(),
        Some("withdrawn")
    );
    assert!(!run.zkp_implemented);
    let html = html(&run).unwrap();
    assert!(!html.contains("__SIMULATION_DATA__"));
}
