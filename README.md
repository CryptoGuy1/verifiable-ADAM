# Verifiable ADAM

Verifiable ADAM investigates whether dependency-aware verification can reduce repeated work compared with a competent expiry-aware cache while preserving the same acceptance rules. Our physical testbed is one Pi 5 and one Pi 4, using CO₂ and light observations.

The software now includes signed synthetic observations, coordinator-signed decisions, individually signed ballots, a deterministic B1 public verifier, a B2 expiry-aware cache, and a separately implemented reference evaluator. A matched replay runner checks B1 and B2 against declared expected outcomes and the reference. These establish a protocol and development corpus, not research novelty, physical accuracy, or hardware performance.

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

Each command uses a new output directory and saves a workload, full report, and hash manifest. The frozen workload has 31 synthetic checks covering stable evidence, unrelated updates, exact expiry, historical checks, replacement, malformed claims, revocation, quorum failures, and restart. A replay exits 0 only when B1, B2, the reference, and expected outcomes all agree; mismatch exits 1 and input/operational error exits 2. Run `replay WORKLOAD` without an output directory to print full JSON.

B2 reuses valid signatures and decision-processing results while checking time and required source heads on every decision hit. Its complete policy/registry/content bindings prevent stale cached authorization. It already avoids revalidation for unrelated updates. The report includes cache maintenance, key hashing/normalization, and occupancy counters as well as saved signature work. These are diagnostic counts, not timing or hardware performance measurements. See [the B2 contract](specs/b2-cache.md).

## Code structure

| Path | Responsibility |
| --- | --- |
| `crates/protocol` | Typed records, explicit signing encodings, SHA-256 IDs, Ed25519 signatures, public demo fixtures |
| `crates/verification` | B1 authentication, source checks, deterministic classification, quorum, currentness |
| `crates/reference` | Independent rule/currentness evaluator sharing only record validation, encoding, and crypto primitives |
| `crates/cache` | B2 bounded signature/decision caches, fresh time/head checks, maintenance counters |
| `crates/replay` | Matched B1/B2/reference checks over trusted ordered snapshots |
| `crates/cli` | New demo runs, bounded JSON input, machine-readable verification reports |
| `specs/protocol-v1.md` | Implemented provisional semantics and exact binary encodings |
| `specs/next-milestones.md` | Remaining work and baseline fairness constraints |
| `tests/vectors` | Fixed encoding/hash vector and standard signature-vector coverage |
| `tests/corpus/demo-v1` | Frozen synthetic bundles, trusted configuration, and expected outcomes |
| `workloads/b2-currentness-v1.json` | Frozen synthetic currentness and attack replay |
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

B3 dependency updates, alternative support, production key provisioning, persistent anti-replay state, live sensor acquisition, network services, audit-log durability, action release, and B4 private proofs are not implemented. B2 is currently an in-memory library used by the replay runner; separate `verify` invocations remain cold B1 checks. Currentness is checked against the caller's trusted snapshot; there is no atomic live check-and-release mechanism yet. A signed false reading can still pass authentication.

See [BUILD_PLAN.md](BUILD_PLAN.md) for the larger checklist and [the next milestones](specs/next-milestones.md) for the implementation sequence.
