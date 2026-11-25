# Pi deployment handoff

No software or services on either Pi were changed during the connected-agent delivery. Read-only SSH checks to `ben@evidra.local` and `ben@gateway.local` failed hostname resolution. Current SSH IP addresses are needed before installation or deployment can proceed.

Collect a fresh read-only inventory once reachable:

```sh
ssh ben@PI_IP python3 - < scripts/pi_inventory.py > runs/pi-inventory.json
```

Use separate output paths for each Pi and a new `runs/` directory. The script reports model, aarch64 architecture, OS/kernel, CPU/memory/storage, available toolchains, UTC/clock status, I2C device files and power/thermal flags. It does not probe sensor addresses, install dependencies, change configuration or actuate hardware. Missing commands are explicitly reported.

Before a hardware result can be claimed:

1. Confirm the expected Pi 5/Pi 4 identities, Debian environment and current resources. Install and validate the repository's pinned Rust 1.98.1 toolchain; the present builds/tests run on the development Mac only.
2. Provision persistent OS-random device/role keys with protected storage and independently trusted registry/policy/calibration versions. Local development keys are ephemeral and are not exported. Do not copy public fixture seeds onto devices.
3. Implement the actual acquisition/service boundary, supported SCD40 data-ready cadence and full temperature/humidity/light metadata. Use the approved one SCD40 and one BH1750, two breadboards and existing wires; additional channels are labelled datasets. Do not invent a second physical CO2 sensor or equate replay with calibration.
4. Implement authenticated peer transport, challenge lifetime, monotonic sample sequences and restart-safe replay/head state. Source observations in the current v2 contract are event-bound; long-lived cross-event reuse requires a separately reviewed contract.
5. Validate current power/thermal conditions, clocks and network behaviour. No cooling purchase is added to the approved order. Record the actual configuration and resource limits before interpreting timing.
6. Exercise valid/forged/replayed/missing/stale/replaced evidence, lost ballots, policy/role changes, recording failure, crash/recovery and private-proof substitution on the real devices. Document where Qwen and the historical embedding service actually run; workstation inference is not on-Pi inference.
7. Swap role placement, measure signing/verification/proof and full pipeline costs under declared load, then conduct the physical pilot and frozen matched evaluation. Do not infer Pi latency, accuracy, energy or novelty from Mac tests.

The connected local actor/verification/private-claim/audit components can be reused, but `adam-agent-run` itself deliberately generates synthetic scenarios and is not a sensor daemon or unattended production deployment service.
