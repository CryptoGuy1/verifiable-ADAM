# Verifiable ADAM

**Specification alignment warning:** The current demo uses provisional equal-weight, boolean-only decision semantics and is not the completed client-specified ADAM workflow. See [requirements and known gaps](specs/requirements-traceability.md). All further implementation and demo work must follow [AGENTS.md](AGENTS.md); deviations require an explicit **SPEC DRIFT WARNING** before proceeding with the affected change.

Verifiable ADAM investigates whether dependency-aware verification can reduce repeated work compared with a competent expiry-aware cache while preserving the same acceptance rules. Our physical testbed is one Pi 5 and one Pi 4, using CO₂ and light observations.

The present equal-weight rule and simulation are **ADAM-inspired**, not a reproduction of original ADAM's weighted fusion, role-specific ballots, reasoner, and governance. Equivalence with a strong ordinary indexed cache is the working hypothesis for the current B3 index. See [the review decisions](specs/review-decisions.md) for the comparison, privacy-format, and data-collection gates.

The software now includes signed synthetic observations, coordinator-signed decisions, individually signed ballots, a deterministic B1 public verifier, a B2 expiry-aware cache, a B3 reverse dependency index, and a separately implemented reference evaluator. A matched replay runner checks B1, B2, and B3 against declared expected outcomes and the reference. These establish a protocol and development corpus, not research novelty, physical accuracy, or hardware performance.

## Connected weighted agents — no UI update

```sh
cargo run --locked --offline -- adam-agent-run runs/NEW rules
cargo run --locked --offline -- adam-agent-run runs/OTHER ollama
```

This path connects actual inbox-driven workers to the weighted signed pipeline, v2 B1/B2/B3 checks, a real two-reading weighted private proof and a durable advisory-authorization gate. Use `private-export/` for the restricted auditor package; full round/disclosure files contain private readings and model text. See [the runtime contract and remaining gates](specs/weighted-agent-runtime.md), including optional Weaviate/MiniLM history setup. The UI is unchanged at the user’s request. These are synthetic local runs, not deployed Pis, physical actuation or demonstrated novelty.

## Weighted ADAM decision stage

The corrective `decision` module implements paired-reference inverse-variance calibration, weighted fusion/dispersion, temporal Sensor assessment, the seven-field recommendation, recommendation selection, strict crew quorum, final action alignment and governance. It preserves genuine majority disagreement with the fused result. See [the source mapping and limits](specs/adam-decision-stage.md).

```sh
cargo run --locked --offline -- adam-run runs/adam-stage-new rules
# Uses local qwen3:1.7b after model preparation; choose a new directory.
cargo run --locked --offline -- adam-run runs/adam-stage-qwen-new ollama
```

These commands export inspectable unsigned decision-stage records and explicit policies/calibration. They do not release an advisory. The connected actor path above now wraps this engine, with private fusion proofs, dependencies and durable authorization. Historical semantic-service validation and deployment remain open; UI changes are deferred. The older browser demo below still uses legacy v1 semantics.

## Authenticated weighted records and independent checking

```sh
cargo run --locked --offline -- adam-auth-run runs/adam-authenticated-new rules
# Uses the same weighted engine with the installed local Qwen model:
cargo run --locked --offline -- adam-auth-run runs/adam-authenticated-qwen-new ollama
cargo run --locked --offline -- adam-check \
  runs/adam-authenticated-new/weighted-normal/record.json \
  runs/adam-authenticated-new/weighted-normal/authorized-disclosure.json \
  runs/adam-authenticated-new/trust.json \
  runs/adam-authenticated-new/weighted-normal/request.json
```

Version 2 binds committed source observations, registered weights, complete decision/reasoner references, eligible roles, signed ballots, proposed action and governance. A separate checker recomputes the weighted/temporal rules, actual quorum and policy. The runner uses fresh ephemeral OS-random keys and synthetic evidence; these are local components, not deployed agents or device provisioning. Source mapping, canonical encoding, acceptance cases and limits are in [the authenticated record contract](specs/authenticated-decision-v2.md).

Successful processing still says **awaiting recording**, with no action release. Real hiding commitments prepare the selected privacy path, but **ZKP is not implemented**: `adam-check RECORD - TRUST REQUEST` returns unconfirmed. `authorized-disclosure.json` contains readings, openings and private model text; do not send it to an external private verifier. Legacy v1 signed bytes and examples remain unchanged.

## Run the agent simulation with local Qwen

```sh
cargo run --locked --offline -- agent-simulate runs/agents-local ollama
python3 scripts/run_simulation_viewer.py runs/agents-local/simulation.html --port 8767
```

Open **http://127.0.0.1:8767/**. This runs stateful Sensor, Aggregator and Decision actors through message inboxes; the Decision actor calls the installed local `qwen3:1.7b`. The viewer shows individual assessments, dissenting signed ballots, the collective advisory, withholding and expiry. Use a new output directory. The `rules` backend tests the same actors without model inference. See [the agent contract](specs/agent-simulation-v1.md) for signature bindings, real-time deadlines, fault cases and limitations. The coordinator is a nonvoting local state machine; these are local workers, not deployed Pi services. This legacy viewer has no ZKP; the new terminal pipeline above implements the weighted private-claim pilot.

## Run the scripted trajectory simulation

To view the saved example, run this from the project folder and leave the terminal running:

```sh
python3 scripts/run_simulation_viewer.py
```

Open **http://127.0.0.1:8765/** in your browser, then click **Play**. This serves just the simulation page on your own computer. No packages need installing. If the page says JavaScript has not started, check that your browser allows JavaScript for this local page. An initialization failure now displays an error instead of an active-looking Play button. Use `--port 8766` if the port is occupied.

To generate a different run:

```sh
cargo run --locked -- simulate runs/sim-mixed mixed 42
python3 scripts/run_simulation_viewer.py runs/sim-mixed/simulation.html
```

The run creates `simulation.html` with playback controls, CO₂/light plots, node/link states, strategy outcomes, and an event timeline. It also saves the configuration, observations, workload, reports, and manifest. The simulation uses virtual time, toy environmental data, and public fixture keys; no sensors or Pis need to be connected.

Scenarios are `stable`, `dropout`, `partition`, `delay`, `tamper`, `replay`, `false-reading`, and `mixed`. Use a new directory each time. For example, `simulate runs/sim-stable stable 42` produces a normal run, while `simulate runs/sim-false false-reading 42` demonstrates that signed false readings can pass protocol verification. `simulate-config CONFIG NEW_DIRECTORY` accepts the saved typed configuration. See [the simulation contract](specs/simulation-v1.md) and [what remains to build](docs/project-status.md).

## Run the signed event demonstration

Install the Rust toolchain selected in `rust-toolchain.toml` on the development machine. Both Pis still need Rust installed; nothing in this repository has been deployed to them. The workspace pins Rust 1.98.1 and commits dependency versions in Cargo.lock.

```sh
cargo run --locked -- demo runs/first-demo
cargo run --locked -- verify runs/first-demo/bundle.json runs/first-demo/policy.json runs/first-demo/registry.json runs/first-demo/request.json
```

Use a new directory for every demo; existing outputs are not overwritten. The demo uses public, deterministic test keys and synthetic readings of 900 and 850 ppm. No private device keys or real physical data are generated.

The demo also writes alternate inputs:

```sh
# Rejected: a reading was changed after signing. Exits 1.
cargo run --locked -- verify runs/first-demo/bundle-tampered.json runs/first-demo/policy.json runs/first-demo/registry.json runs/first-demo/request.json

# Expired: the check occurs exactly at the support expiry. Exits 1.
cargo run --locked -- verify runs/first-demo/bundle.json runs/first-demo/policy.json runs/first-demo/registry.json runs/first-demo/request-expired.json

# Historical processing verifies, but this is not a current advisory. Exits 0.
cargo run --locked -- verify runs/first-demo/bundle.json runs/first-demo/policy.json runs/first-demo/registry.json runs/first-demo/request-historical.json
```

The verifier prints JSON containing its outcome, reason, decision ID, support expiry, and work counters. It also runs the independent reference checker and reports an operational error if their outcomes disagree. These counters describe the B1 software path; they are not benchmark results.

**Trust boundary:** policy, registry, and request files are trusted verifier configuration. Do not accept them from a claimant along with its bundle. The request supplies the expected event, nonce, check time, check kind, and authenticated evidence checkpoints. In this demo those files are synthetic fixtures, not a secure provisioning system.

## Run the B2 cache replay

```sh
cargo run --locked -- replay-demo runs/b2-demo
cargo run --locked -- replay workloads/b2-currentness-v1.json runs/b2-replay
```

Each command uses a new output directory and saves a workload, full report, and hash manifest. The frozen workload has 31 synthetic checks covering stable evidence, unrelated updates, exact expiry, historical checks, replacement, malformed claims, revocation, quorum failures, and restart. A replay exits 0 only when B1, B2, B3, the reference, and expected outcomes all agree; mismatch exits 1 and input/operational error exits 2. Run `replay WORKLOAD` without an output directory to print full JSON.

B2 reuses valid signatures and decision-processing results while checking time and required source heads on every decision hit. Its complete policy/registry/content bindings prevent stale cached authorization. It already avoids revalidation for unrelated updates. The report includes cache maintenance, key hashing/normalization, and occupancy counters as well as saved signature work. These are diagnostic counts, not timing or hardware performance measurements. See [the B2 contract](specs/b2-cache.md).

## Run the B3 dependency index prototype

```sh
cargo run --locked -- dependency-demo runs/b3-demo
cargo run --locked -- replay workloads/b3-multi-claim-v1.json runs/b3-replay
```

B3 shares B2's static authentication cache and adds a source-to-claim reverse index. It updates mismatch state only along affected links. It still receives full head snapshots, so change detection and index maintenance are explicitly counted. The frozen 27-step workload exercises multiple logical claims, shared source links, dropout, replacement, authorization changes, and restart.

On this synthetic workload B2 and B3 perform the same 23 signature checks. B3 saves some direct support comparisons but adds snapshot scanning and metadata work. No overall speedup or novelty is established. See [the B3 contract and findings](specs/b3-dependencies.md). Reports use schema version 2, while both workload input formats remain version 1.

## Code structure

| Path | Responsibility |
| --- | --- |
| `crates/protocol` | Typed records, explicit signing encodings, SHA-256 IDs, Ed25519 signatures, public demo fixtures |
| `crates/verification` | B1 authentication, source checks, deterministic classification, quorum, currentness |
| `crates/reference` | Independent rule/currentness evaluator sharing only record validation, encoding, and crypto primitives |
| `crates/cache` | B2 bounded signature/decision caches, fresh time/head checks, maintenance counters |
| `crates/dependencies` | B3 reverse source/claim index, snapshot reconciliation, bounded metadata cleanup |
| `crates/replay` | Matched B1/B2/B3/reference checks over trusted ordered snapshots |
| `crates/simulation` | Deterministic environmental signals, claim-delivery faults, independent scenario expectations, offline playback |
| `crates/decision` | Paper-derived weighted calibration, role decisions, complete recommendation, crew resolution and governance; authenticated durable release still pending |
| `crates/cli` | New demo runs, bounded JSON input, machine-readable verification reports |
| `specs/protocol-v1.md` | Implemented provisional semantics and exact binary encodings |
| `specs/next-milestones.md` | Remaining work and baseline fairness constraints |
| `tests/vectors` | Fixed encoding/hash vector and standard signature-vector coverage |
| `tests/corpus/demo-v1` | Frozen synthetic bundles, trusted configuration, and expected outcomes |
| `workloads/b2-currentness-v1.json` | Frozen synthetic currentness and attack replay |
| `workloads/b3-multi-claim-v1.json` | Frozen synthetic multi-claim dependency replay |
| `docs` | Build context, hardware inventory, and procurement |

Python will supply acquisition, orchestration, and analysis later. We have not created empty sensor drivers or claimed hardware integration before selecting and receiving the boards. Existing Elsevier IoT source code has not been copied into this project.

## Verification

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

After dependencies are fetched, add `--offline` to test/build commands for repeatable local checks. Tests cover signed tampering, event/nonce replay, policy and registry changes, duplicate voters and sources, missing quorum, exact expiry boundaries, replacement checkpoints, malformed input, numerical bounds, canonical encoding, CLI exit codes, cache poisoning/eviction/restart, saved replays, and deterministic matched sweeps.

## Scope of this delivery

The provisional public rule is an equally weighted mean CO₂ threshold, calculated with exact integer sums, with every configured source required. This is an initial controlled fixture, not a full reproduction of original ADAM aggregation, reasoner, or governance behavior. The 800 ppm threshold is experimental and is not a safety limit.

Nominal mode requires two matching ballots from three registered logical voters. Explicit degraded mode requires both of two voters. The demo maps three logical voters onto two node labels; it does not demonstrate three physically independent voters or Byzantine fault tolerance.

For the legacy paths in this section, audit durability, private proofs and the new advisory gate are absent; use the connected weighted pipeline above for those local components. Alternative support, incremental fusion, exact B0 reproduction, production device provisioning, persistent acquisition replay state, live sensors and secure network services remain incomplete. B2 and B3 are currently in-memory libraries used by replay and simulation; separate `verify` invocations remain cold B1 checks. Currentness is checked against the caller's trusted snapshot; there is no atomic live check-and-release mechanism yet. A signed false reading can still pass authentication. The simulator transports pre-signed claim packets, not distributed planner/ballot negotiation.

See [BUILD_PLAN.md](BUILD_PLAN.md) for the larger checklist and [the next milestones](specs/next-milestones.md) for the implementation sequence.
