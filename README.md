# Verifiable ADAM

**Specification alignment warning:** The legacy browser examples below use provisional equal-weight, boolean-only decision semantics. The connected weighted terminal pipeline is separate and is not yet a complete physical deployment. See [requirements and known gaps](specs/requirements-traceability.md). All further implementation and demo work must follow [AGENTS.md](AGENTS.md); deviations require an explicit **SPEC DRIFT WARNING** before proceeding with the affected change.

Verifiable ADAM investigates whether dependency-aware verification can reduce repeated work compared with a competent expiry-aware cache while preserving the same acceptance rules. Our physical testbed is one Pi 5 and one Pi 4, using CO₂ and light observations.

The legacy equal-weight rule and browser simulation are **ADAM-inspired**. The connected pipeline implements the corrected weighted/role/governance rules, with reconstruction limits recorded in the runtime contract; the older examples do not establish ADAM compatibility. Equivalence with a strong ordinary indexed cache is the working hypothesis for the current B3 index. See [the review decisions](specs/review-decisions.md) for the comparison, privacy-format, and data-collection gates.

The software now includes signed synthetic observations, coordinator-signed decisions, individually signed ballots, a deterministic B1 public verifier, a B2 expiry-aware cache, a B3 reverse dependency index, and a separately implemented reference evaluator. A matched replay runner checks B1, B2, and B3 against declared expected outcomes and the reference. These establish a protocol and development corpus, not research novelty, physical accuracy, or hardware performance.

## Connected weighted agents — no UI update

```sh
cargo run --locked --offline -- adam-agent-run runs/NEW rules
cargo run --locked --offline -- adam-agent-run runs/OTHER ollama
```

This path connects actual inbox-driven workers to the weighted signed pipeline, v2 B1/B2/B3 checks, a real two-reading weighted private proof and a durable advisory-authorization gate. Use `private-export/` for the restricted auditor package; full round/disclosure files contain private readings and model text. See [the runtime contract and remaining gates](specs/weighted-agent-runtime.md), including optional Weaviate/MiniLM history setup. The UI is unchanged at the user’s request. These are synthetic local runs, not deployed Pis, physical actuation or demonstrated novelty.

## Using the two Raspberry Pis for tests now

This is the **current-code testing procedure**, updated 26 November 2026. Start with software-only tests; no sensor purchase or wiring is needed for these steps. The commands below are instructions for you to run, not a record that deployment has already happened. Previous checks from this computer could not resolve `evidra.local` or `gateway.local`.

### 1. Understand the test arrangement

| Machine | First test assignment | What actually runs there |
| --- | --- | --- |
| Pi 5, `evidra`, 4 GB RAM | Generate decisions and proofs | All local source/Sensor/Aggregator/Decision/Coordinator workers, Rust verification, private proof generation and local audit recording |
| Pi 4, `gateway`, approximately 8 GB RAM | Independent checking | Native Rust public/private verification and audit/authorization checks on files produced by Pi 5 |
| Your Mac | Orchestration; optional model/history host | Source transfer, retained trust/checkpoint copies, and the already installed Qwen/Ollama and Weaviate/MiniLM services when requested |

First use `rules`, which needs neither Ollama nor Weaviate. Afterwards use the optional SSH tunnels below for `ollama` or `ollama-history`.

**Scope:** all voting workers in a producer run are still on that producer Pi. Transferring the resulting files to the other Pi does not implement a live distributed crew or the unfinished peer service. Generated registry labels `local-a`/`local-b` are logical labels, not proof of physical placement. Three logical votes on two owned Pis do not establish three independent physical voters.

The generator still uses **scripted synthetic readings and synthetic calibration**, not connected sensors or imported environmental datasets. The experimental 800 ppm threshold is not a health or safety limit. Native Pi builds, timings and proof feasibility must be established by these tests; Mac results do not establish them.

### 2. Find the addresses and confirm SSH access — Mac terminal

In an existing SSH session on **each Pi**, run:

```sh
hostname
hostname -I
```

Record the Pi 5 and Pi 4 addresses separately. Then, on your Mac, replace the two placeholders below. Keep using this Mac terminal for transfer commands; shell variables are not automatically shared with a newly opened terminal.

```sh
cd /Users/greatadams/Desktop/dev/verifiable-ADAM
PI5='ben@REPLACE_WITH_PI5_IP'
PI4='ben@REPLACE_WITH_PI4_IP'
ssh "$PI5" 'hostname; uname -m'
ssh "$PI4" 'hostname; uname -m'
```

Expected: the correct hostnames and `aarch64`. Verify the SSH host fingerprint on first connection. If either address fails, resolve connectivity before continuing; do not substitute guessed addresses or disable host-key checking.

### 3. Copy the same source to both Pis — Mac terminal

Use a dedicated `~/verifiable-ADAM` directory. If it already contains work, inspect/preserve it before copying; choose a fresh directory and adjust the commands if necessary. Do not overwrite the older Elsevier project. Copy source rather than the Mac's compiled binaries: Linux/aarch64 needs a native build.

```sh
mkdir -p runs/pi-setup

git rev-parse HEAD > runs/pi-setup/source-commit.txt
git status --short > runs/pi-setup/source-working-tree.txt

python3 - <<'PY'
from pathlib import Path
import hashlib
roots = ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
         'crates', 'scripts', 'tests', 'workloads', 'infra']
files = set()
for name in roots:
    root = Path(name)
    candidates = root.rglob('*') if root.is_dir() else [root]
    for path in candidates:
        if (path.is_file() and '__pycache__' not in path.parts
                and path.name != '.DS_Store' and path.suffix != '.pyc'):
            files.add(path)
with open('runs/pi-setup/source.sha256', 'w') as output:
    for path in sorted(files):
        output.write(hashlib.sha256(path.read_bytes()).hexdigest()
                     + '  ' + path.as_posix() + '\n')
PY

for PI in "$PI5" "$PI4"; do
  ssh "$PI" 'mkdir -p ~/verifiable-ADAM/setup'
  rsync -az \
    --exclude='.git/' --exclude='target/' --exclude='runs/' \
    --exclude='private-keys/' --exclude='.env*' --exclude='.venv/' \
    --exclude='.codex/' --exclude='.agents/' --exclude='__pycache__/' \
    --exclude='.DS_Store' --exclude='output/' \
    ./ "$PI:verifiable-ADAM/"
  scp runs/pi-setup/source-commit.txt runs/pi-setup/source-working-tree.txt \
    runs/pi-setup/source.sha256 "$PI:verifiable-ADAM/setup/"
done
```

The manifest records the transferred runtime/test source, including local changes; a commit ID alone does not describe a dirty checkout. The repository contains public test fixtures, but this procedure excludes private key directories and environment files. Current connected runs generate new ephemeral keys; persistent device key provisioning is still unfinished.

### 4. Install and build — repeat in an SSH session on each Pi

Start with `ssh "$PI5"` or `ssh "$PI4"` from the Mac terminal. Run the following **on the Pi**, not on the Mac:

```sh
cd ~/verifiable-ADAM
mkdir -p runs/setup runs/measurements
sha256sum -c setup/source.sha256 > runs/setup/source-check.txt
python3 scripts/pi_inventory.py > runs/setup/inventory-before.json
```

Stop if the source check reports `FAILED` or missing files. The inventory is read-only: it records hardware/software, clock status, I2C device files and power/thermal flags; it does not probe a sensor or claim acquisition works.

Install the compiler prerequisites and the repository's pinned Rust toolchain:

```sh
sudo apt update
sudo apt install --no-install-recommends build-essential curl ca-certificates python3 rsync time

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  -o /tmp/veri-adam-rustup.sh
sh /tmp/veri-adam-rustup.sh -y --profile minimal --default-toolchain none
. "$HOME/.cargo/env"

rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
rustc --version
cargo --version
rustc -vV
```

If rustup is already installed, skip its installer and load the existing Cargo environment. The project pins Rust **1.98.1**; do not silently change the toolchain, dependencies or thresholds to make an error disappear. If that exact toolchain cannot be downloaded, retain the error and resolve it before treating the setup as reproducible. Installation follows the [official Rust instructions](https://rust-lang.org/install) and [rustup profiles](https://rust-lang.github.io/rustup/concepts/profiles.html); ARM64 Linux is listed in [Rust's supported platforms](https://doc.rust-lang.org/rustc/platform-support.html).

Fetch dependencies once, then run the checks and native release build. Run each command in order and inspect failures before proceeding. `pipefail` preserves failed test/build exit status when saving output with `tee`.

```sh
cd ~/verifiable-ADAM
set -o pipefail
cargo fetch --locked
cargo fmt --all -- --check
cargo test --workspace --locked --offline -j 2 -- --test-threads=1 \
  2>&1 | tee runs/setup/rust-tests.txt
cargo clippy --workspace --all-targets --locked --offline -j 2 -- -D warnings \
  2>&1 | tee runs/setup/clippy.txt
cargo build --release --locked --offline -p veri-adam-cli -j 2 \
  2>&1 | tee runs/setup/release-build.txt
python3 scripts/pi_inventory.py > runs/setup/inventory-after.json
```

Two build jobs and serial test execution limit simultaneous work on the smaller-memory Pi. The previously recorded test count is a Mac result, not an expected Pi pass certificate. Keep any Pi failures unchanged for diagnosis. Do not install Docker, MiniLM or Ollama on either Pi for this first procedure.

### 5. Run the complete sensor-free pipeline — Pi 5 first

On Pi 5:

```sh
cd ~/verifiable-ADAM
mkdir -p runs/measurements
python3 scripts/pi_inventory.py > runs/measurements/pi5-rules-before.json
/usr/bin/time -v -o runs/measurements/pi5-rules-process.txt \
  target/release/veri-adam adam-agent-run runs/pi5-rules rules
python3 scripts/pi_inventory.py > runs/measurements/pi5-rules-after.json
```

`runs/pi5-rules` must not already exist. Every attempt uses a **new** output directory; keep partial/failed runs rather than overwriting their evidence. The command creates real signatures, ballots, proofs and audit entries around synthetic readings. It does not read GPIO/I2C or actuate equipment.

Inspect what happened:

```sh
python3 - <<'PY'
import json
from pathlib import Path
root = Path('runs/pi5-rules')
for round in json.loads((root / 'agent-run.json').read_text()):
    name = round['event']['event_id']
    outcome = (round.get('verification') or {}).get('outcome', 'withheld_before_decision')
    private = (round.get('private_verification') or {}).get('private_fusion_verified', False)
    authorized = (root / name / 'advisory-authorization.json').exists()
    print(name, '|', outcome, '| private fusion:', private, '| authorized:', authorized)
PY
python3 -m json.tool runs/pi5-rules/verification-work.json
```

Expected behaviour **when deadlines, freshness and recording checks pass**:

| Case | What to inspect |
| --- | --- |
| `weighted-normal` | Verified processing and a normal advisory authorization |
| `weighted-anomaly` | Verified processing and an anomaly advisory authorization |
| `temporal-spike` | Sensor detects the rise; the `rules` backend normally resolves NORMAL because Aggregator and Decision support NORMAL. Quorum, not the Sensor alone, decides. |
| `degraded-crew` | Only Sensor and Decision are eligible; both must agree |
| `missing-ballot` | Full processing is unconfirmed; no advisory authorization |
| `tampered-source` | Evidence is withheld before inference/voting; no authorization |

A successful command exit means the run/reports were produced, not that all six decisions were accepted. `verified_current_awaiting_recording` describes the signed processing stage; the later signed `advisory-authorization.json` is separate evidence that the recording/release gate passed. Original stage flags remain unchanged. An authorization is a recommendation receipt, not a physical fan/valve command.

If a Pi exceeds the budget, record the withholding result. Do not extend the deadline/TTL or remove checks to force the expected happy path. Record temperatures and raw `get_throttled` results; nonzero flags can include historical conditions and should not be assigned a cause without checking. No additional cooling purchase is added here.

### 6. Have Pi 4 independently check Pi 5's private package

The default transfer is **only `private-export/`**. Full run files, `round.json`, `authorized-disclosure.json` and semantic-store data contain readings/openings/model text; do not send those to a checker whose role is restricted to the private claim.

First, on the Mac, obtain and retain the operator-approved trust/configuration, latest checkpoint and replay requests through the known Pi 5 SSH connection. Stage them separately on Pi 4 before verifying candidate packages:

```sh
cd /Users/greatadams/Desktop/dev/verifiable-ADAM
mkdir -p runs/pi5-roots/requests
scp "$PI5:verifiable-ADAM/runs/pi5-rules/trust.json" runs/pi5-roots/trust.json
scp "$PI5:verifiable-ADAM/runs/pi5-rules/latest-checkpoint.json" runs/pi5-roots/latest-checkpoint.json
for CASE in weighted-normal weighted-anomaly temporal-spike degraded-crew missing-ballot; do
  scp "$PI5:verifiable-ADAM/runs/pi5-rules/$CASE/request.json" \
    "runs/pi5-roots/requests/$CASE.json"
done
shasum -a 256 runs/pi5-roots/trust.json runs/pi5-roots/latest-checkpoint.json
ssh "$PI4" 'mkdir -p ~/verifiable-ADAM/trusted/pi5-rules ~/verifiable-ADAM/incoming'
rsync -az runs/pi5-roots/ "$PI4:verifiable-ADAM/trusted/pi5-rules/"

scp -r "$PI5:verifiable-ADAM/runs/pi5-rules/private-export" runs/from-pi5-private
scp -r runs/from-pi5-private "$PI4:verifiable-ADAM/incoming/"
```

Use a new local transfer directory if `runs/from-pi5-private` already exists. This is manual trust registration through an operator-controlled SSH path, **not** the unfinished automatic provisioning protocol. Do not automatically trust a registry, policy, request clock or checkpoint simply because it arrived in a candidate package. Retain the Mac checkpoint copy independently; copies beside an attacker-controlled log do not alone prevent whole-state rollback.

On Pi 4:

```sh
cd ~/verifiable-ADAM
mkdir -p runs/pi4-private-checks
PACKAGE=incoming/from-pi5-private
TRUST=trusted/pi5-rules/trust.json
PIN=trusted/pi5-rules/latest-checkpoint.json
sha256sum "$TRUST" "$PACKAGE/trust.json"

/usr/bin/time -v -o runs/pi4-private-checks/normal-process.txt \
  target/release/veri-adam adam-private-check \
  "$PACKAGE/weighted-normal/record.json" \
  "$PACKAGE/weighted-normal/private-proof.json" \
  "$TRUST" trusted/pi5-rules/requests/weighted-normal.json \
  > runs/pi4-private-checks/normal.json

target/release/veri-adam adam-audit-check \
  "$PACKAGE/audit" "$TRUST" "$PIN" \
  > runs/pi4-private-checks/audit.json

target/release/veri-adam adam-release-check \
  "$PACKAGE/weighted-normal/advisory-authorization.json" \
  "$PACKAGE/weighted-normal/record.json" "$TRUST" "$PACKAGE/audit" "$PIN" \
  > runs/pi4-private-checks/normal-authorization.json
```

The trust-file digests must match the separately registered copy. Repeat the private/authorization checks for `weighted-anomaly`, `temporal-spike` and `degraded-crew`, changing the case name and result filename.

Interpret the reports correctly:

- `private_fusion_verified=true`: both bounded hidden inputs support the claimed weighted threshold classification. It does not prove Qwen reasoning, Sensor temporal reasoning, complete governance processing or physical sensor accuracy.
- The private checker can verify the **numerical predicate** for `missing-ballot` while reporting no authenticated crew classification. That is not permission to release an advisory; its quorum is unresolved and no authorization receipt should exist.
- `authorization_history_verified=true` verifies the past signed authorization and its recording linkage. `currentness_verified=false` is intentional: a historical receipt does not prove support is still current.
- Saved requests use a **synthetic epoch and operator-selected replay time**. Here “current” means current at that specified test time. These files are not fresh live challenges, UTC room measurements or a secure peer-head service.
- Fixed synthetic values are published in the fixtures and can be inferred from scenario/source knowledge. This checks cryptographic binding and the export boundary, not secrecy of already published values.

### 7. Exercise rejection and expiry — Pi 4

Create damaged **copies**, leaving the original signed package and operator-approved files untouched. Continue on Pi 4:

```sh
cd ~/verifiable-ADAM
PACKAGE=incoming/from-pi5-private
TRUST=trusted/pi5-rules/trust.json
mkdir -p runs/pi4-private-checks
python3 - <<'PY'
import json
from pathlib import Path
root = Path('incoming/from-pi5-private/weighted-normal')
proof = json.loads((root / 'private-proof.json').read_text())
raw = bytearray.fromhex(proof['proof_hex'])
raw[0] ^= 1
proof['proof_hex'] = raw.hex()
Path('runs/pi4-private-checks/corrupt-proof.json').write_text(json.dumps(proof))
request = json.loads(Path('trusted/pi5-rules/requests/weighted-normal.json').read_text())
record = json.loads((root / 'record.json').read_text())
trust = json.loads(Path('trusted/pi5-rules/trust.json').read_text())
request['now_ms'] = min(o['payload']['captured_at_ms'] + trust['policy']['evidence_ttl_ms']
                        for o in record['observations'] if o['payload']['purpose'] == 'fusion')
Path('runs/pi4-private-checks/expired-request.json').write_text(json.dumps(request))
PY

target/release/veri-adam adam-private-check \
  "$PACKAGE/weighted-normal/record.json" runs/pi4-private-checks/corrupt-proof.json \
  "$TRUST" trusted/pi5-rules/requests/weighted-normal.json
printf 'exit status: %s\n' "$?"

target/release/veri-adam adam-private-check \
  "$PACKAGE/weighted-normal/record.json" "$PACKAGE/weighted-normal/private-proof.json" \
  "$TRUST" runs/pi4-private-checks/expired-request.json
printf 'exit status: %s\n' "$?"
```

Both should refuse the private claim with exit **1** and `private_fusion_verified=false`. Exact expiry is already too old; a valid proof cannot override freshness. Exit **2** means an input/operational error, such as a missing file, rather than a successful rejection test. The workspace tests additionally cover wrong nonce/event, duplicate voters, changed policy/roles, replacements, forged summaries, corrupt audit history and restart cases.

### 8. Optional full-disclosure verification — Pi 4 as an authorized reader

This is a **different disclosure mode**. Only perform it when Pi 4 is allowed to see the raw readings, openings and model text. Once received, Pi 4 is not a “never saw the readings” outside auditor for that dataset.

On the Mac, relay the full run into a separately named directory:

```sh
scp -r "$PI5:verifiable-ADAM/runs/pi5-rules" runs/from-pi5-authorized
scp -r runs/from-pi5-authorized "$PI4:verifiable-ADAM/incoming/"
```

On Pi 4:

```sh
cd ~/verifiable-ADAM
FULL=incoming/from-pi5-authorized
TRUST=trusted/pi5-rules/trust.json
target/release/veri-adam adam-check \
  "$FULL/weighted-normal/record.json" "$FULL/weighted-normal/authorized-disclosure.json" \
  "$TRUST" trusted/pi5-rules/requests/weighted-normal.json

target/release/veri-adam adam-check \
  "$FULL/missing-ballot/record.json" "$FULL/missing-ballot/authorized-disclosure.json" \
  "$TRUST" trusted/pi5-rules/requests/missing-ballot.json
```

Expected: normal processing verifies; the missing-ballot check is unconfirmed/exit 1. This checker recomputes the disclosed weighted/temporal/quorum/governance path; the private predicate checker intentionally provides a narrower guarantee.

### 9. Optional Qwen and authenticated history — Mac services, Pi workers

Keep Qwen and Weaviate/MiniLM on the Mac initially. Native inference on either Pi is a separate feasibility test; the existing code does not require an Ollama command-line installation on the Pis. The Python adapters contact loopback HTTP endpoints.

On the Mac:

```sh
cd /Users/greatadams/Desktop/dev/verifiable-ADAM
ollama list
docker compose -f infra/history.compose.yml up -d
python3 scripts/history_store.py init
```

Confirm `qwen3:1.7b` is installed and the normal local Ollama service is running. Keep the Mac awake. In a **separate Mac terminal**, set the Pi 5 address again and keep this command running:

```sh
PI5='ben@REPLACE_WITH_PI5_IP'
ssh -N -T -o ExitOnForwardFailure=yes \
  -o ServerAliveInterval=30 -o ServerAliveCountMax=3 \
  -R 127.0.0.1:11434:127.0.0.1:11434 \
  -R 127.0.0.1:8091:127.0.0.1:8091 "$PI5"
```

These reverse SSH forwards make the Mac's services available through Pi 5's loopback ports. Do not expose Ollama/Weaviate on `0.0.0.0` to make this work. The two forwards need unused ports and server permission for remote forwarding. If setup fails, inspect the SSH error rather than disabling security checks or bypassing the history requirement. On Pi 5, `ss -ltn` should show the forwarded listeners bound only to loopback (`127.0.0.1`/`::1`); close the tunnel if it instead exposes them on all interfaces. See the [OpenSSH forwarding documentation](https://man.openbsd.org/ssh).

On Pi 5, verify connectivity without downloading a model:

```sh
python3 - <<'PY'
import json, urllib.request
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
with opener.open('http://127.0.0.1:11434/api/tags', timeout=5) as response:
    print([m['name'] for m in json.load(response)['models']])
with opener.open('http://127.0.0.1:8091/v1/.well-known/ready', timeout=5) as response:
    print('history HTTP status:', response.status)
PY

cd ~/verifiable-ADAM
python3 scripts/pi_inventory.py > runs/measurements/pi5-history-before.json
/usr/bin/time -v -o runs/measurements/pi5-history-process.txt \
  target/release/veri-adam adam-agent-run runs/pi5-qwen-history ollama-history
python3 scripts/pi_inventory.py > runs/measurements/pi5-history-after.json
```

`ollama` alone uses Qwen without semantic history; `ollama-history` enables authenticated prior-event retrieval and requires history-store write acknowledgment before authorization. A missing required acknowledgment must withhold authorization. Model loading happens before observations, but inference, retrieval and subsequent processing still have their defined event budget. Check `model-preparation.json`, `history-preparation.json`, each round's recorded attempts/retrieval metadata, store acknowledgments and authorization files. Real Qwen outputs can differ from the deterministic rule run; inspect actual votes, confidence, repair/fallback and withholding rather than forcing four authorizations.

**Label this run accurately:** agents/proving/local audit on Pi 5; Qwen, embeddings and semantic storage on the Mac through SSH. This is hybrid inference, not on-Pi Qwen or Pi-only latency. Pi-to-Mac authorized traffic includes readings and model/history context; the restricted external auditor still receives only `private-export/`. These tunnels route existing development services; they are not the unfinished signed peer protocol between the Pis.

The deliberate required-store fault runner is currently a **workstation development test** (`scripts/check_history_store_failure.py`), with a debug-binary path and local proxy. Do not present running that script on the Mac as Pi validation, or assume it is a Pi deployment command. The normal pipeline records required-store failures in the signed journal.

### 10. Swap placements, archive evidence, and interpret costs

After Pi 5 generation/Pi 4 checking works, run on Pi 4:

```sh
cd ~/verifiable-ADAM
python3 scripts/pi_inventory.py > runs/measurements/pi4-rules-before.json
/usr/bin/time -v -o runs/measurements/pi4-rules-process.txt \
  target/release/veri-adam adam-agent-run runs/pi4-rules rules
python3 scripts/pi_inventory.py > runs/measurements/pi4-rules-after.json
```

Relay its private package through the Mac to Pi 5 and repeat steps 6–7 with the roles, run names, trusted roots and result directories swapped. Register the **new run's** trust/checkpoint/requests; ephemeral keys differ, so Pi 5-run trust files do not authorize a Pi 4-generated run. For a Qwen crossover, open equivalent reverse tunnels from the Mac to Pi 4 and record that inference remains on the Mac.

Keep the source manifest, commit/dirty status, before/after inventories, test/build logs, whole-process time/RSS, complete internal runs, public exports, operator-retained roots/checkpoints and checker JSON. Copy results back to the Mac into distinct Pi/run directories. Preserve rejected, failed and timed-out attempts as well as successful ones.

`/usr/bin/time -v` measures whole-process wall/CPU time and maximum resident memory; it is not direct energy or per-proof timing. Use release builds, the same transferred packets, comparable load and recorded power/thermal conditions when comparing devices. Run one producer at a time for the initial baseline; concurrent inference/load belongs in separately labelled tests. Each new verifier process is cold; repeated command launches are **not warm B2/B3 cache measurements**. `verification-work.json` reports in-process development counters, not a demonstrated speedup. B2 and B3 must keep equivalent assurance, and equivalence/negative findings are valid results. A held-out/scaled v2 benchmark and the confirmatory experiment are still future work.

### What this procedure does not yet connect

- Live SCD40/BH1750 acquisition, full environmental metadata and calibration against a suitable reference.
- A real dataset ingestion path into the connected runtime. The approved extra channels will be labelled replay; `adam-agent-run` currently runs fixed synthetic cases and has no sensor/CSV mode.
- Protected persistent device keys, automatic registry/policy distribution, live authenticated Pi-to-Pi challenge/head transport and restart-safe acquisition/replay services.
- An unattended deployed agent service or a CLI that resumes an old run with its lost ephemeral keys. Journal recovery is tested library functionality, not permission to edit log files or invent a resume command.

Use exactly the approved hardware scope: two breadboards, one SCD40 and one BH1750, existing wires, and labelled datasets later; cooling packs remain excluded from the order. One physical CO2 sensor plus a replay channel is not two independent live CO2 sensors. See [the deployment handoff](docs/pi-deployment.md) and [the current runtime contract](specs/weighted-agent-runtime.md) before progressing from this software/device pilot to physical claims.

## Weighted ADAM decision stage

The corrective `decision` module implements paired-reference inverse-variance calibration, weighted fusion/dispersion, temporal Sensor assessment, the seven-field recommendation, recommendation selection, strict crew quorum, final action alignment and governance. It preserves genuine majority disagreement with the fused result. See [the source mapping and limits](specs/adam-decision-stage.md).

```sh
cargo run --locked --offline -- adam-run runs/adam-stage-new rules
# Uses local qwen3:1.7b after model preparation; choose a new directory.
cargo run --locked --offline -- adam-run runs/adam-stage-qwen-new ollama
```

These commands export inspectable unsigned decision-stage records and explicit policies/calibration. They do not release an advisory. The connected actor path above now wraps this engine, with private fusion proofs, dependencies and durable authorization. Historical semantic retrieval is live-validated locally; Pi deployment and physical validation remain open. UI changes are deferred. The older browser demo below still uses legacy v1 semantics.

## Authenticated weighted records and independent checking

```sh
cargo run --locked --offline -- adam-auth-run runs/adam-authenticated-new rules
# Uses the same weighted engine with the installed local Qwen model:
cargo run --locked --offline -- adam-auth-run runs/adam-authenticated-qwen-new ollama
cargo run --locked --offline -- adam-check \
  runs/adam-authenticated-new/weighted-normal/record.json \
  runs/adam-authenticated-new/weighted-normal/authorized-disclosure.json \
  runs/adam-authenticated-new/trust.json \
  runs/adam-authenticated-new/weighted-normal/request.json
```

Version 2 binds committed source observations, registered weights, complete decision/reasoner references, eligible roles, signed ballots, proposed action and governance. A separate checker recomputes the weighted/temporal rules, actual quorum and policy. The runner uses fresh ephemeral OS-random keys and synthetic evidence; these are local components, not deployed agents or device provisioning. Source mapping, canonical encoding, acceptance cases and limits are in [the authenticated record contract](specs/authenticated-decision-v2.md).

Successful processing still says **awaiting recording**, with no action release. This older `adam-auth-run` command does not generate the private proof. The connected `adam-agent-run` path does; use `adam-private-check` for that narrower claim. `adam-check RECORD - TRUST REQUEST` still returns unconfirmed because full disclosed processing needs the disclosure file. `authorized-disclosure.json` contains readings, openings and private model text; do not send it to an external private verifier. Legacy v1 signed bytes and examples remain unchanged.

## Run the agent simulation with local Qwen

```sh
cargo run --locked --offline -- agent-simulate runs/agents-local ollama
python3 scripts/run_simulation_viewer.py runs/agents-local/simulation.html --port 8767
```

Open **http://127.0.0.1:8767/**. This runs stateful Sensor, Aggregator and Decision actors through message inboxes; the Decision actor calls the installed local `qwen3:1.7b`. The viewer shows individual assessments, dissenting signed ballots, the collective advisory, withholding and expiry. Use a new output directory. The `rules` backend tests the same actors without model inference. See [the agent contract](specs/agent-simulation-v1.md) for signature bindings, real-time deadlines, fault cases and limitations. The coordinator is a nonvoting local state machine; these are local workers, not deployed Pi services. This legacy viewer has no ZKP; the new terminal pipeline above implements the weighted private-claim pilot.

## Run the scripted trajectory simulation

To view the saved example, run this from the project folder and leave the terminal running:

```sh
python3 scripts/run_simulation_viewer.py
```

Open **http://127.0.0.1:8765/** in your browser, then click **Play**. This serves just the simulation page on your own computer. No packages need installing. If the page says JavaScript has not started, check that your browser allows JavaScript for this local page. An initialization failure now displays an error instead of an active-looking Play button. Use `--port 8766` if the port is occupied.

To generate a different run:

```sh
cargo run --locked -- simulate runs/sim-mixed mixed 42
python3 scripts/run_simulation_viewer.py runs/sim-mixed/simulation.html
```

The run creates `simulation.html` with playback controls, CO₂/light plots, node/link states, strategy outcomes, and an event timeline. It also saves the configuration, observations, workload, reports, and manifest. The simulation uses virtual time, toy environmental data, and public fixture keys; no sensors or Pis need to be connected.

Scenarios are `stable`, `dropout`, `partition`, `delay`, `tamper`, `replay`, `false-reading`, and `mixed`. Use a new directory each time. For example, `simulate runs/sim-stable stable 42` produces a normal run, while `simulate runs/sim-false false-reading 42` demonstrates that signed false readings can pass protocol verification. `simulate-config CONFIG NEW_DIRECTORY` accepts the saved typed configuration. See [the simulation contract](specs/simulation-v1.md) and [what remains to build](docs/project-status.md).

## Run the signed event demonstration

Install the Rust toolchain selected in `rust-toolchain.toml` on the development machine. Both Pis still need Rust installed; nothing in this repository has been deployed to them. The workspace pins Rust 1.98.1 and commits dependency versions in Cargo.lock.

```sh
cargo run --locked -- demo runs/first-demo
cargo run --locked -- verify runs/first-demo/bundle.json runs/first-demo/policy.json runs/first-demo/registry.json runs/first-demo/request.json
```

Use a new directory for every demo; existing outputs are not overwritten. The demo uses public, deterministic test keys and synthetic readings of 900 and 850 ppm. No private device keys or real physical data are generated.

The demo also writes alternate inputs:

```sh
# Rejected: a reading was changed after signing. Exits 1.
cargo run --locked -- verify runs/first-demo/bundle-tampered.json runs/first-demo/policy.json runs/first-demo/registry.json runs/first-demo/request.json

# Expired: the check occurs exactly at the support expiry. Exits 1.
cargo run --locked -- verify runs/first-demo/bundle.json runs/first-demo/policy.json runs/first-demo/registry.json runs/first-demo/request-expired.json

# Historical processing verifies, but this is not a current advisory. Exits 0.
cargo run --locked -- verify runs/first-demo/bundle.json runs/first-demo/policy.json runs/first-demo/registry.json runs/first-demo/request-historical.json
```

The verifier prints JSON containing its outcome, reason, decision ID, support expiry, and work counters. It also runs the independent reference checker and reports an operational error if their outcomes disagree. These counters describe the B1 software path; they are not benchmark results.

**Trust boundary:** policy, registry, and request files are trusted verifier configuration. Do not accept them from a claimant along with its bundle. The request supplies the expected event, nonce, check time, check kind, and authenticated evidence checkpoints. In this demo those files are synthetic fixtures, not a secure provisioning system.

## Run the B2 cache replay

```sh
cargo run --locked -- replay-demo runs/b2-demo
cargo run --locked -- replay workloads/b2-currentness-v1.json runs/b2-replay
```

Each command uses a new output directory and saves a workload, full report, and hash manifest. The frozen workload has 31 synthetic checks covering stable evidence, unrelated updates, exact expiry, historical checks, replacement, malformed claims, revocation, quorum failures, and restart. A replay exits 0 only when B1, B2, B3, the reference, and expected outcomes all agree; mismatch exits 1 and input/operational error exits 2. Run `replay WORKLOAD` without an output directory to print full JSON.

B2 reuses valid signatures and decision-processing results while checking time and required source heads on every decision hit. Its complete policy/registry/content bindings prevent stale cached authorization. It already avoids revalidation for unrelated updates. The report includes cache maintenance, key hashing/normalization, and occupancy counters as well as saved signature work. These are diagnostic counts, not timing or hardware performance measurements. See [the B2 contract](specs/b2-cache.md).

## Run the B3 dependency index prototype

```sh
cargo run --locked -- dependency-demo runs/b3-demo
cargo run --locked -- replay workloads/b3-multi-claim-v1.json runs/b3-replay
```

B3 shares B2's static authentication cache and adds a source-to-claim reverse index. It updates mismatch state only along affected links. It still receives full head snapshots, so change detection and index maintenance are explicitly counted. The frozen 27-step workload exercises multiple logical claims, shared source links, dropout, replacement, authorization changes, and restart.

On this synthetic workload B2 and B3 perform the same 23 signature checks. B3 saves some direct support comparisons but adds snapshot scanning and metadata work. No overall speedup or novelty is established. See [the B3 contract and findings](specs/b3-dependencies.md). Reports use schema version 2, while both workload input formats remain version 1.

## Code structure

| Path | Responsibility |
| --- | --- |
| `crates/protocol` | Typed records, explicit signing encodings, SHA-256 IDs, Ed25519 signatures, public demo fixtures |
| `crates/verification` | B1 authentication, source checks, deterministic classification, quorum, currentness |
| `crates/reference` | Independent rule/currentness evaluator sharing only record validation, encoding, and crypto primitives |
| `crates/cache` | B2 bounded signature/decision caches, fresh time/head checks, maintenance counters |
| `crates/dependencies` | B3 reverse source/claim index, snapshot reconciliation, bounded metadata cleanup |
| `crates/replay` | Matched B1/B2/B3/reference checks over trusted ordered snapshots |
| `crates/simulation` | Deterministic environmental signals, claim-delivery faults, independent scenario expectations, offline playback |
| `crates/decision` | Paper-derived weighted calibration, role decisions, complete recommendation, crew resolution and governance |
| `crates/agents` | Legacy viewer plus the separate connected weighted inbox-driven workers |
| `crates/records` | V2 authentication, independent checks, weighted private claim, historical verification, reuse/dependencies and durable advisory authorization |
| `crates/cli` | New demo runs, bounded JSON input, machine-readable verification reports |
| `specs/protocol-v1.md` | Implemented provisional semantics and exact binary encodings |
| `specs/next-milestones.md` | Remaining work and baseline fairness constraints |
| `tests/vectors` | Fixed encoding/hash vector and standard signature-vector coverage |
| `tests/corpus/demo-v1` | Frozen synthetic bundles, trusted configuration, and expected outcomes |
| `workloads/b2-currentness-v1.json` | Frozen synthetic currentness and attack replay |
| `workloads/b3-multi-claim-v1.json` | Frozen synthetic multi-claim dependency replay |
| `docs` | Build context, hardware inventory, and procurement |

Python will supply acquisition, orchestration, and analysis later. We have not created empty sensor drivers or claimed hardware integration before selecting and receiving the boards. Existing Elsevier IoT source code has not been copied into this project.

## Verification

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

After dependencies are fetched, add `--offline` to test/build commands for repeatable local checks. Tests cover signed tampering, event/nonce replay, policy and registry changes, duplicate voters and sources, missing quorum, exact expiry boundaries, replacement checkpoints, malformed input, numerical bounds, canonical encoding, CLI exit codes, cache poisoning/eviction/restart, saved replays, and deterministic matched sweeps.

## Scope of this delivery

The provisional public rule is an equally weighted mean CO₂ threshold, calculated with exact integer sums, with every configured source required. This is an initial controlled fixture, not a full reproduction of original ADAM aggregation, reasoner, or governance behavior. The 800 ppm threshold is experimental and is not a safety limit.

Nominal mode requires two matching ballots from three registered logical voters. Explicit degraded mode requires both of two voters. The demo maps three logical voters onto two node labels; it does not demonstrate three physically independent voters or Byzantine fault tolerance.

For the legacy paths in this section, audit durability, private proofs and the new advisory gate are absent; use the connected weighted pipeline above for those local components. Alternative support, incremental fusion, exact B0 reproduction, production device provisioning, persistent acquisition replay state, live sensors and secure network services remain incomplete. B2 and B3 are currently in-memory libraries used by replay and simulation; separate `verify` invocations remain cold B1 checks. Currentness is checked against the caller's trusted snapshot; there is no atomic live check-and-release mechanism yet. A signed false reading can still pass authentication. The simulator transports pre-signed claim packets, not distributed planner/ballot negotiation.

See [BUILD_PLAN.md](BUILD_PLAN.md) for the larger checklist and [the next milestones](specs/next-milestones.md) for the implementation sequence.
