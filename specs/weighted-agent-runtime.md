# Connected weighted agents, private claim and durable authorization

This delivery implements local integration for REQ-01–07. The user explicitly deferred UI changes; no viewer, browser asset or legacy signed fixture is modified. This is a workstation development pipeline with synthetic evidence, not Pi deployment, industrial actuation, exact historical ADAM reproduction or evidence of research novelty.

## Run and inspect

```sh
cargo run --locked --offline -- adam-agent-run runs/NEW rules
cargo run --locked --offline -- adam-agent-run runs/OTHER ollama
# Historical semantic retrieval additionally requires the local service below:
cargo run --locked --offline -- adam-agent-run runs/HISTORY ollama-history
```

The six cases are weighted normal, weighted anomaly, temporal spike, degraded crew, missing Decision ballot and tampered source. Each exports a complete round, actual actor messages, source evidence, stage output, signed record, authorized disclosure, independent B1/B2/B3 results, a private proof/report where possible, a recording acknowledgment and either an advisory authorization or a withholding result. Invalid source evidence remains inspectable even when no decision is produced. These are recorded local computations, not an animation.

Use `private-export/` for the restricted auditor package. It contains signed commitment headers, ballots/proposals, the actual proof, public request/trust inputs, signed authorization receipts when issued, and public audit entries/checkpoints. It excludes raw readings/openings, full round/stage files, private model text and semantic history. Everything outside that folder, including `agent-run.json`, `round.json` and `authorized-disclosure.json`, is authorized-internal material. Public metadata/classification/confidence/action still reveal information; no claim of complete metadata privacy is made.

```sh
cargo run --locked --offline -- adam-private-check \
  runs/OTHER/private-export/weighted-normal/record.json \
  runs/OTHER/private-export/weighted-normal/private-proof.json \
  runs/OTHER/private-export/trust.json \
  runs/OTHER/private-export/weighted-normal/request.json
cargo run --locked --offline -- adam-audit-check \
  runs/OTHER/audit runs/OTHER/trust.json runs/OTHER/latest-checkpoint.json
```

Saved requests use a synthetic epoch and trusted replay time. A production verifier must provide its own fresh challenge, time and authenticated heads; these saved files do not establish that a room is currently above a threshold.

## Actual role execution

`agents::weighted::Runtime` moves every source/voter key into a persistent named worker with a message inbox. The coordinator retains only its own key. Distinct local keys do not mean independent devices or Byzantine tolerance.

1. Source workers commit and sign observations **before** inference. All receiving actors check signatures, opening/context/calibration linkage, ordinal coverage and freshness. Invalid evidence stops before the model or voting.
2. The Sensor worker calculates its threshold/temporal assessment; the Aggregator worker calculates registered weighted fusion. Previous triggering-source observations are exported as explicit authenticated historical support, rather than trusting an invisible producer baseline. Bounded six-reading history remains local session state, not restart-safe acquisition provenance.
3. The Decision worker owns the reasoner and handles initial inference and any permitted repair. It retains the actual raw responses/errors/metadata. Its ballot refuses a changed recommendation or inference transcript. The existing decision component orchestrates the same paper-derived rule functions and event budget; it does not acquire voter secrets.
4. Each worker signs its own assessment class bound to the exact proposal. Explicit message loss preserves the formed eligible set and quorum. Conflicting proposals for the same context are refused within the worker session. A timed-out reply cannot authorize an advisory, and an unresponsive reasoner cannot block the coordinator indefinitely.
5. The independent checker recomputes the full disclosed decision path. B2 and B3 check the same actual record and must agree with B1. Legitimate crew disagreement with fusion remains allowed. The sources are the client refinement §6–10 and original ADAM §3.2.3–3.5, with the policy choices already recorded in `adam-decision-stage.md`.

New agent runs use fresh OS-random event nonces and ephemeral OS-random keys. Existing protocol-v1 and earlier v2 signed bytes remain verifiable. The new `weighted-fusion-two-committed-co2/v1` statement identifier is accepted alongside the earlier reserved identifier by the public-disclosure checker; the private checker requires the implemented identifier. Ballots bind the chosen identifier before proof generation, so attaching a proof does not rewrite a signed proposal.

## Real weighted private predicate

This is a genuine Bulletproofs range proof using the pinned library, not a hidden UI value or signed binary assertion. It proves **both** hidden fusion readings in 0..40000 ppm and the weighted classification against the policy threshold, bound to their signed Ristretto Pedersen commitments and the exact proposal ID. See the [library API](https://docs.rs/bulletproofs/latest/bulletproofs/struct.RangeProof.html).

For positive registered integer weights `wA,wB`, define `N=wA*mA+wB*mB`, `W=wA+wB`, and `U=40000*W`. For ANOMALY, `D=N-threshold*W`; for NORMAL, `D=threshold*W-1-N`. The proof covers:

```text
mA, 40000-mA, mB, 40000-mB, D, U-D, 0, 0
```

All eight are proved nonnegative 64-bit integers. Their commitments are independently derived from the authenticated input commitments and public weights/threshold; the proof cannot substitute its own commitments. Two fixed zero entries pad aggregation to eight. With `W<=1,000,000`, numerical bounds stay far below scalar modulus and u64 capacity. The transcript binds the full proposal ID, which binds event/session/location/nonce, support IDs, policy/calibration/registry, classifications and action. Generator initialization is cached; timing on the Pis is not measured.

The fixed development readings are public in the fixture definitions and can therefore be inferred from scenario names/source code. These scenarios validate proof binding and export filtering, not secrecy of already published synthetic values. Physical privacy evaluation requires unpublished inputs and a metadata/disclosure audit.

The private checker also authenticates ballots and independently counts their fixed eligible quorum. Its result explicitly says `private_fusion_verified`, while **`full_decision_processing_verified=false`**: it does not privately prove the Sensor's temporal rule, Qwen execution, hidden recommendation text or every numerical governance check. It does not force crew classification to equal fusion. Internal full-disclosure verification and durable recording are still required for advisory authorization. This proof is the selected private-claim pilot integrated with actual decisions; it is not a claim that the entire ADAM process has been proved in zero knowledge.

Tests include both threshold sides, equality, 0/40000 bounds, false predicates, wrong openings, malformed/corrupted proofs, event/commitment substitution, weight changes, expiry/replacement and revocation. No physical sensor accuracy follows from a valid proof.

## Dependencies and competent ordinary reuse

`records::reuse` adds bounded v2 B2 and B3 paths alongside the preserved legacy libraries. Every query hashes all candidate content, signatures, disclosure, configuration and context before reuse. Even on a hit, B2 checks time and the exact supporting source heads. B3 uses that same ordinary cache and adds source-to-claim links and explicit invalidations. Authenticated source updates, expiration and policy/calibration/registry/role configuration changes invalidate affected registered claims; unrelated source updates preserve them.

Runtime applies source-head updates only after validating source signatures/openings in its serial local input path. This is not authenticated remote checkpoint distribution. Observations remain event-bound; no unsupported cross-event observation reuse is introduced. Current and historical checks remain distinct.

The bounded expiry scan, configuration hashing, full snapshot scan, reverse-link maintenance, full checks, successful signatures and cache hits are counted. These development counters are not total CPU/energy measurements or signature-attempt microbenchmarks. B2/B3 equivalence tests cover unrelated/changed heads, exact expiry, configuration changes and tampered cached inputs. No B3 efficiency advantage is asserted. Alternative support, incremental fusion, secure remote heads and restart-safe dependency restoration remain open.

## Durable recording and advisory authorization

`records::audit::Journal` holds an exclusive OS file lock, appends individually Ed25519-signed, sequenced hash-linked entries, fsyncs the log, atomically replaces a signed checkpoint, then fsyncs its directory. Storage errors poison the writer until restart/recovery. The APIs use the [standard filesystem locking and synchronization operations](https://doc.rust-lang.org/std/fs/struct.File.html).

The runner records valid, withheld and rejected attempts and dependency invalidations. Before returning an authorization, the release gate independently rechecks current evidence/quorum/policy and deadline, persists a release reservation, obtains its durable checkpoint acknowledgment, then checks freshness/deadline again. The signed receipt binds the exact advisory/proposal, reservation and checkpoint. The receipt is an authorization artifact; no physical device is actuated. Signed pre-recording stage reports retain their original false release/proof flags; later evidence is carried separately, rather than changing signed history.

Reservations are keyed by session/event and reloaded on reopen. Duplicate authorization is refused even after a restart. A crash after reservation can lose delivery, but cannot safely cause automatic reauthorization: this provides **at-most-once authorization, not exactly-once delivery**. A late acknowledgment consumes the reservation and withholds the advisory. `Journal::recover` explicitly re-checks signatures and the complete chain before acknowledging a complete uncheckpointed crash tail; it does not release recovered reservations. Incomplete/corrupt tails fail closed.

Independent audit checks detect signature corruption, sequence/link changes and truncation. Detecting rollback of the **whole log plus an older valid checkpoint** requires an independently retained latest checkpoint. The exported pin illustrates that interface; a pin stored only alongside an attacker-controlled log does not provide independent rollback protection. Signer compromise, protected device storage, long-term registry/key transitions and remote anchoring require separate deployment work. Blockchain remains optional and absent.

An advisory's historical authorization does not make it current forever. Runtime invalidations and current verification determine continued support; authorization files are historical receipts.

## Historical semantic retrieval

`infra/history.compose.yml` provisions an isolated localhost-only Weaviate 1.30.2 and `text2vec-transformers` all-MiniLM-L6-v2 service, matching the manuscript's reference-rerun stack. It uses a separate `VerifiableAdamEventTrace` collection and persistent Docker volume. [Official setup documentation](https://docs.weaviate.io/weaviate/model-providers/transformers/embeddings) lists this model image. No production/other-project collection is modified.

```sh
docker compose -f infra/history.compose.yml up -d
python3 scripts/history_store.py init
cargo run --locked --offline -- adam-agent-run runs/NEW-HISTORY ollama-history
```

Historical entries now carry their complete signed decision/disclosure evidence and a signed durable-resolution checkpoint. A separate Rust verifier pins the registry to the receiving actor, rechecks evidence/quorum/policy and rejects resolution at or after the current event. Returned database summaries must exactly match that verified evidence. Full historical envelopes stay out of model text/response metadata to prevent recursive trace duplication; authenticated proposal references and summaries are retained. The first unverified-history run is preserved as diagnostic only. The verifier uses the active Rust CLI executable, supplied to the local Python adapter at launch; it does not trust a boolean verification claim from the database. Authorized run files retain the original records, disclosures, log entries and checkpoints needed to reproduce these historical checks.

The adapter constructs a semantic `nearText` query from fused concentration, retrieves up to five resolved events, and rejects current/future records. Resolution time must precede the event start. Synthetic/replay/physical modes and experiment scopes remain separate; local development scope follows the ephemeral registry to avoid cross-run data leakage. Full historical numerical/recommendation provenance is supplied in the authorized model context and retained in signed response metadata. In `ollama-history` mode, the enabled history store must acknowledge accepted-event recording before advisory authorization; a missing acknowledgment withholds it. Retrieval failure is an explicit failed reasoner attempt and may invoke the already specified degraded fallback within budget, never a fake successful empty retrieval.

The query/filter/authentication boundary has unit tests. Live validation on 26 November 2026 succeeded in `runs/weighted-agents-qwen-authenticated-history`: actual Qwen requests received 0, 1, 2, 3 and 4 authenticated prior events, with no repair/fallback. Four decisions obtained both store acknowledgments and durable authorization; the missing-vote and tampered-source cases stayed withheld. `scripts/check_history_store_failure.py` separately exercised real Qwen/retrieval with only history writes returning HTTP 503: all four otherwise valid decisions were withheld from authorization. The isolated failure proxy was stopped after the test; the normal localhost retrieval service remains running. `ollama` without the suffix is explicitly **without semantic retrieval**, even when the rest of the pipeline succeeds. It must not be presented as complete historical ADAM compatibility. Query text/schema and Qwen remain declared CO2 reconstruction choices, not recovered historical code.

## Acceptance evidence, 26 November 2026

- `runs/weighted-agents-qwen-authenticated-history/integration-validation.json`: real Qwen history progression 0–4; independently checked weighted private claims and durable authorizations; private export field audit.
- `runs/weighted-agents-required-store-failure-durable/required-store-failure-test.json`: four otherwise valid Qwen decisions, zero authorizations when the enabled semantic store refuses writes, with every recording failure in the durable signed journal.
- The original `runs/weighted-agents-qwen-history/` is explicitly diagnostic because it preceded historical authentication; original signed records are preserved.
- Rust tests, Python history-boundary tests and workspace lint/format checks cover the implementation. These are software-development results, not physical trials, confirmatory statistics or a B3 advantage.

## Remaining milestones

- Extend the validated local retrieval setup to persistent device identities and authenticated peer services, with explicitly chosen experiment scopes and protected historical trust-version retention.
- Provision persistent device keys, fresh peer challenges, acquisition sequence/replay state and trusted configuration/head transport. Clean journal reopen is not a complete distributed restart protocol.
- Access both Pis, install/validate the pinned Rust environment, add actual SCD40/BH1750 acquisition and clearly labelled replay channels, and validate clocks/power/thermal conditions. `.local` names did not resolve during this delivery; SSH IP addresses were requested. No hardware has been changed.
- Establish appropriate physical calibration and collect the approved pilot; preregister/freeze held-out workload and evaluation choices. Run matched B0/B1/B2/B3/private disclosure comparisons at equal stated assurance, including adverse cases and full costs. Novelty, speedup, physical accuracy and Pi proof feasibility remain unproven.
- UI changes remain deferred at the user's request. The legacy browser demo is not the connected pipeline described here.
