# Project status and remaining work

**Specification alignment warning:** Existing development milestones are not evidence of compliance with the full agreed specification. Equal-weight fusion, the reduced decision record, the current fusion-equals-final-class rule, absent ZKP, and missing dependency/audit integration are recorded in [the requirements traceability register](../specs/requirements-traceability.md). Follow [AGENTS.md](../AGENTS.md) and explicitly warn before any unapproved deviation, including in demos.

## Working software

- **Corrective ADAM decision stage:** a shared `decision` crate implements inverse-variance calibration, registered weighted fusion/dispersion, temporal Sensor voting, complete recommendations, selection, crew quorum, final assembly, repair/fallback and governance. It is exercised by `adam-run` and `adam-evaluate`; details and source boundaries are in [the decision-stage contract](../specs/adam-decision-stage.md).
- **Authenticated weighted v2 records:** `records` and `adam-auth-run` sign actual weighted-stage outputs, role ballots and hiding source commitments using fresh ephemeral OS-random keys. `adam-check` independently checks full disclosed evidence, numerical/role/quorum rules, reasoner provenance and governance against separate trust/challenge inputs. Rules and real local Qwen runs each verified four cases and withheld the split missing-ballot case. See [the v2 contract](../specs/authenticated-decision-v2.md). This does not migrate the existing worker/browser runtime, implement ZKP, provision devices, persist anti-replay state or authorize release; all successful reports still await durable recording.

- Typed event records, explicit canonical signing format, and real Ed25519 verification.
- Signed measurements, coordinator proposals, and individual logical ballots.
- B1 public recomputation, B2 ordinary caching, and B3 reverse dependency indexing.
- Independently implemented reference acceptance rules and adversarial tests.
- Frozen development workloads, matched reports, input hashes, and virtual-time replay.
- Deterministic two-node claim-delivery simulation with CO₂/light trajectories, faults, and offline playback.
- Local message-driven Sensor/Aggregator/Decision actors, independent signed assessments and ballots, a nonvoting coordinator, advisory publication/withholding/expiry, and an Ollama Qwen adapter. See [agent simulation](../specs/agent-simulation-v1.md). These are local threads, not Pi services, and the old trajectory simulator remains scripted.

The current software is a development prototype. It has not been deployed to the Pis, connected to real sensors, or shown a measured B3 efficiency advantage. Simulation data and public test keys must remain separate from physical research data and production keys.

**Current purchase decision, 25 November 2026:** two breadboards, one SCD40 breakout and one BH1750 light sensor; cooling packs are removed from the order. The revised experiment uses one live CO2 source plus clearly labeled dataset replay for additional channels, not two simultaneous independent CO2 sensors. See [the revised purchase and dataset plan](experiment-procurement.md). This records the user's procurement choice, not external client acceptance or completed hardware validation.

## Five-component coverage and completion gates

This is the completion register for the user's five-component architecture. Working library code, integration in the agent demo, and physical deployment are separate milestones. A visible badge is not evidence that a component is implemented.

| Component | Current agent demo | Remaining completion gate |
| --- | --- | --- |
| Sensor evidence | Synthetic observations have timestamps, source/event/policy bindings, Ed25519 signatures and SHA-256 content IDs. Tampering is rejected. | Device acquisition, private device identities, restart-safe sequence/replay state and authenticated context distribution. Explicit commitment/opening semantics are missing; a deterministic content hash does not provide the hiding needed for private claims. |
| Agent decision record | Separate role assessments, signed ballots, majority collection and B1/reference checking; a wrong majority can be blocked. | A versioned signed record for classification plus a typed proposed action and relevant deadline/governance bindings. Today the final record contains an advisory boolean, not a full signed action proposal. |
| Evidence dependency manager | The demo binds decisions to reading IDs, advances modeled heads, and removes the last accepted advisory at expiry. B2/B3 libraries separately test replacements and policy/registry changes. | Connect actual agents to the dependency manager and authenticated update transport; demonstrate selective revalidation, replacement, policy/role changes and restart recovery. The agent demo currently uses B1/reference, not B3. Do not claim a B3 advantage over an equivalent ordinary indexed cache. |
| Optional private claim | Absent. Readings are public and no proof is generated or checked. | Versioned hiding commitment with fresh randomness; authenticated binding; bounded threshold proof; proof/public-input/freshness verification; adversarial tests and a disclosure audit. An optional branch must be labeled disabled until a real proof is verified. |
| Audit and governance | Acceptance/rejection reports, original model artifacts, signed records and a run manifest are exported. A narrow public policy gate is enforced. | Persistent tamper-evident event history across restarts; authenticated/checkpointed log heads with defined truncation/rollback guarantees; typed action authorization and signed policy evolution; independently reproducible audit verification. Exported JSON is not an immutable audit log. Blockchain anchoring remains optional and absent. |

All demo signing keys are public fixtures. They demonstrate protocol processing, not protection against someone who knows these keys or assurance of physical sensor truth. Three local voters on two intended device labels are not three independent devices.

The user's architecture table mentions methane. The accepted project refinement and implemented public demo use CO2. Retain CO2 as the current scope; do not silently reuse methane field names, calibration or proof statements. A methane return requires an explicit scope decision.

### Integration acceptance checklist

- [ ] Freeze a new versioned record contract covering the typed action, policy/role/deadline context and private-capable commitments; preserve existing v1 signed bytes and verifiers.
- [ ] Route one agent decision through authenticated evidence, individual signed assessments/ballots, quorum, dependency validity and action authorization. An accepted vote alone must not release an action.
- [ ] Connect dependency updates to this same runtime and demonstrate affected versus unrelated support changes, exact expiry, policy/role changes and restart recovery.
- [ ] Complete the optional private branch with a real proof bound to the same authenticated evidence and event. A failure or missing required proof must block that branch; public mode must explicitly state that privacy is not provided.
- [ ] Persist accepted, rejected, withheld and timed-out attempts before reporting durable publication. Reopen the log after restart and verify evidence, policy, decision, dependency/proof checks and protected checkpoints independently.
- [ ] Add one integrated acceptance suite: valid decision; altered evidence; replayed event; duplicate/missing voter; incorrect majority/action; expiry/replacement; unrelated update; policy/role change; forged/misbound/replayed proof; corrupt/truncated/rolled-back log; restart. Expected outcomes must be declared independently of the implementation being tested.
- [ ] Back each demo component indicator with the actual corresponding check. Show `not included` for an absent optional component, never a synthetic successful proof or audit result.
- [ ] Deploy the same semantics to the Pis with real keys/transport/sensors and measure costs there. Computer-hosted Qwen remains hybrid inference until on-Pi inference is demonstrated.

Recommended build order is the record/commitment contract, full decision/action record, runtime dependency integration, durable audit/governance, then optional private proof integration and the complete acceptance demonstration. Pi bring-up and primitive cost measurement can proceed in parallel. Neither private proof nor blockchain anchoring is assumed complete or mandatory merely because it appears in this register.

## Parallel work and decision gates

**Start now in parallel: Pi preparation and cost measurement.** Install the pinned Rust toolchain, validate release builds, choose/install cooling, and check power under load. Record versions, governor, temperature, throttling, network conditions, and clocks. Measure Ed25519 signing/verification and complete cold B1, warm B2, and B3 checks, including all serialization and maintenance costs. These tasks do not require the final fusion or privacy design. Use the real expected check rate and sensing cadence to decide whether further optimization has practical value. Existing counters and virtual deadlines are not timing results.

**Resolve semantics and contribution while those measurements proceed.** Call the current rule and simulation ADAM-inspired. The original paper uses fixed inverse-variance fusion and role-specific ballots, reasoner output, and governance; changing only weights is insufficient to reproduce it. Define B0 from original code/traces where available or explicitly bound a reconstruction to the paper's specification. Decide reusable-observation/event binding, TTL, tolerance, roles, deadlines, and independently expected decisions. Treat equivalence with a strong ordinary indexed cache as the working hypothesis for the present B3 mechanism.

**Freeze the private record decision before confirmatory acquisition.** Version 1 signs public CO₂ values and is not a private-proof format. If B4 is required, specify a new version with a hiding commitment, public/private disclosure boundary, authenticated context, openings, bounds, and freshness before collecting the corresponding dataset. An undefined reserved field is not sufficient. Old public records remain auditable under their original version, but cannot be made private retroactively. Keep proof-system implementation gated until its purpose and budget are justified.

**Predeclare adverse cases before further index tuning.** Include warm-cache stable claims, unrelated snapshots, high fanout, low query/update ratios, eviction/restart, expiry, policy/role changes, and a competent ordinary indexed comparator. Existing exposed simulations are development data, not held-out evidence. Freeze new hypotheses and workloads before choosing favorable scales or inspecting confirmatory results.

**Record the revised budget and development modality.** Proceed with the user's one-SCD40/one-BH1750 purchase scope and dataset-supported development. Confirm exact breakouts, wiring, acquisition settings, calibration and supplier stock. Record the departure from the original two-SCD41/two-BH1750 physical protocol and obtain external agreement where required; do not assume it. Label live, replay and synthetic channels explicitly, never invent a second physical CO2 source or treat unrelated datasets as device calibration.

**Then connect secure acquisition and peers.** Provision OS-random keys, protected storage, role/node bindings, authenticated configuration updates, nonce challenges, sequence/replay persistence, actual transport, UTC/monotonic handling, and explicit failure/deadline behavior. Rust owns canonical encoding/signing; Python can acquire and orchestrate. Model-created trusted checkpoints do not implement this service.

**Complete audit, scoring, and physical evaluation.** Save raw readings, signed records, rejected/timed-out attempts, manifests, and restart traces with defined tamper-evident log guarantees. Keep processing correctness separate from physical sensor accuracy and useful outcomes. Run pilot sessions, then freeze primary outcomes, practical-effect thresholds, sample sizes, uncertainty analysis, held-out traces, and exclusions. Measure on each Pi with role crossover; direct energy measurement remains optional. Cryptographic operations and synthetic seeds are not independent environmental replications.

See [research review decisions](../specs/review-decisions.md) for the verified ADAM differences, privacy-format implications, adverse cases, and client questions. No client answers, purchases, or Pi deployment are assumed.

## Immediate review artifact

The current corrective numerical/decision artifact is `runs/adam-weighted-stage-rules-v2/`; the corresponding Qwen-backed run is `runs/adam-weighted-stage-qwen-v2/`. Inspect `policy.json`, `calibration-input.json`, `calibration-profile.json`, `events.json` and `decision-stage-run.json`. Governance-approved records stop at `approved_awaiting_recording` and have explicit false authentication, durable-acknowledgment, release and private-proof flags. No new browser visualization is claimed for this corrective stage.

The **legacy** Qwen-backed browser run is `runs/agents-qwen-v3/`. Serve it with `python3 scripts/run_simulation_viewer.py runs/agents-qwen-v3/simulation.html --port 8767` and open `http://127.0.0.1:8767/`. It contains four real Qwen assessments and signed votes under the old provisional rule. Its 790/830 ppm majority rejection belongs to that old rule, not original ADAM's crew semantics. This is one development run; Qwen's explanations and classification booleans were sometimes inconsistent. No rewriting of model votes or model-accuracy claim is made. Do not present this legacy view as the corrected research workflow.

To generate a fresh agent run use `cargo run --locked --offline -- agent-simulate runs/NEW_DIRECTORY ollama`. Cold model loading is warmed up before capture; inference after capture counts against the deadline. The recorded viewer does not call Qwen again. Read `specs/agent-simulation-v1.md` for timing, messages and limitations.

The earlier trajectory runner remains available as `cargo run --locked -- simulate runs/NEW_DIRECTORY mixed 42`. That path is still scripted claim delivery, as specified in `specs/simulation-v1.md`.
