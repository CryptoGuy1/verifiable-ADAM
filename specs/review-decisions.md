# Research review decisions and open gates

This register reconciles the supplied review with the repository and the locally available ADAM manuscript. It changes the development plan and claim boundaries; it does not change signed records, implement a private proof, authorize purchases, or record client agreement.

## B2 and B3 equivalence is the working hypothesis

The current B3 is a conventional reverse dependency index over B2's authenticated decision certificates. A competent ordinary cache can use the same mechanism. Equivalence with that stronger indexed baseline is the working hypothesis; an advantage or novel verification property must be demonstrated, not inferred from the name B3.

Per-reading expiry alone does not specify reverse indexing or incremental maintenance, so algorithm details still matter. The relevant comparator must match support bindings, invalidation, expiry, cached signatures, policy/registry handling, input visibility, and time/checkpoint assumptions. B2 versus B3 as currently implemented isolates indexed support matching versus direct support matching. It does not establish superiority over ordinary indexed caching.

Keep the present software as implementation and measurement infrastructure. Before further optimization, state the additional property being investigated, if any. If the strongest ordinary cache and B3 share the same operations and guarantees, document equivalence instead of copying the implementation under another name and treating that as independent evidence.

## ADAM inspired until reproduction is demonstrated

The local manuscript `ADAM_EAAI-3.pdf`, Section 3.2.3, specifies fixed inverse-variance fusion derived from residuals against an NDIR methane reference. Its reported four-node normalized weights are 0.254, 0.239, 0.247, and 0.260. Missing contributors are excluded and the remaining weights renormalized. Weighted dispersion is also passed to the reasoner.

Sections 3.3.1–3.3.4 specify independently computed Sensor, Aggregator, and Decision ballots, a non-voting Coordinator, nominal/degraded eligibility, majority resolution, and final alignment of classification, severity, and action. In particular, the final classification can be resolved by the crew rather than simply being forced to equal the fused-threshold prediction.

Our equal-weight two-channel rule and fixture ballots are ADAM-inspired development semantics. Merely replacing equal weights would not reproduce the original workflow. B0 requires a traceable implementation of the paper's roles, aggregation, context, reasoner interface, resolution, and policy/governance behavior. Request original code, traces, calibration inputs, and reasoner artifacts where needed. Without those, identify the result as a reconstruction of specified paper behavior and list what cannot be reproduced.

The paper's methane-sensor calibration weights are not calibration for our two CO₂ sensors. Any CO₂ weighting must be separately justified; do not copy those numerical weights into physical CO₂ experiments.

## Decide the private record format before final data collection

Version 1 remains a public-reading development format. Its signed measurement contains `co2_ppm`, and its measurement hash covers that value. It must not be described as private or used unchanged as the external disclosure format for B4.

Before confirmatory acquisition, decide whether B4 is required. If it is, specify a new versioned private-capable record: a hiding commitment with fresh randomness, authenticated event/policy/source bindings, explicit public/private fields, authorized access to openings, a bounded threshold statement, and a complete disclosure inventory. The outside verifier must not receive raw readings or a guessable deterministic digest of them through another record ID, signature message, manifest, or bundle field. An extra commitment field does not hide a plaintext value that remains publicly accessible elsewhere.

Do not reserve an undefined commitment field and assume the privacy problem is solved. First define its semantics and binding. Keep the existing public format and its test vectors immutable; add an explicit new protocol version rather than silently reinterpreting bytes.

A format change does not inherently invalidate existing signed public records. Preserve the versioned encoder/verifier and original bytes so they remain auditable under their original semantics. However, data already disclosed cannot become confidential retroactively, and old public-only data cannot automatically satisfy a later private-proof collection design. A paired B1/B4 study must collect the necessary private-capable evidence after the format decision.

## Procurement and staged sensing need explicit scope

Treat CO₂ cost and lead time as actual constraints. The current purchase estimates do not support calling the sensors inexpensive for this project. Confirm a supplier quotation and budget before committing to dates; no order is authorized by a reviewer's procurement suggestion.

Ask the client whether temperature/light bring-up is acceptable as a development phase while retaining real CO₂ validation for the final study. Do not present it as satisfying CO₂ acquisition, change the accepted experimental scope silently, or fabricate CO₂ fields. If a lower-cost final modality is proposed, obtain an explicit protocol refinement before using it for confirmatory results.

## Predeclare adverse workloads before the next optimization

Existing simulations and frozen corpora are exposed development data, not held-out evaluation. Their earlier results cannot be retroactively preregistered. Before selecting a favorable load or tuning the index further, record and freeze a new workload plan including:

- Stable claims already served by warm B2 caches, with no redundant signatures to remove.
- Large unchanged or unrelated head snapshots that make change detection expensive.
- High fanout or correlated changes that dirty nearly every indexed claim.
- Low query-to-update ratios that pay maintenance for claims rarely checked.
- Small capacities and frequent eviction, restart, policy changes, or role revocation.
- Short TTLs, unavailable sources, late delivery, and rejected claims.
- Equivalent ordinary indexed caching, with the same trust and visibility as B3.

Declare expected correctness and the cost hypothesis for each family, including neutral/adverse outcomes. Preserve all attempts and failures. Freeze confirmatory seeds, sampling units, scoring, and practical-effect thresholds before held-out testing; define numerical resource thresholds after a separate pilot.

## Parallel work starts with cost measurement

Pi preparation and primitive microbenchmarks need not wait for the final fusion or privacy format. Start host setup, release builds, power/cooling checks, and timing of Ed25519 signing and verification while the design questions are resolved.

Measure a single primitive, a complete cold B1 check, warm B2, and B3 separately. Include serialization, hashing, snapshot reconciliation, index maintenance, and eviction in the full-path measurements. Record sample sizes, warm-up, timing overhead, CPU/thermal conditions, governor, throttling flags, and repeated sessions. Distinguish host processing time from simulated deadlines and physical sensing cadence.

Use the actual proposed check rate, active-claim count, and deadline budget to judge practical value. A tiny primitive saving may be immaterial at low rates, while checking many claims could change the budget. Do not select an unrealistic scale merely to favor the index. First establish whether a typical full check is already cheap enough on each Pi; use that pilot to decide how much optimization is justified.

## Questions to resolve with the client

1. What verification property or algorithmic contribution is required beyond a competent signed, expiry-aware indexed cache? Is equivalence an acceptable reported finding?
2. Must the final evaluation reproduce original ADAM behavior, and can its code, traces, calibration inputs, and reasoner artifacts be supplied?
3. Is B4 required in the final study, who is the restricted verifier, and what exact information may that verifier see?
4. Is temperature/light development acceptable while CO₂ procurement proceeds, and what budget and collection schedule are feasible?

These questions are recorded for review. No message has been sent to the client and no answers are assumed.
