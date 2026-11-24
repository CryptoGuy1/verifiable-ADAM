# Development replay workloads

`b2-currentness-v1.json` is a frozen synthetic workload with 31 ordered checks and declared expected outcomes. It includes the trusted policy, registry, and request for every step. It is an evaluation fixture, not an untrusted peer's configuration, not raw physical observations, and not a held-out confirmatory dataset.

The public deterministic demo keys are known test material. Virtual-time rollback and restored registry snapshots exercise cache equivalence with B1; they do not model secure live rollback protection. A `reset_cache` step discards prior cache entries before the check.

Run `cargo run --locked -- replay workloads/b2-currentness-v1.json runs/NEW_DIRECTORY` to save the complete report and normalized-input manifest. There is no stochastic timing or energy measurement in this replay, so identical input should produce identical report contents. Capacities can be changed in a copy to examine eviction and cold behavior; do not alter the frozen workload while reporting it under the original identity.

The generator is `crates/replay/src/demo.rs`; tests require it to match the frozen file. Add a new version if semantics or declared cases change. See `specs/b2-cache.md` for accounting and baseline boundaries.

`b3-multi-claim-v1.json` adds 27 synthetic checks with two disjoint logical source pairs and two ballot variants of one claim. It exercises reverse-link fanout, unaffected claims, unrelated updates, dropout, renewal, expiry, authorization edits and restart. The generator is `crates/replay/src/dependencies_demo.rs`. Logical source/claim counts do not describe physical device counts.

All replay commands now include B3 as well as B1/B2/reference checks. Workload schema remains version 1; report schema version 2 adds the B3 strategy and index maintenance fields. B2's frozen input and authentication work are preserved. See `specs/b3-dependencies.md` for findings and limitations.
