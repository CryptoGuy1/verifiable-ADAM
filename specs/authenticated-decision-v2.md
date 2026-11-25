# Authenticated weighted ADAM record, version 2

Implemented 25 November 2026 for REQ-02/03/04, with commitment groundwork for REQ-06. Sources: client CO2 refinement §6–10 (signed evidence, complete bundles, independent checking); original ADAM §3.2.3–3.5 (weighted fusion, role assessment, eligible quorum, recommendation selection, governance); explicit user choices in the traceability register. This is the next contract/checker component of the integrated-local plan, not completion of worker/UI migration or the entire research system.

## What runs

`crates/records` wraps the corrected `decision` engine's actual output in source-signed observations, individually signed role ballots and a coordinator-signed full proposal. `records::checker` independently recomputes the numerical, temporal, recommendation-selection, role, quorum, assembly and governance rules. It shares types, the encoding and standard cryptography; it does **not** call the producer's decision, fusion, quorum, selection, assembly or governance functions or the legacy B2/B3 paths.

```sh
cargo run --locked --offline -- adam-auth-run runs/adam-authenticated-new rules
# Same path with the locally installed model; no download:
cargo run --locked --offline -- adam-auth-run runs/adam-authenticated-qwen-new ollama

cargo run --locked --offline -- adam-check \
  runs/adam-authenticated-new/weighted-normal/record.json \
  runs/adam-authenticated-new/weighted-normal/authorized-disclosure.json \
  runs/adam-authenticated-new/trust.json \
  runs/adam-authenticated-new/weighted-normal/request.json
```

The new directory is never overwritten. The runner also saves the synthetic calibration inputs and hashes its output files in the run manifest. `adam-check` exits 0 for independently verified decision processing, 1 for rejection/unconfirmed, 2 for input/operational failure. `adam-auth-run` exits 0 when it has generated the run and reports, including withheld cases; inspect each verification result rather than treating command success as every decision being accepted. All observations/calibration in these cases are synthetic. `rules` is explicitly a configured rule response, not model inference. `ollama` records real responses, repair/fallback attempts and metadata using the existing seven-field adapter.

## Records and trust

- **Observation:** event/session/location/challenge/start time; source identity; purpose (current fusion or historical temporal baseline); ordinal; sample sequence when present; capture time; data kind; registered calibration hash; commitment scheme and point. Each is signed by its registered source. Current fusion samples require a positive sequence. The older `PastReading` type lacks sequence/provenance: baseline attestations consequently say `historical_baseline` and omit sequence. These are authenticated historical *claims*, not a complete physical acquisition format. Additional environmental channels/acquisition metadata remain a later hardware integration task.
- **Proposal:** protocol 2; complete context; coordinator; full registry/policy/calibration hashes; exact sorted unique observation IDs; all registered fixed integer weights; hiding references to the full event/stage trace and selected reasoner output; eligible roles and quorum rule/version; separate fused class and selected recommendation summary; complete final classification/confidence/severity/action/review summary; degraded disclosure; policy verdict/reasons; stage status; completion/deadline; reserved private-statement identifier. Reasoning/factors and full backend attempt provenance are bound in the authorized trace rather than exposed as public text.
- **Ballot:** proposal ID, registered voter identity, explicit Sensor/Aggregator/Decision role and class. Each signature binds the exact proposal (including event/challenge, support, policy, output and action). Coordinator is nonvoting. Ballots are part of the same record; no signature or quorum is inferred merely from a summary count.
- **Authorized disclosure:** original event input, complete seven-field recommendations and stage/attempt trace, each CO2 value and commitment opening, and independent random salts for the trace/reasoner references. This file contains private material. It is not an external-auditor export.
- **Trust/request:** independently loaded registry, policy, calibration and fixed signer/role assignments; verifier-chosen event challenge and time; current/historical mode; trusted source heads. The claimant cannot supply a replacement policy inside its record and have it accepted. Development-generated `trust.json` and `request.json` are local fixtures, not authenticated peer distribution.

Signers have distinct keys/role identities and declared node labels, but local co-location is explicit. Two Pis would still not establish three independent physical voters. The local runner provisions fresh ephemeral Ed25519 keys from OS randomness and never writes secret keys; these are not production device provisioning. A durable challenge/consumed-action store, restart protection and secure peer transport are still absent.

All observation headers are **event-bound**. This version does not silently allow cross-event reuse: both the source and each role attest this event's context. The immutable-observation/event-claim split remains required before extending to long-lived cross-event caches. A source may legitimately sign an inaccurate number; verification cannot establish physical truth.

## Commitment and disclosure decisions

CO2 commitments use `bulletproofs` **5.0.0** `PedersenGens::default()` and `curve25519-dalek` **4.1.3** Ristretto scalars, isolated from Ed25519's newer curve dependency. The signed scheme identifier is `ristretto255-pedersen-bulletproofs5/v1`. The generators are the library defaults: Ristretto basepoint for the value and SHA3-512 hash-to-group of its compressed encoding for blinding. See [the library source](https://docs.rs/bulletproofs/latest/src/bulletproofs/generators.rs.html). Fresh 64-byte OS random input is reduced to a blinding scalar per attestation; openings use canonical 32-byte little-endian scalar encoding, lowercase hexadecimal. Compressed points are 32-byte lowercase hexadecimal. Values are integers 0..40000 ppm; this public checker verifies the disclosed opening and bound. This is a real hiding commitment, **not a range/threshold proof**.

Trace and reasoner references are SHA-256 over domain-separated encodings of `(fresh random 32-byte salt, typed content)`. Their salts and content appear only in the disclosure file. The public proposal never includes the old deterministic hashes of raw readings or private reasoner text. A valid public record exposes metadata, classification, confidence, severity, action and review status; those have their own inference/disclosure implications. It does not expose raw ppm, blindings, reasoning/factors, original guessable evidence labels or unsalted private-output hashes. There is no claim of complete metadata confidentiality.

`weighted-fusion-two-committed-co2/v1:reserved-unimplemented` reserves the selected future statement in the signed proposal. The present checker accepts only public-disclosure verification; passing `-` in place of disclosure returns **unconfirmed**, with `private_proof_verified=false`. No arbitrary proof bytes, fake success flag, or unverified threshold assertion can complete that path. The later proof must bind both committed readings, registered weights, threshold, range and exact event/proposal context. It proves fusion, not Qwen reasoning or the final majority's correctness. The private verifier/export will require its own implementation and adversarial tests; this record file alone is not a functioning private advisory package.

## Canonical signing and identifiers

Legacy protocol-v1 code, signatures and frozen fixtures are unchanged. New types use a separate version-2 typed-tree binary encoding. JSON is transport only. Before encoding, the Rust typed value is converted with pinned `serde_json`; objects are sorted by UTF-8 key bytes. For supported wire values:

| Value | Encoding |
| --- | --- |
| Domain prefix | u32 big-endian byte length, then UTF-8 `veri-adam/{kind}/v2` |
| null / false / true | single tag byte 0 / 1 / 2 |
| nonnegative integer | tag 3, u64 big-endian |
| negative integer | tag 4, i64 two's-complement big-endian |
| floating number | tag 5, IEEE-754 binary64 bits big-endian; typed validations require finite decision numbers |
| string | tag 6, u32 big-endian UTF-8 length, UTF-8 bytes |
| array | tag 7, u32 big-endian element count, recursively encoded elements in order |
| object | tag 8, u32 big-endian entry count, sorted entries: key length/key bytes then encoded value |

Kinds are `observation`, `proposal`, `ballot`, `registry`, `application-policy`, `calibration`, `trace-commitment`, `reasoner-commitment` (and explicitly separate internal comparison/test domains). SHA-256 of the bytes is the lowercase hexadecimal identifier. Ed25519 signs those exact bytes, excludes the signature field, and verifies with the registered role key. Incoming JSON is deserialized to its typed schema before encoding; foreign implementations must respect integer/float field types rather than sign arbitrary source JSON. Arrays remain order-sensitive. Proposal observation IDs alone are sorted/unique. CLI files are limited to 1 MiB; checker also bounds evidence, histories, competitors and ballot counts.

Independent encoding vector, kind `vector`, typed JSON `{"a":1,"b":2}`:

```text
00000013766572692d6164616d2f766563746f722f7632080000000200000001610300000000000000010000000162030000000000000002
```

The executable vector fixes domain prefix, object ordering, length and integer representation. No existing acquisition dataset is rewritten into this version.

## Independently checked behaviour

The checker verifies source, coordinator and ballot signatures and authorization; all context/configuration/content/opening bindings; weights and exact integer threshold arithmetic; displayed fusion/dispersion and quantization values; authenticated temporal baseline and Sensor rule; recommendation schema, reported repair/fallback consistency and severity/time selection; required crew formation, role-specific assessments and fixed eligible majority; selected confidence preservation, final assembly and explicit policy checks. It authenticates a role's report of its model invocation, not a proof of model execution or semantic accuracy.

Only explicit authenticated temporal support is accepted across this boundary. An existing stateful producer with a hidden internal history receives **unconfirmed** until it exports that history as source-authenticated support. The local runner uses isolated cases, including an explicit temporal-baseline case; it does not claim stateful worker integration is complete. Historical baselines are deliberately past context, not current fusion readings subject to the same TTL.

Freshness is measured against the trusted request time, not claimant wall time. Completion must precede both event deadline and fusion-support expiry. A current check also requires each trusted head to match and time strictly before expiry; replacement or expiry is unconfirmed. Historical checks retain valid past processing independently of current heads. Same-event rechecking is allowed; preventing duplicate action execution requires the later durable replay/release store. Authenticated majority disagreement with fusion is legitimate; altered ballots or false final summaries are rejected.

Successful outcomes are `verified_current_awaiting_recording` or `verified_historical_awaiting_recording`. No-quorum, unavailable disclosure, hidden history, stale/replaced support are unconfirmed; malformed/authentication/rule mismatches are rejected. **Every report has `action_released=false` and `private_proof_verified=false`.** Durable recording acknowledgment must still be implemented before advisory release.

## Acceptance evidence and next integration

`crates/records/tests/authentication.rs` covers weighted acceptance, both directions of fusion/majority disagreement, nominal/degraded missing votes, tampering/openings, event/ballot replay, forged final/weight/policy summaries even when re-signed, role impersonation/duplicate ballots, trusted-policy/registry changes, exact-expiry/historical/replacement handling, absent proof/disclosure, private-text binding/leak checks, fresh commitments, false numeric/reasoner traces, hidden history, repair/fallback, competing recommendation provenance and deadline rejection. The CLI test checks five cases, saved independent re-verification, no-disclosure withholding and refusal to overwrite runs.

Next: migrate the actual message-driven role workers and demo to this contract, including explicit authenticated history export and formation provenance; then implement the selected real private proof, B3 decision dependencies and durable governance/release. Semantic retrieval, actual Pi acquisition/device keys/transport and matched B2/B3 novelty measurements remain open. This delivery does not modify the legacy browser demo or claim those components are now integrated.
