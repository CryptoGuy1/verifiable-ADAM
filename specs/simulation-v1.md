# Virtual time simulation version 1

The simulator generates an environmental timeline and signed event traffic, then runs the existing B1/B2/B3/reference checks on the resulting workload. It runs without new sensors or access to the Pis. Its output includes an offline HTML playback, configuration, observations, event trace, reproducible verification workload, reports, and hash manifest.

## What is modeled

Two logical sensing channels represent Pi A and Pi B. Both capture a CO₂/light observation at the configured sample cadence, five seconds by default. Light is sampled alongside CO₂ in this version; there is no separate one-second light acquisition loop. Repeated claim checks never create new observations.

A prescribed CO₂ trajectory rises from 600 ppm, plateaus near 1,000 ppm, and declines. The ideal value is shared by both channels. Each measured value has a fixed channel offset and deterministic seeded noise. A lamp/cover profile changes modeled illuminance between 500 and 20 lux. Temperature/humidity are fixed fixture values, not an additional thermal or humidity model.

The trajectory is a toy test signal, not a calibrated room mass-balance model, manufacturer sensor-response model, or physical experiment. Stabilization, calibration dynamics, sensor accuracy, clock offset, CPU load, fan behavior, temperature throttling, and energy consumption are not modeled. Seeds identify reproducible software cases, not independent environmental sessions.

Each complete capture round produces a new event ID, challenge nonce, sequence, and event-bound signed evidence. The coordinator and three logical voters sign its proposed deterministic decision. Real Ed25519 operations are used, but all fixture signing keys are public test material. Three voters on two node labels do not establish three independent machines.

This first version models delivery of a complete pre-signed claim packet from a claiming peer to a verifier. Source exchange, planner inference, ballot negotiation, individual ballot transport, secure challenge provisioning, and consensus timing are not simulated. Real device services will need those mechanisms separately. A source dropout prevents construction of a complete new round; the generator does not relabel an old sample as new evidence.

## Scenarios

| Scenario | Behavior |
| --- | --- |
| `stable` | Regular samples and short delivery delay; no injected faults |
| `dropout` | Pi A produces no sample during 20–35% of the run; the previous claim can remain current until its TTL |
| `partition` | Complete claim packets are lost during 45–60% of the run; local checks continue against previously delivered support |
| `delay` | Packets captured during 65–80% of the run are delayed, including expired delivery and fresh but superseded delivery |
| `tamper` | One copied packet has a CO₂ field changed without a new signature |
| `replay` | An old authentic packet is submitted against a newer round's trusted event/nonce |
| `false-reading` | During 15–35% of the run, Pi A signs a 400 ppm positive bias; correct protocol processing can disagree with modeled truth |
| `mixed` | Dropout, partition, delays, one tamper attempt, and one replay attempt together; false reading remains a separate scenario |

For the default 60-second mixed run, capture rounds occur at 0, 5, …, 55 seconds. The normal packet delay is 250 ms. Captures at 15 and 20 seconds are missed; packets at 30 and 35 seconds are lost. Packets at 40 and 45 seconds are delayed and arrive after the round captured at 50 seconds. Their outcomes expose expiry and replacement checks. Fault windows scale with duration; discrete capture times determine which rounds fall inside them.

## Trusted state and independent expectations

The simulation model knows which packets it created legitimately and which it modified. Only legitimate delivered packets can advance its trusted source checkpoints, and only when their sequence is newer than the previous accepted round. A delayed older round cannot roll checkpoints or the active claim backwards. These checkpoints are model ground truth, not a secure live checkpoint-distribution implementation.

For each delivery, a request uses the separately recorded authorized event/nonce. A replay attack deliberately supplies an older bundle against the new context. Periodic requests check the newest legitimately delivered active claim. All strategies receive identical policy, registry, request time, and checkpoints.

Expected outcomes are derived from scenario state, not by calling either verifier: injected tampering/replay is REJECTED; an authentic packet at or beyond its TTL is EXPIRED; a fresh older packet behind trusted heads is UNCONFIRMED; otherwise the complete legitimate claim is VERIFIED CURRENT. The engine then separately evaluates B1, B2, B3, and the independent reference, requiring agreement with the declared expectation.

Signed false readings illustrate a different property. The bundle may verify while its observed-data classification differs from the prescribed environmental truth. The simulator reports that disagreement separately; it does not call a valid signature evidence of physical accuracy.

## Timeliness and denominators

Every planned capture round counts as an attempt. Missing source rounds, lost packets, delayed/withheld results, and packets pending beyond the horizon remain in the denominator. A timely verified round is a legitimate delivered claim that passes current verification within the configured virtual deadline measured from capture. This models capture-to-delivery completion, not measured request-to-verification host latency.

The default mixed run has 12 attempted rounds, 2 source misses, 2 partition losses, 8 legitimate deliveries, and 6 timely verified rounds. It generates 70 verification checks, including repeated periodic checks and two attacks. The stable run has 12 timely rounds and 72 checks. These are declared synthetic workload results, not performance guarantees or the research decision-correctness target.

## Run and inspect

```sh
cargo run --locked -- simulate runs/sim-mixed mixed 42
cargo run --locked -- simulate runs/sim-stable stable 42
cargo run --locked -- simulate runs/sim-false-reading false-reading 42

# Modify a copy of config.json, then generate a new run.
cargo run --locked -- simulate-config runs/sim-mixed/config.json runs/sim-custom

# Check the exported workload independently of generation.
cargo run --locked -- replay runs/sim-mixed/workload.json runs/sim-replay
```

`simulation.html` is self-contained and needs no external scripts or internet connection. For a local browser link, use `python3 scripts/run_simulation_viewer.py RUN/simulation.html`, then open `http://127.0.0.1:8765/`. With no file argument the launcher serves the saved mixed example. It binds only to loopback and serves only that HTML, without exposing directory contents. Direct file opening also requires a browser that allows JavaScript. Controls remain disabled until initialization finishes, with a visible startup/error message if it cannot finish.

Its controls play/pause, seek, restart, change playback speed, and include periodic checks in the event table. It displays computed results from the saved run; it does not rerun Rust when a slider moves. Changing a seed or scenario requires a new simulation command.

The manifest identifies synthetic data, public test keys, model version, configuration hash, and normalized workload hash. Host timing and energy are explicitly unmeasured. Run directories and individual outputs are not overwritten.

Configuration bounds limit duration to 20–300 seconds and require positive bounded intervals. The generated workload must fit the existing 1 MiB replay file limit. Larger configured cases that exceed it fail explicitly; a future streaming trace format is needed for larger campaigns.

## Validation and remaining scope

Rust tests cover stable and mixed counts, dropout/loss denominators, out-of-order delivery, attack rejection, signed false readings, deterministic regeneration, replay round trips, bounds, and safe embedding of viewer data. `scripts/check_simulation_viewer.mjs` checks playback and startup-error behavior against a minimal isolated in-memory DOM. A local HTTP browser check confirmed that Play advances the clock and verification results. The reported unresponsive Play button in Brave has not been reproduced in Brave; the local launcher and startup messages provide a clearer way to open and diagnose it.

This is a protocol/traffic simulation, not the final physical study, original ADAM reasoner reproduction, distributed consensus implementation, hardware circuit simulation, or ZKP experiment. The remaining work is tracked in `docs/project-status.md`.
