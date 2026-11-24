# B2 ordinary expiry aware cache

B2 reuses authenticated facts and previously checked decision processing while preserving B1's currentness and acceptance semantics. It is the conventional comparator for future B3 work, not the proposed novelty. No sensor procurement change is implied by this software milestone.

## What is retained

The signature cache stores only successful verification of a tuple containing the complete canonical signed bytes, signature, and public key. Failure is never cached. The same key cannot authenticate different signed bytes or an altered signature. This is a mathematical authentication fact, not proof that a role remains authorized or a reading is fresh.

The decision cache stores a certificate only after all structural, configuration, authorization, signature, classification, and ballot checks have passed. The certificate contains the decision ID, earliest required expiry, latest capture time, quorum sufficiency, and required source-to-measurement bindings. It can represent an expired or unconfirmed result whose underlying processing is valid; it never preserves the old acceptance outcome as a permanent answer.

The decision key binds the complete signed decision, all signed measurements and ballots, full policy contents, full registry contents, and expected event/nonce. Registry contents are included even if an operator fails to bump a version. Measurement, ballot, and registry-member arrays are normalized by identity before key construction; harmless transport reordering is eligible for reuse. Policy lists retain their order because it affects the signed policy hash. Record counts, duplicate entries, all payload fields, signatures, and role flags remain in the key.

Keys are SHA-256 digests of domain-separated, typed JSON tuples. These are internal cache identifiers, separate from the protocol's binary signing format. Reconstructing a decision key requires serializing and hashing its full contents; that cost must not be ignored in comparisons.

## What happens on each request

1. Construct the content-bound key and look up a decision certificate.
2. On a miss, run the B1 rule evaluator with a signature-cache backend. It still checks source sets, configuration, binding, classification, quorum, time, and heads. Unchanged signatures can be reused even when a changed proposal requires rule recomputation.
3. On a hit, reject a check time earlier than any capture. Preserve insufficient-quorum outcomes. Historical checks verify past processing without asserting present-day support. Current checks compare the trusted time with the support horizon, then compare every required source's trusted head with the certificate.
4. Return the resulting outcome and diagnostic work counts. Neither a cached historical success nor a previous current success overrides expiry or a changed head.

Required source heads are checked directly at request time. Unrelated source updates do not invalidate a reusable result. A changed required head with the old bundle produces UNCONFIRMED; presenting a newly signed, consistently bound bundle requires a new decision check. A policy or registry change cannot use the previous configuration's certificate. No global invalidation stamp is used.

An identical signed statement need not have its signature checked again merely because time advanced. B2 revalidates currentness separately. In particular, an expired statement stays expired as a current claim even though its historical signatures remain valid.

## Capacity and restart

Both caches use bounded deterministic FIFO eviction, with default capacities of 1,024 signature entries and 256 decision entries. Capacities are counts, not RAM limits. A zero capacity disables retention in that cache. Eviction changes work, not acceptance. Explicit `clear()` or a process restart discards cached work. There is no persisted cache import format or mechanism for trusting previously serialized certificates.

Replay can move virtual time backwards to exercise the contract. Future captures are still rejected even if a later-time check was previously cached. A trusted earlier time at or after capture can make evidence fresh under the provisional B1 contract; this is not protection against live clock rollback. Trusted monotonic scheduling and durable anti-replay/action state remain separate implementation work.

## Work accounting

| Counter | Meaning |
| --- | --- |
| `signature_checks` | Actual signature-verification attempts, including failures; reused signatures do not increment it |
| `signature_cache_hits` | Successful per-record signature-cache lookups; a complete decision hit does not visit this cache |
| `measurements_evaluated`, `ballots_evaluated` | Records fully processed by the B1 rule path |
| `decision_lookups`, `decision_hits` | Decision-cache probes and reusable processing certificates |
| Insertions and evictions | Signature and decision cache maintenance during the request |
| `checkpoint_comparisons` | Required source-head comparisons, including the B1 fallback path |
| `key_bytes_hashed` | JSON bytes hashed specifically to construct cache keys; excludes protocol record-ID hashing |
| `key_items_sorted` | Number of measurement, ballot, and registry-member references supplied to key normalization; not the number of sort comparisons |
| `certificate_items_examined` | Measurement and ballot counts involved in constructing a processing certificate |
| Signature/decision entries | Cache occupancy after a check; replay also reports peak entry counts |

These counters are different operations and must not be added into an unweighted claim of total computational work. They expose maintenance rather than pretending that saved signatures are free. Serialization, allocation, sorting comparisons, protocol-ID hashes, and other instruction costs are not all separately counted. Wall time, CPU, peak RAM, traffic, and energy still require a proper measurement campaign on the Pis.

## Matched replay

The replay runner reads ordered steps containing a bundle, trusted policy/registry/request snapshot, expected outcome, and optional cache reset. B1 and B2 receive identical snapshots; the independent reference checker separately computes an outcome. All three must match the declared expected result. Mismatches remain in the report and cause CLI exit code 1 rather than silently replacing the expectation.

The later B3 milestone extends report schema to version 2 and checks B3 against these same snapshots and expectations; the B2 workload and baseline behavior remain unchanged. See [the B3 contract](b3-dependencies.md).

`workloads/b2-currentness-v1.json` freezes 31 labeled synthetic checks: cold and stable checks, unrelated updates, exact expiry, historical verification, a pre-capture check, missing/replaced heads, signed replacement, expiry of remaining support, renewal, tampering, unknown sources, changed policy contents, role revocation, changed keys, event/nonce replay, missing/duplicate ballots, explicit degraded mode, and restart.

```sh
# Generate the development workload and save the full report and manifest.
cargo run --locked -- replay-demo runs/b2-demo

# Replay the committed workload with a new output directory.
cargo run --locked -- replay workloads/b2-currentness-v1.json runs/b2-replay

# Print the complete JSON report without writing files.
cargo run --locked -- replay workloads/b2-currentness-v1.json
```

Output directories are never overwritten. Each saved run includes the normalized workload, full report, and a manifest hashing the compact typed JSON workload. The manifest explicitly states that timing and energy were not measured. The CLI retains the 1 MiB file limit; library replay accepts at most 4,096 steps. Larger campaigns will need a specified streaming format instead of bypassing that bound.

## Boundary for B3

B2 already tracks required evidence bindings, handles expiry, caches signatures across related bundles, and avoids work for unrelated updates. B3 must preserve all these capabilities and add a clearly specified benefit, such as reverse indexing across many claims or incremental rule evaluation. It must use equivalent authentication, checkpoint, cache-key trust, disclosure, and time assumptions. If a competent B2 already offers the same behavior, report equivalence; do not remove an ordinary optimization to manufacture an advantage.

Neither this development workload nor a lower signature count establishes B3 novelty, general speedup, or physical sensing accuracy. Production key provisioning, distributed checkpoint updates, secure clock handling, Pi deployment, live sensing, durable logs, and action release remain out of scope here.
