# Next implementation milestones

## Current delivery

The workspace implements an experimental B1 public-data path, independent reference outcomes, a file-based CLI, synthetic corpus, exact signature encoding, and adversarial tests. It is ready for review and further implementation, not for claims of a completed two-Pi experiment. The procurement, power/cooling validation, sensor calibration, and Pi deployment work remain open.

## Ordinary caching before dependency optimization

Specify B2 as a competent expiry-aware cache. It should retain authenticated record and policy/registry bindings, per-record validation results, and valid decision results with required support horizons/checkpoints. It must already preserve reuse when an unrelated observation changes. Do not invalidate every cached decision on any global update just to create a B3 advantage.

B3 needs an explicitly additional operation, such as a reverse index for affected claims or incremental evaluation of shared computations. If B2 already has the same mechanism, compare against that stronger baseline and report equivalence. Count dependency-index construction, memory, update propagation, and lookups as well as signature and rule work. Stable, sparse, bursty, and shared-support workloads may favor either strategy.

Both strategies must use the same policy, trusted checkpoint stream, required checks, check times, and disclosure. Compare each against the independent reference at every check, including expiry, new support, policy changes, registry revocation, future captures, and partitions. Timing measurements must not replace correctness checks.

The initial format binds measurements to a decision event. Decide how a later long-lived advisory references reusable sensor observations before extending experiments to cross-event reuse. Do not silently weaken nonce or event binding to improve cache hits. Alternative support sets and calibrated threshold tolerance also need explicit semantics and new independent cases.

## Live acquisition and bilateral service

1. Install the pinned Rust toolchain on both devices and validate clean builds on their aarch64 Linux environments. Existing validation is on the development Mac only.
2. Freeze the actual sensor models and wiring. SCD40 is a procurement candidate; the research protocol currently names SCD41. Do not label one as the other.
3. Add Python acquisition that honors sensor data-ready state, logs raw readings and calibration settings, and passes typed readings to Rust. Rust owns canonical encoding and signing.
4. Add secure device key provisioning and trusted registry/policy version distribution. Generate real keys from operating-system randomness; never reuse demo seeds.
5. Specify challenge freshness, consumed-action/replay persistence, sample sequence history, restart behavior, clock uncertainty, and authenticated checkpoint updates.
6. Add bilateral transport and durable logs with explicit failure behavior. No automatic physical actuation is required for an advisory verification experiment.
7. Record Pi model, role placement, software versions, cooling, power flags, clock offset, and load. Swap role placements and compare B2/B3 on each device.

## Optional proof and confirmatory evaluation

Define a real confidentiality requirement and hiding commitment before choosing a proof system. B4 must bind the private bounded threshold witness to an authenticated event, and enforce freshness separately. Public-reading records in this first milestone are not private proof inputs.

After pilot measurements, preregister primary metrics, practical-effect thresholds, sample sizes, deadline budgets, and uncertainty analysis. Freeze held-out workloads before the confirmatory campaign. No efficiency, accuracy, energy, or ZKP feasibility claim follows from the software tests alone.
