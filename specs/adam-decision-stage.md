# Paper derived ADAM decision stage

This shared Rust component implements the numerical, voting and governance stage required by REQ-01, REQ-03 and REQ-04. It is the first corrective milestone, not the complete authenticated version 2 system. The old signed version 1 records and expected outcomes remain unchanged as legacy fixtures.

## Source and implementation boundary

The source is the user-supplied `ADAM_EAAI-3.pdf`: Section 3.2.3 for inverse-variance calibration and weighted dispersion; Sections 3.3.1–3.3.4 for role assessment, recommendation selection, majority and final assembly; Section 3.4.2 for the seven-field recommendation; Section 3.4.5 for repair/fallback; and Section 3.5 for application validation and recording-before-release. The client CO2 refinement requires registered weights, role-attributed ballots, proposed actions and policy verdicts; it does not require final crew classification to equal the fused threshold.

The engine is reusable library code, with file-based commands for exercising the same functions. Its `adam-decision-stage/v1` record is an unsigned component record, not the old protocol v1 bundle or the outer cryptographic protocol v2. `authenticated_record`, `durable_recording_acknowledged`, `action_released` and `private_proof_verified` are explicitly false. A successful governance result ends at `approved_awaiting_recording`; no caller-supplied boolean can simulate a durable acknowledgment or release an action through this API.

The historical semantic/vector retrieval service is not implemented. The reasoner context explicitly records `semantic_retrieval_status: not_integrated`; an empty history is not presented as a successful retrieval. The [authenticated v2 wrapper and independent checker](authenticated-decision-v2.md) now bind this stage to signed observations/ballots. Actual message-driven worker migration, private proofs, dependency integration, durable audit and Pi deployment remain open milestones. The legacy agent UI is not migrated to this new decision stage yet.

## Calibration and fusion

`calibrate` takes unique source series of 2–10000 paired sensor/reference observations, bounded at 0–40000 ppm. It computes residual sample variance with the n−1 denominator. Zero variance, duplicate identities and out-of-bound pairs are rejected. Integer moments are evaluated exactly before the variance is converted to floating point; constant bias is not incorrectly treated as residual variance.

Inverse variances are normalized and quantized to positive integer weights summing to 1000000 by largest remainders, with source ID breaking equal-remainder ties. Zero-quantized weights are rejected rather than silently clamped. The profile includes its source-data hash, version, provenance and synthetic flag. These are trusted numerical inputs here; authenticating profile publication belongs to v2.

Fusion uses the registered weights of the actual contributing sources and renormalizes when contributors are absent. Unknown or duplicate sources are rejected. Classification compares exact integer weighted sums with threshold times the contributing weight sum. Floating-point display mean and weighted population dispersion are also returned, together with the unquantized inverse-variance mean and quantization delta. This precision decision is explicit; no claim of bit-for-bit reproduction of historical floating-point outputs is made.

The provided development fixture contains 500 synthetic pairs per source with residual patterns `[-2,-1,0,1,2]` and twice that pattern. It yields weights 800000 and 200000. For readings 790/830, the weighted result is 798 ppm with dispersion 16 ppm; the old equal-weight average was 810 ppm. The fixture is not calibration for an SCD40, and the paper's methane weights are not reused as CO2 calibration.

## Roles and decision lifecycle

- Sensor classification uses the triggering reading against the threshold OR at least twice its baseline. The supplied temporal window takes precedence; otherwise the engine uses up to six earlier local observations from the same source/session. With no prior observations, the current reading initializes the baseline. Future/current-time, duplicate and other-source window entries are rejected.
- Aggregator classification uses the registered weighted concentration. Dispersion is context, not a replacement voting rule.
- Decision classification comes from the selected seven-field recommendation. The Coordinator does not vote. Sensor/Decision/Coordinator are required at formation; Aggregator absence permits only the explicitly degraded Sensor/Decision set. Subsequent ballot loss never reduces that set or quorum.
- Competing recommendations must refer to the same location and configured event window. Highest severity wins, then latest recommendation time. A source-event ID breaks an exact severity/time tie as a documented deterministic implementation detail.
- Either binary class can win a strict majority of the formed crew. A legitimate majority may differ from fusion. If crew resolution changes the selected recommendation's class, severity and action come from the required policy alignment table. The selected confidence is retained unchanged; crew support has its own eligible-voter denominator. Original local and selected recommendations remain separately available as provenance.
- Malformed output receives one repair request within the remaining event budget. Inference failure or failed repair uses the explicit threshold fallback when time remains, records the failed attempts and marks degraded mode. Expired deadlines cause timeout rather than fallback. Valid low-confidence output is rejected by governance, not silently repaired into a more confident answer.

The engine keeps local history and processed event IDs in bounded session state; this is not durable anti-replay protection. A monotonic clock supplies processing time, while capture times may use a labeled synthetic epoch. The fixture budget and TTL are both 30000 ms. Expiry and deadline boundaries are exclusive: evidence at expiry or processing at the deadline cannot be published.

## Explicit application policy

The policy requires the permitted/passive action sets, normal/anomaly alignment entries, threshold, event/TTL/window budgets, confidence floor, high-concentration guard and its explicit input source, and fallback confidence. The source specifies governance checks but does not supply a complete project action-name/alignment table or fallback confidence, so these are not hidden source-derived defaults.

The checked development policy uses 800 ppm, confidence floor 0.35 and a 5000 ppm passive-action guard; all are experimental settings, not CO2 safety advice. `ContinueMonitoring`, `VentilationAdvisory`, `RequestHumanReview`, the alignment entries and fallback confidence 0.5 are explicit test-fixture settings. The policy must explicitly select the triggering reading or weighted fusion as the guard input. The fixture selects the triggering reading; no unsupported interpretation is hidden in the engine. The policy and action/alignment choices must be reviewed before v2 contract freezing.

Governance checks structural/severity validity, the selected reasoner's confidence floor, permitted actions, passive response at high concentration, CRITICAL human review and degraded-anomaly disclosure. Confidence remains recommendation provenance rather than a calibrated probability for the crew-resolved result. Policy approval is not cryptographic verification, physical truth or action release.

## Commands and outputs

```sh
cargo run --locked --offline -- adam-run runs/NEW rules
cargo run --locked --offline -- adam-run runs/OTHER ollama
cargo run --locked --offline -- adam-calibrate CALIBRATION_INPUT NEW_DIRECTORY
cargo run --locked --offline -- adam-evaluate EVENT POLICY CALIBRATION_PROFILE
```

All run directories are new/non-overwriting. A run exports policy, synthetic calibration data/profile, individual event inputs, decision-stage records, a manifest and a limitations note. The rules backend is explicitly rule-based; Ollama uses the installed `qwen3:1.7b`, warmed before observations. Its seven-field JSON requests, raw responses, repair attempts, failures and timing metadata are retained. Successful repair is distinct from deterministic failure fallback. No automatic model download, remote inference or hardware change is performed.

`adam-evaluate` treats policy and calibration as trusted inputs and prints a machine-readable stage record. Exit 0 means policy approval awaiting further integration, not action release; exit 1 means a withheld/rejected stage; exit 2 means malformed input or operational failure. `adam-run` exits 0 when it completes and saves its case records, even when particular cases correctly remain unresolved.

Acceptance tests use hand-calculated calibration/weighted vectors and declared role outcomes. They cover authentic disagreement, crew override with preserved confidence, temporal history, formation/degraded quorum, missing/duplicate voters, competing recommendations, one repair/fallback, policy rejection, expiry and deadlines. Existing v1 regression tests remain required and do not certify v2 compliance.
