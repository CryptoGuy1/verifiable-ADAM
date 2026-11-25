# Verifiable ADAM build plan and open concerns

**MANDATORY SPECIFICATION CHECK:** Follow [AGENTS.md](AGENTS.md) and [the requirements traceability register](specs/requirements-traceability.md) before implementation, simulation or demo work. Existing code and passing fixtures do not authorize different requirements. On an unapproved deviation, give a **SPEC DRIFT WARNING** identifying the source, mismatch, impact and required resolution before changing the affected behaviour.

**Known drift:** the current equal-weight, boolean-only decision path and its requirement that crew classification equal the fused threshold are provisional implementation choices, not instructions in the client refinement. They are not the compliant ADAM pipeline. Preserve their signed records as legacy evidence while implementing the agreed weighted, role-specific, governance-checked pipeline. The register records current user-approved proof and procurement scope.

This is a new project based on the accepted veri-ADAM proposal and its CO₂ experimental refinement. Its main experiment compares dependency-aware verification with a competent expiry-aware cache under identical acceptance rules. An efficiency advantage is a hypothesis, not an established result.

This checklist records concerns to resolve during implementation. Unchecked items are open decisions, not completed capabilities. The structure below is a proposed starting architecture; no previous project code has been copied.

## Connected-agent continuation

The user requested actor integration and explicitly deferred UI work. The [connected runtime contract](specs/weighted-agent-runtime.md) records the delivered actor-owned signing, authenticated temporal support, real weighted private proof, v2 ordinary/dependency checks, restricted export and signed durable advisory-authorization gate. Rules and actual local Qwen runs pass the valid and withholding cases. Historical retrieval is wired but awaits live Weaviate/MiniLM validation; SSH IPs are needed for Pi preparation. Physical acquisition, persistent device identities/secure peer services, full distributed recovery and matched confirmatory evaluation remain open. No novelty or hardware result is claimed.

## First implementation milestone

**Authenticated v2 contract/checker milestone:** `crates/records` now authenticates the corrected weighted engine's actual evidence, registered weights, full trace/reasoner references, eligible roles, ballots, final action and governance. It includes real hiding commitments and separately held authorized disclosures. The independent checker does not call producer rule functions. New encoding/adversarial tests and both rules/local-Qwen runs are recorded in [the v2 contract](specs/authenticated-decision-v2.md). Legacy signed bytes remain unchanged. This completes the local record/checker step of REQ-02/03/04, not the entire agent integration phase: actual worker/UI migration, authenticated history export, ZKP, runtime dependencies, durable acknowledgment/release and hardware validation remain open. No advisory is released and no proof is claimed.

**Corrective decision-stage implementation:** `crates/decision` now supplies shared weighted calibration/fusion, temporal Sensor logic, complete seven-field recommendations, role-specific ballots, strict crew formation/quorum, competing-recommendation selection, final assembly, repair/fallback and application governance. Source mapping and configurable-policy boundaries are in [the decision-stage contract](specs/adam-decision-stage.md). Tests cover both directions of legitimate fusion/crew disagreement and preserve selected confidence separately from crew support. Its unsigned stage is now wrapped by the authenticated v2 component above. Records await durable acknowledgment and cannot claim release. Existing agent workers/UI still use the legacy path until migration; semantic retrieval, ZKP, runtime dependencies and durable audit remain open.

The project now has a Rust workspace with `protocol`, `verification`, `reference`, and `cli` crates. It implements signed synthetic measurements, signed proposals/ballots, the provisional B1 checker, explicit current/historical outcomes, and adversarial fixtures. Read [README.md](README.md), [the provisional protocol](specs/protocol-v1.md), and [the next milestones](specs/next-milestones.md). The larger checklist below remains open wherever final research semantics or actual Pi validation are still required.

The second software milestone adds `cache` and `replay`: bounded B2 signature/decision caches, fresh time/checkpoint checks, a 31-step frozen synthetic workload, saved reports/manifests, and maintenance counters. The [B2 contract](specs/b2-cache.md) defines what is reused and what B3 must preserve. Final research baseline freezing, physical deployment, and confirmatory evaluation remain open.

The third milestone adds `dependencies`: a B3 reverse source/claim index using the same B2 cache and trusted snapshots. Both frozen workloads now compare B1/B2/B3/reference outcomes. The [B3 contract](specs/b3-dependencies.md) records the index mechanics, full-snapshot overhead, and neutral signature-count findings; no efficiency advantage or novelty is claimed. Alternative support and incremental fusion remain open.

The simulation milestone adds `simulation`: deterministic environmental signals, discrete virtual-time claim delivery, source/network faults, tampering/replay, independent scenario expectations, and a self-contained playback file. See [simulation version 1](specs/simulation-v1.md) and [the remaining project work](docs/project-status.md). This is not physical sensor validation, distributed consensus timing, or measured hardware performance.

The research review is reconciled in [review decisions](specs/review-decisions.md): current semantics are ADAM-inspired; strong indexed-cache equivalence is the working hypothesis; privacy format is a pre-collection decision; and Pi preparation/microbenchmarks proceed alongside design work.

## Language and implementation direction

Use **Rust for the protocol, verification, cache, dependency manager, replay engine, and command-line services**, and **Python for sensor acquisition, experiment orchestration, and analysis**.

Rust keeps the verification paths and B1–B3 comparisons in one implementation language and lets us selectively adapt the earlier evidence and expiry mechanisms. Python keeps hardware integration and data analysis separate from protocol authority. Python acquisition supplies measurements; the Rust service validates, encodes, and signs them. Python must not independently implement a second canonical signing format.

Keep the first implementation small: a Cargo workspace, local files and command-line tools, followed by bilateral transport. Defer a dashboard, blockchain anchoring, and proof-system selection until their requirements are established. Use established cryptographic libraries; do not implement cryptographic primitives ourselves.

- [x] Adopt Rust for the protocol/verification core and Python for later acquisition/orchestration/analysis.
- [ ] Choose and pin the Rust toolchain, Python environment, and dependencies after checking support on the actual Pis.
- [ ] Record actual Pi models, RAM, OS, and available sensors; do not assume the procurement table matches existing hardware.
- [ ] Check candidate cryptography and sensor libraries against their official documentation before choosing versions.

## Research comparison and baseline fairness

- [ ] Specify B2's cache key, retained metadata, expiry handling, invalidation rules, and revalidation granularity.
- [ ] Specify exactly what B3 adds: affected-claim indexing, partial recomputation, alternative support handling, or another explicit mechanism.
- [ ] Distinguish evidence expiry from dependency tracking. Individual expiry times do not automatically provide selective recomputation.
- [ ] Allow a legitimate cached-decision expiry derived from the earliest required evidence expiry. A single timestamp alone does not make B2 unfair.
- [ ] Give B2 competent conventional caching rather than deliberately withholding ordinary optimizations.
- [ ] Hold visibility, trust assumptions, verification checks, check times, and acceptance semantics constant between B2 and B3.
- [ ] Use an independent reference evaluator to judge acceptance; agreement between B2 and B3 alone is insufficient.
- [ ] Define comparable work counters, including invalidation, indexing, and cache-maintenance costs. Measure total resource costs as well as avoided checks.
- [ ] Include workloads where B3 may offer no benefit or cost more, alongside stable, unrelated-update, bursty, and shared-support workloads.
- [ ] Decide the research response if B2 and B3 are equivalent or B3 provides no practical improvement. Report the result without weakening B2.
- [ ] State a contribution beyond ordinary indexed caching, or explicitly report equivalence; do not treat the current reverse index as novelty by itself.
- [ ] Predeclare new neutral/adverse workload families before additional tuning; existing exposed corpora are development evidence only.

## Event format and cryptographic bindings

- [ ] Freeze versioned measurement, ballot, decision-bundle, and verifier-report schemas.
- [ ] Choose deterministic canonical serialization and publish exact signing bytes and cross-process test vectors.
- [ ] Define numeric units, precision, allowed ranges, and integer encoding before signing or proving values.
- [ ] Define domain-separated signing and hashing contexts for each record type.
- [ ] Bind sensor identity, role, sequence, event context, policy, and commitment references without circular hash definitions.
- [ ] Define how an existing sensor measurement is referenced by a later decision event; distinguish measurement identity from decision-event identity.
- [ ] Define nonce, sequence, duplicate-delivery, replay, restart, and persistence behavior.
- [ ] Specify key provisioning, node/role authorization, rotation, and revocation. Keep private keys out of datasets and source control.
- [ ] Define malformed-input rejection and record-size limits before accepting network bundles.

## Decision and quorum semantics

- [ ] Define the deterministic observation-to-classification rule, fusion weights, missing-data behavior, and threshold boundary behavior.
- [ ] Treat 800 ppm as an experimental advisory threshold, not a safety or health limit.
- [ ] Define uncertainty/tolerance-band behavior and keep physical measurement uncertainty separate from protocol validity.
- [ ] Specify what agents vote on, which roles may vote, and how a ballot binds to the exact event and proposal.
- [ ] Reject duplicate voter counting and define disagreement, abstention, missing ballots, and conflicting signed ballots.
- [ ] Reproduce the specified nominal three-voter majority and permitted two-voter degraded mode; never silently reduce quorum.
- [ ] Separate live bilateral tests from logical-role replay. Two devices do not demonstrate three independently controlled voters or Byzantine fault tolerance.
- [ ] Define any reasoner output's role. A signed output hash authenticates a reference; it does not prove inference correctness.
- [ ] Reproduce the paper-specified aggregation, independent role ballots, majority resolution, and governance for B0 before claiming ADAM compatibility; obtain original code/traces or document reconstruction limits.

## Currentness and dependency semantics

- [ ] Define trusted verifier time, timestamp interpretation, allowable clock uncertainty, and the exact TTL boundary.
- [ ] Separate UTC timestamps used across devices from local monotonic durations used for timing.
- [ ] Separate historical validity from current support: an expired advisory may remain a valid historical record.
- [ ] Define whether a newer observation replaces older support, and when a qualifying replacement requires a new claim or renewed verification.
- [ ] Track policy, aggregation configuration, role authorization, and commitment versions as dependencies where relevant.
- [ ] Define alternative support sets, insufficient support, and the effect of unrelated updates.
- [ ] Specify state transitions and invalidation from VERIFIED or RELEASED; distinguish rejected, expired, and unconfirmed outcomes.
- [ ] Prevent a dependency change between verification and advisory release from bypassing the required checks.
- [ ] Define behavior during sensor dropout, network partition, delayed delivery, and service restart.

## Optional private threshold proof

- [ ] Identify the external verifier and the operational reason it must not receive the exact reading.
- [ ] Decide the private-capable format before confirmatory collection; preserve public version 1 records without implying they can become private retroactively.
- [ ] Specify the public statement, private witness, bounded integer range, threshold, and event context.
- [ ] Select a hiding commitment with an explicit randomness policy; do not assume a digest of a low-range reading hides its value.
- [ ] Ensure the proof concerns the same committed value authenticated by the sensor record.
- [ ] Define a private record/disclosure format; an outside verifier must not receive the exact value through another bundle field.
- [ ] Enforce freshness and event binding alongside proof validity. An old valid proof does not establish current support.
- [ ] Test below-threshold values, out-of-range inputs, mismatched commitments, and substituted or replayed proofs.
- [ ] Document metadata leakage and compare disclosed signed values, signed binary assertions, and private proofs as different guarantees.
- [ ] Measure setup, witness preparation, proving, and verification separately. Retain B4 only if the privacy objective and measured Pi budget justify it.

## Audit records and claim wording

- [ ] Use “passes the specified verification checks” rather than treating a verified decision as physical truth.
- [ ] Describe ballot checking as authorization, binding, counting, and rule compliance; an authorized signer can still be mistaken or dishonest.
- [ ] State explicitly that valid signatures do not establish sensor accuracy or prevent authorized collusion.
- [ ] Use “tamper-evident” rather than “tamper-proof” for the proposed audit log.
- [ ] Specify how log alteration, deletion, truncation, and rollback would be detected, including any external checkpoint assumptions.
- [ ] Generate any private proof before the external verification that requires it; it is not merely a step after final verification.

## Acquisition and experimental validity

- [ ] Respect sensor data-ready behavior and record acquisition settings; do not label repeated stale values as new samples.
- [ ] Record placement, stabilization, calibration settings, clock offset, software versions, and thermal conditions.
- [ ] Use ordinary indoor sensing and controlled light transitions as described in the protocol; no deliberate gas enrichment.
- [ ] Label physical observations, synthetic attacks, and replay data separately.
- [ ] Use sessions as the unit for environmental replication; sequential observations are correlated.
- [ ] Freeze paired workloads and randomize or alternate benchmark conditions to limit drift and thermal bias.
- [ ] Count rejected, timed-out, and incomplete attempts as well as successful checks.
- [ ] Separate acquisition, transport, verification, proof, and storage timing.
- [ ] Freeze practical-effect thresholds, deadlines, primary metrics, sample sizes, and uncertainty treatment after the pilot and before held-out evaluation.
- [ ] Claim direct energy measurement only with suitable logged measurements and a defined idle adjustment.

## Proposed repository structure

```text
verifiable-ADAM/
├── BUILD_PLAN.md
├── README.md
├── Cargo.toml                  # Workspace and shared dependency configuration
├── Cargo.lock
├── rust-toolchain.toml
├── crates/
│   ├── protocol/               # Types, canonical encoding, IDs, hashes, signatures
│   ├── reference/              # Independent acceptance and decision evaluator
│   ├── verification/           # Production authentication and rule checks; B1
│   ├── cache/                  # Ordinary expiry-aware verification cache; B2
│   ├── dependencies/           # Dependency-aware verification strategy; B3
│   ├── replay/                 # Virtual time, workloads, mutations, work counters
│   └── cli/                    # Acquisition input, signing, checking, replay, peer service
├── python/
│   ├── pyproject.toml          # Acquisition, orchestration, analysis dependencies
│   └── src/veri_adam_tools/
│       ├── acquisition/
│       ├── experiments/
│       └── analysis/
├── specs/                      # Formats, rules, threat model, baseline contracts
├── tests/
│   ├── vectors/               # Canonical bytes and signature interoperability cases
│   └── corpus/                # Frozen normal, adversarial, and currentness cases
├── workloads/                 # Versioned replay definitions and manifests
├── configs/                   # Public node, policy, and experiment configuration
└── docs/                      # Hardware setup, experiment procedure, limitations
```

Add a separate private-proof crate only after selecting the statement, commitment, and proof system. Keep generated run outputs and private key storage outside committed fixtures; document their locations and retention rules.

The reference evaluator may share record types and standard cryptographic primitives, but must independently evaluate decision rules and currentness. It must not call B2/B3 invalidation or acceptance logic to produce its expected answers.

## Build order and exit checks

1. **Specification and fixtures:** settle canonical records, decision rules, threat boundaries, currentness, and B2/B3 contracts. Write hand-reviewed expected outcomes first.
2. **Signed vertical slice:** create one synthetic reading, sign it, bundle a small decision, independently verify it, and reject mutations and cross-event reuse.
3. **Replay and ordinary cache:** implement B1/B2 with deterministic time, explicit expiry, and the frozen reference corpus.
4. **Dependency-aware strategy:** implement B3, require reference-equivalent acceptance, and count its full maintenance and verification work.
5. **Two-Pi acquisition and transport:** connect real sensor input to the verified path, test both role placements, and preserve reproducible manifests.
6. **Pilot and optional proof:** establish hardware costs and practical-effect targets; add B4 only after its privacy and binding requirements are settled.
7. **Held-out evaluation:** freeze configuration and analysis, run paired experiments, and report failures and limitations alongside results.

Hardware inventory and sensor bring-up can proceed alongside the specification work. Benchmark conclusions must wait for equivalent baseline semantics and frozen evaluation rules.

## Source documents

- Accepted proposal supplied by the user: `/Users/greatadams/Downloads/veri_ADAM_Research_Proposal.docx`.
- Experimental refinement supplied by the user: `/Users/greatadams/Downloads/veri_ADAM_CO2_Experimental_Protocol.docx`.
- Project-story review in this chat, including baseline equivalence, verification wording, dependency changes, proof ordering, audit-log guarantees, and voting limitations.

The supplied document instructions are research inputs. This file does not record their proposed experiments, equipment, or milestones as already implemented or completed.
