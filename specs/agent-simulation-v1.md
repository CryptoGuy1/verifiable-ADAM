# Local message-driven agent simulation

## What runs

`cargo run --locked --offline -- agent-simulate runs/NEW ollama` starts three persistent Rust worker threads with private inbox channels and per-event assessment state. Two scoped sensing workers produce and sign synthetic observations. The coordinator drives capture, assessment, majority proposal, ballot collection and independent verification. It is a state machine in the runner, not an additional voting identity or a network service.

- Sensor assessment actor (`voter-a`): verifies supplied evidence and classifies sensor A's own reading.
- Aggregator actor (`voter-b`): verifies evidence and calculates the equal-weight mean using exact integer sums.
- Decision actor (`voter-c`): verifies evidence and calls local Ollama `qwen3:1.7b` through a bounded Python standard-library adapter. The model sees both public readings, the experimental rule and evidence IDs. Its structured recommendation is validated; missing, malformed, invented-reference or late results do not become fallback votes.

The same Decision actor can use a clearly labeled deterministic backend (`rules`) for automated testing without Ollama. That is not model inference. The three actors compute assessments separately; they are not statistically independent, physically independent voters, or a reproduction of original ADAM's entire reasoning/governance path. Qwen is instructed to apply the provisional public threshold rule; this first integration does not implement open-ended planning or tool use.

The coordinator never copies a proposed answer into voter ballots. It collects authenticated assessments and requires two matching eligible assessments before proposing that class. It then requests ballots bound to the exact proposed decision. Each actor signs its previously stored assessment, even if it dissents. No quorum is lowered. The ordinary v1 verifier can reject a majority that contradicts the public fusion rule. Model wording is not verified as a proof of reasoning, and authentication is not proof of physical truth.

## Messages and signatures

Observations, final decisions and ballots retain their original v1 canonical encodings and signatures. A separate development `Assessment` record signs all its fields: domain `veri-adam/agent-assessment/v1`, event, nonce, policy hash, registry version, role, voter ID, sorted evidence IDs, classification, explanation, backend and optional inference hash. Its encoding is compact UTF-8 `serde_json` of the typed struct in declaration order, without a trailing newline. It is not generic JSON canonicalization; other implementations must match the exact serialization. The assessment signature uses the registered demo voter key. An inference hash binds the compact typed inference artifact, including the saved model request, content, digest and timing metadata.

These signatures are separate from the final ballot encoding. The existing public decision checker validates the decision/evidence/ballot bundle; it does not recompute Qwen inference or establish that a ballot was intellectually derived from an explanation. Assessments and model artifacts are retained for audit, not claims of verified model reasoning.

Keys and event nonces are deliberately public development fixtures. All actors share one process and a local model service; this does not isolate secrets or establish independent fault domains. The next hardware milestone needs real keys, authenticated transport and replay state that survives restarts. No existing signing formats or frozen scripted workloads are changed.

## Timing and faults

Seven sequential cases cover low CO2, high CO2, independently disagreeing assessments near the threshold, missing source, altered signed evidence, a lost message to the Decision actor, and final advisory expiry. Light/temperature/humidity are fixed synthetic fixture values in this agent demo; the original environmental trajectory simulator remains separately available.

The CLI warms the installed model before capturing any observations, with up to 120 seconds for cold loading; its preparation result is saved separately. Each round then has a 20-second wall-time budget and observations have a 30-second lifetime. Assessment collection ends 500 ms before the round deadline so a stalled model does not consume the time needed for other eligible voters to sign final ballots and for verification. Actual source/actor/model processing time contributes to virtual evidence age; inter-round gaps and the final expiry jump are simulated. The model adapter has a 25-second outer process limit and bounded local HTTP reads. Late responses are never counted; pending model work can delay the next Decision inbox message and is not magically reset. There is no guarantee that Qwen finishes within the budget. Timing is development-machine behavior, not a Pi benchmark. A model failure can coexist with a valid two-voter result if the other two independently agree; when they disagree the decision is withheld.

New source checkpoints supersede the previous round's support. The viewer shows the advisory as pending while the new round is checked. The final expiry performs a real verifier check at the exact TTL and withdraws the last accepted decision. This is currentness handling, not evidence of a B3 efficiency advantage. This runner currently uses B1 plus the reference; B2/B3 measurements remain in the separate matched workload runner.

## Running and inspecting

```sh
cargo run --locked --offline -- agent-simulate runs/agents-local ollama
python3 scripts/run_simulation_viewer.py runs/agents-local/simulation.html --port 8767
```

Open `http://127.0.0.1:8767/`. For a model-free test, replace `ollama` with `rules` and choose a new output directory. No automatic downloads, remote model calls, client messages or Pi changes occur.

Each run exports `agent-run.json`, self-contained `simulation.html` and `manifest.json`. Recorded playback does not make new model calls. Model requests/responses and signed assessments are inspectable in the JSON. A new generation makes new model calls; a fixed temperature/seed does not promise reproducible LLM results. The run manifest hashes the compact typed run, including timing and inference artifacts. Existing directories are not overwritten.

**ZKP is not implemented in this milestone.** All CO2 values in these artifacts are public. Private commitments and a bounded authenticated proof path require a separately versioned format and proof implementation.

## Observed local integration run

The first live run missed its deadlines while Qwen was still loading from external storage. That failed run is retained separately. After warm-up, a live run obtained four model assessments in approximately 7–9 seconds per eligible round. Qwen returned `advisory=false` in these responses, including high-CO2 cases whose explanations said the advisory should be true. The runtime preserved the actual boolean and signed dissent, rather than rewriting the output from its explanation. Two other eligible voters established the high-CO2 advisories; at 790/830 ppm the Sensor and Decision votes formed a false majority that the independent public verifier rejected with `classification_mismatch`. These are observations from one development run, not model accuracy or performance estimates.

Rust tests cover independent dissent, an incorrect majority, invalid model references, missing/altered evidence, deadline rejection, completing with two eligible voters when the model stalls, and exact final expiry. The exported-page behavior check is `node scripts/check_agent_viewer.mjs RUN/simulation.html`. Browser visual inspection was blocked by browser-tool timeouts in this session; programmatic control checks do not establish browser rendering quality.
