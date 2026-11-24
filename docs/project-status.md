# Project status and remaining work

## Working software

- Typed event records, explicit canonical signing format, and real Ed25519 verification.
- Signed measurements, coordinator proposals, and individual logical ballots.
- B1 public recomputation, B2 ordinary caching, and B3 reverse dependency indexing.
- Independently implemented reference acceptance rules and adversarial tests.
- Frozen development workloads, matched reports, input hashes, and virtual-time replay.
- Deterministic two-node claim-delivery simulation with CO₂/light trajectories, faults, and offline playback.

The current software is a development prototype. It has not been deployed to the Pis, connected to real sensors, or shown a measured B3 efficiency advantage. Simulation data and public test keys must remain separate from physical research data and production keys.

## Parallel work and decision gates

**Start now in parallel: Pi preparation and cost measurement.** Install the pinned Rust toolchain, validate release builds, choose/install cooling, and check power under load. Record versions, governor, temperature, throttling, network conditions, and clocks. Measure Ed25519 signing/verification and complete cold B1, warm B2, and B3 checks, including all serialization and maintenance costs. These tasks do not require the final fusion or privacy design. Use the real expected check rate and sensing cadence to decide whether further optimization has practical value. Existing counters and virtual deadlines are not timing results.

**Resolve semantics and contribution while those measurements proceed.** Call the current rule and simulation ADAM-inspired. The original paper uses fixed inverse-variance fusion and role-specific ballots, reasoner output, and governance; changing only weights is insufficient to reproduce it. Define B0 from original code/traces where available or explicitly bound a reconstruction to the paper's specification. Decide reusable-observation/event binding, TTL, tolerance, roles, deadlines, and independently expected decisions. Treat equivalence with a strong ordinary indexed cache as the working hypothesis for the present B3 mechanism.

**Freeze the private record decision before confirmatory acquisition.** Version 1 signs public CO₂ values and is not a private-proof format. If B4 is required, specify a new version with a hiding commitment, public/private disclosure boundary, authenticated context, openings, bounds, and freshness before collecting the corresponding dataset. An undefined reserved field is not sufficient. Old public records remain auditable under their original version, but cannot be made private retroactively. Keep proof-system implementation gated until its purpose and budget are justified.

**Predeclare adverse cases before further index tuning.** Include warm-cache stable claims, unrelated snapshots, high fanout, low query/update ratios, eviction/restart, expiry, policy/role changes, and a competent ordinary indexed comparator. Existing exposed simulations are development data, not held-out evidence. Freeze new hypotheses and workloads before choosing favorable scales or inspecting confirmatory results.

**Confirm budget and development modality.** CO₂ cost and lead time are real constraints. Ask the client whether temperature/light bring-up is acceptable while retaining CO₂ for final validation; do not assume acceptance of a changed modality. Confirm exact sensors, wiring, calibration, data-ready cadence, and procurement timing. Use a typed non-CO₂ development policy if needed, never invented physical CO₂ fields.

**Then connect secure acquisition and peers.** Provision OS-random keys, protected storage, role/node bindings, authenticated configuration updates, nonce challenges, sequence/replay persistence, actual transport, UTC/monotonic handling, and explicit failure/deadline behavior. Rust owns canonical encoding/signing; Python can acquire and orchestrate. Model-created trusted checkpoints do not implement this service.

**Complete audit, scoring, and physical evaluation.** Save raw readings, signed records, rejected/timed-out attempts, manifests, and restart traces with defined tamper-evident log guarantees. Keep processing correctness separate from physical sensor accuracy and useful outcomes. Run pilot sessions, then freeze primary outcomes, practical-effect thresholds, sample sizes, uncertainty analysis, held-out traces, and exclusions. Measure on each Pi with role crossover; direct energy measurement remains optional. Cryptographic operations and synthetic seeds are not independent environmental replications.

See [research review decisions](../specs/review-decisions.md) for the verified ADAM differences, privacy-format implications, adverse cases, and client questions. No client answers, purchases, or Pi deployment are assumed.

## Immediate review artifact

Run `cargo run --locked -- simulate runs/NEW_DIRECTORY mixed 42` and inspect the generated `simulation.html`, event trace, and report. The viewer plays a precomputed deterministic run. Read `specs/simulation-v1.md` for exactly what is modeled and what remains outside the model.
