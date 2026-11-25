# Requirements traceability and drift register

Standing user instruction, 25 November 2026: adhere to the agreed specifications for every implementation, simulation and demo; explicitly warn before drifting. See the mandatory workflow in [AGENTS.md](../AGENTS.md).

## Sources and authority

- **P:** Accepted proposal, `/Users/greatadams/Downloads/veri_ADAM_Research_Proposal.docx`, especially objectives O1–O5, technical architecture, selective privacy statement, threat model and baseline comparisons.
- **R:** Client refinement, `/Users/greatadams/Downloads/veri_ADAM_CO2_Experimental_Protocol.docx`, especially purpose/scientific separation, signed evidence/decision-bundle requirements, verification modes, currentness, attack matrix and experimental gates.
- **A:** Original ADAM manuscript, `/Users/greatadams/Downloads/ADAM_EAAI-3.pdf`, Sections 3.2.3, 3.3, 3.4 and 3.5, for paper-derived aggregation, voting, reasoner interface and governance. A reconstruction with Qwen, CO2 and synthetic calibration is not an exact reproduction of the historical Gemma/methane experiment.
- **U:** Explicit user decisions in this chat. They update local project scope; external client acceptance is not inferred.

When implementation details are absent from the sources, distinguish a routine technical choice from a behavioural requirement change. Do not invent evidence of client authorization. Proposed plans describe intended work, not completed implementation or approval of every unstated detail.

## Required behaviour and acceptance evidence

| ID | Requirement and source | Evidence needed | Current position |
| --- | --- | --- | --- |
| REQ-01 | ADAM-compatible weighted fusion, role-specific assessment, coordination and governance. R purpose; A 3.2.3–3.5; U original weighted rule first. | Independent vectors for inverse-variance weights, weighted dispersion, temporal Sensor rule, recommendation selection, quorum, final assembly, confidence and action checks. | **Partly corrected in shared decision core:** calibrated weighted fusion/dispersion, temporal Sensor logic, complete recommendations, crew assembly and governance now have hand-reviewed tests. Worker/signed-v2 integration and historical semantic retrieval remain open; the old browser demo still deviates. |
| REQ-02 | Timestamped, source/role/event-bound authenticated observations and canonical signing. P O1/O2; R signed-record requirements. | Published encodings and signature vectors; reject tampering, substitution and replay; actual device acquisition/key evidence before deployment claims. | Public v1 development encoding/signatures exist. Physical acquisition, private device keys and durable replay state remain incomplete. |
| REQ-03 | Full decision bundle includes measurement IDs, registered weights, reasoner reference, eligible roles, individual ballots, quorum version, proposed action, policy verdict and proof references. R decision-bundle requirements. | Signed complete record and independent field/binding checks. | **Partly corrected:** the new unsigned decision-stage record carries the complete recommendation/final action, evidence context, weights/profile references and policy result. The authenticated v2 contract and signed ballots are not integrated; the legacy demo remains boolean-only. |
| REQ-04 | Independently verify actual quorum and specified policy; never silently lower eligibility or conflate fusion with crew resolution. R verification modes/currentness/invalid-majority attack; A 3.3.3–3.5. | Authentic disagreement and forged-summary cases have independently specified, distinct outcomes. | **Corrected in the shared decision core:** tests accept legitimate majorities on either side of the fused result, retain selected confidence and apply explicit alignment/governance. Legacy v1 remains unchanged and must not define the new protocol; signed-v2 independent verification is still open. |
| REQ-05 | Decisions depend on exact support, freshness, replacement, calibration, policy and role versions; historical processing differs from current support. P O3; R currentness/scenarios. | Agent-integrated dependency tests, independent reference agreement and changes during publication. | B2/B3 libraries exist. Current agent demo uses B1/reference and modeled heads/expiry, not integrated B3 or authenticated peer updates. |
| REQ-06 | Real private threshold verification over authenticated hiding commitments. P O4; R privacy/baseline sections; U expands first proof to weighted fusion from both hidden CO2 readings. | Genuine proof generation and independent verification; public export excludes readings, openings and leaking private explanations; adversarial binding/range/freshness cases. | **Not implemented.** Existing hashes, signatures and UI are not substitutes. Original proposal's smaller single-reading proof is not the user's selected next target. |
| REQ-07 | Independent acceptance/rejection and durable event history with policy-governed advisory release. P architecture; A 3.5 recording/release; U integrated local system. | Recording acknowledgment before publication, recovery, signed/protected checkpoints and explicit corruption/truncation/rollback assumptions. | Exported reports/manifests exist; durable tamper-evident audit/governance remains incomplete. Blockchain anchoring stays optional. |
| REQ-08 | Fair B1/B2/B3 and strong ordinary indexed-cache comparisons, counting total costs at equivalent assurance. P novelty/baselines; R hypothesis and gates. | Matched frozen workloads, independently expected outcomes, maintenance/verification/proof costs, neutral/adverse results and later device measurements. | Development replay/counters exist. No demonstrated novelty or overall B3 advantage. The agent demo is not a matched B2/B3 benchmark. |
| REQ-09 | Separate physical, replay and synthetic evidence; no fabricated acquisition or independence. R scientific separation; U revised procurement. | Source kind, original/replay timestamps, provenance and honest device/sensor counts in every report. | Two Pis are owned; current order is two breadboards, one SCD40 and one BH1750, no cooling packs. Additional channels are datasets, not simultaneous independent sensors. No Pi deployment has been completed here. |
| REQ-10 | Demo behaviour follows governing specifications; displayed checks are real and omissions explicit. U standing no-drift instruction. | UI reads the agreed runtime's records; each success claim traces to an actual check; legacy/provisional views carry a clear limitation notice. | Current recorded demo is a legacy provisional implementation, not the agreed completed research demonstration. |

## Explicit user decisions to preserve

- Integrated local system first; Pi deployment is a later validation milestone.
- Original weighted ADAM logic before the new private-proof integration.
- Synthetic calibration/reference fixtures for development; real calibration is required before corresponding physical accuracy claims.
- First private proof covers the weighted fused classification from both hidden readings. It does not prove Qwen inference or equate the fused result with the final crew class.
- Buy exactly two breadboards, one SCD40 and one BH1750. Remove cooling packs from this order; reuse existing wires and tools. Use clearly labeled datasets to reduce additional sensor purchases.
- No copying the previous project wholesale. Reuse necessary ideas/components only when justified and traced to the required behaviour.

These are user-authorized local decisions, not statements that the external client has approved revised procurement or proof scope. Do not re-ask the user to authorize them during routine implementation.

## Handling known mismatches

The existing equal-weight/boolean-only/v1 acceptance paths and their recorded examples are retained for reproducibility. They must not be promoted as compliant ADAM baselines, deployed research policy or evidence of the intended private-proof/dependency contribution. Passing their regression tests preserves legacy behaviour only.

Correct these mismatches through the agreed versioned pipeline work, with independently specified acceptance cases. Do not erase original bytes or silently reinterpret existing signatures. An old rejected-majority fixture cannot be used to require rejection of a legitimate majority under a different, correctly specified policy.

Before each implementation milestone, report the relevant IDs, changes, source sections, tests and remaining gaps. On an unapproved deviation, use the **SPEC DRIFT WARNING** format in AGENTS.md before proceeding with that affected change. A corrective action already authorized by the user does not require another permission cycle.

## Corrective implementation evidence

The shared `crates/decision` component and `specs/adam-decision-stage.md` implement and document the first corrective stage for REQ-01/03/04. Seventeen hand-reviewed rule tests and a CLI integration test cover calibration, weighted/temporal assessments, authentic disagreement, fixed quorum, selected-recommendation provenance, repair/fallback and governance/deadline outcomes. Existing signed v1 tests remain regression coverage only.

The component explicitly stops at policy approval awaiting recording, with authentication, durable acknowledgment, release and proof flags false. Configurable action names/alignment, fallback confidence and the numerical input to the high-concentration guard are required policy choices; the supplied values are clearly synthetic development-fixture choices rather than recovered client defaults. Semantic/vector retrieval is marked not integrated, not silently simulated as successful. Signed-v2/worker/UI migration, B3 integration, ZKP, durable audit and hardware validation remain pending.
