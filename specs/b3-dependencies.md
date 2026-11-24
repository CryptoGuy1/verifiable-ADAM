# B3 dependency index prototype

B3 adds a reverse source-to-claim index to the existing B2 cache. It tests whether incremental maintenance of support matches can help with repeated checks across multiple cached claims. It preserves the same signatures, static verification, cache keys, capacities, time checks, and trusted inputs as B2. This is a conventional indexing prototype whose research value and performance remain to be established.

## Mechanism

Each indexed claim has the exact source-to-measurement bindings from an authenticated decision certificate and a set of currently mismatched sources. A reverse map records which indexed claims require each source. A missing required head is a mismatch, just like a different measurement ID.

On each current request, B3 compares the full supplied trusted head snapshot with the last processed current snapshot. It detects changed values, insertions, and deletions. For a changed source, it visits only that source's reverse links and updates each affected claim's mismatch set. Updates to an unrelated source visit no claim links. The underlying head state is then updated to match the request snapshot.

The implementation receives complete snapshots, exactly as B2 does. It does not assume a free authenticated delta stream, a trustworthy revision number, or pointer identity that proves unchanged data. Finding changes costs a scan of the old and new snapshot entries on every current request, including stable requests. Historical requests do not use heads and skip this synchronization; the next current request compares against the last current snapshot.

When B2's static cache misses, B3 uses the same ordinary signature cache and B1 rule path. There is no additional signature-saving mechanism. When a static certificate is hit, the shared cache still checks time, quorum, and historical/current mode before consulting B3 for source support. The first eligible hit builds dependency metadata from that already authenticated certificate. Later eligible hits inspect whether its maintained mismatch set is empty, without scanning the claim's full support list again.

The support matcher is trusted verifier code, like the signature-cache backend. It is not configurable by a peer's bundle. Full content keys still include signatures, policy contents, registry contents, and expected event/nonce. Changed authorization or signed material cannot reuse an indexed result under the old binding. Malformed or rejected bundles never create dependency metadata. A valid static certificate with missing support may be indexed with a nonempty mismatch set and recover when the correct trusted heads return.

## What remains shared with B2

- Positive signature caching and bounded FIFO decision caching.
- Canonical cache-key normalization and its full hashing/serialization cost.
- Exact time boundaries, future-capture rejection, and support horizons.
- Quorum rules and the distinction between historical and current verification.
- Full policy/registry binding, including edits without a version bump.
- Unconfirmed outcomes for missing/replaced support; rejected malformed or forged data.
- No automatic proposal renewal, alternative support substitution, physical action release, or persistent anti-replay authority.

B3 does not add AND/OR/k-of-n support graphs, incremental fusion, or a new private proof system. Version 1 still requires all configured sources. Those features would need explicit semantics and independent test cases rather than being inferred from the presence of an index.

## Retention and cleanup

Metadata is admitted lazily on the first current, time-eligible, quorum-sufficient static cache hit. An expired, future, historical-only, or insufficient-quorum request does not create new metadata. Existing metadata may remain stored while those outcomes are returned; it cannot override them.

Decision-cache eviction triggers a scan of indexed bindings to identify certificates no longer retained. Their forward and reverse links are removed, and empty reverse buckets are deleted. Cleanup scans and removed links are counted. After each check, indexed claim count is at most retained decision count. Source links are bounded by retained claims times the protocol's maximum source count. These are entry bounds, not measured RAM limits.

Clearing the cache or restarting discards signatures, certificates, metadata, and observed heads. Reset counts expose the additional index state cleared. A zero decision capacity produces no indexed claims. Virtual-time rollback and restored trusted snapshots preserve the same provisional B1 contract; this prototype does not establish secure live rollback protection.

## Counter accounting

B3 reports the same common work and cache-maintenance fields as B2. Direct checkpoint comparisons count only actual source/head comparisons in admission or B1 fallback; an indexed metadata hit does not increment them. Its additional `dependency_work` records:

| Counter | Meaning |
| --- | --- |
| `snapshot_entries_examined` | Entries examined while detecting changes across old and new full head snapshots |
| `snapshot_changes` | Inserted, removed, or value-changed source heads applied |
| `dependency_links_visited` | Reverse links visited for changed sources, including already mismatched claims |
| `mismatch_changes` | Actual transitions into or out of a claim's mismatch set |
| `metadata_lookups`, `metadata_hits` | Support matcher calls and already indexed certificates |
| `metadata_entries_inserted`, `dependency_links_inserted` | Index admission work |
| `cleanup_entries_examined` | Indexed bindings tested against the retained decision cache after eviction |
| `metadata_entries_removed`, `dependency_links_removed` | Evicted-certificate index cleanup |
| Reset fields | Claim, link, and observed-head entries discarded on explicit cache reset |

Reports also expose indexed claims, dependency links, reverse-map source keys, and observed-head entry counts. Replay reports peak indexed claims, links, and observed heads. None of these counters substitutes for CPU, elapsed time, peak RAM, or energy measurements; operations have different costs and must not be summed as though interchangeable. State reconciliation, allocation, tree lookups, and copying are not free.

## Reproducible comparison

All replays now compare B1, B2, B3, the independent reference, and each declared expected outcome. The input format remains version 1; report schema version 2 adds B3 fields. Saved manifests identify all three strategies and explicitly state that timing and energy are unmeasured. The original frozen B2 workload is unchanged.

```sh
# Multi-claim topology, including unaffected claims and shared-source fanout.
cargo run --locked -- dependency-demo runs/b3-demo

# Replay the frozen topology with a new output directory.
cargo run --locked -- replay workloads/b3-multi-claim-v1.json runs/b3-replay

# Re-run the unchanged 31-step B2 corpus with B3 included.
cargo run --locked -- replay workloads/b2-currentness-v1.json runs/b3-baseline-check
```

The new 27-step workload contains two claims with disjoint pairs of logical sources and two ballot variants of claim A. Variants exercise shared-source reverse links without pretending to be different physical measurements. Four logical source identities are mapped to the same two node labels; no four-device or independent-sensor claim follows. Signed observations remain event-bound under version 1.

The workload includes stable checks, a changed source affecting both A variants but not B, renewed A evidence, unrelated updates, source dropout, restored trusted snapshots, expiry, historical and future checks, authorization edits, and restart. Registry edits without version changes stress content binding; they are not a recommended registry-management procedure.

## Development findings and limits

In the 27-check development workload, B1 performs 149 signature attempts; B2 and B3 each perform 23. B3 reduces direct checkpoint comparisons from 46 to 22 but additionally examines 202 snapshot entries and visits 8 reverse links, alongside admission and reset work. All outcomes agree in this software corpus. These counts demonstrate index behavior and expose overhead; they do not establish an overall efficiency improvement.

The frozen 31-step baseline also retains identical B2/B3 signature counts. Existing baseline optimizations were not removed. B2 already carries a support list and handles unrelated updates correctly. A reverse index alone must not be presented as proof of novelty or a privacy improvement. A stronger ordinary cache could adopt the same index; that equivalence belongs in the eventual comparison.

The next research step is a prespecified matched workload and resource measurement campaign that includes stable, sparse, shared-support, bursty, and adverse cases. Any event-stream or batch interface should be offered to B2 under the same authentication and checkpoint assumptions. Final efficiency claims require measured costs and an honest neutral/adverse result when the extra index does not help.
