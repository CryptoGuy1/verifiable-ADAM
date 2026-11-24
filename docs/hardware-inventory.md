# Hardware inventory

Specifications below come from SSH command output supplied by the user. Runtime memory availability and CPU scaling are snapshots, not benchmark results. Device roles remain unassigned.

See the [experiment equipment and procurement register](experiment-procurement.md) for reusable equipment recovered from the earlier Elsevier IoT inventory, cooling purchases, sensor requirements, and unconfirmed cart items.

## Device 1

| Field | Reported value |
| --- | --- |
| Hostname | evidra |
| Board | Raspberry Pi 5 Model B Rev 1.0 |
| Board revision code | c04170 |
| Architecture | aarch64, little endian |
| CPU | 4 Cortex-A76 cores, 1 thread per core |
| CPU stepping | r4p1 |
| Reported CPU frequency range | 1500–2400 MHz |
| Frequency boost | Disabled in supplied lscpu output |
| Reported RAM | 4.0 GiB total |
| Available RAM at capture | 3.5 GiB |
| Swap | 2.0 GiB total, unused at capture |
| L1 data cache | 256 KiB total across 4 instances |
| L1 instruction cache | 256 KiB total across 4 instances |
| L2 cache | 2 MiB total across 4 instances |
| L3 cache | 2 MiB |
| OS | Debian GNU/Linux 13 (trixie), DEBIAN_VERSION_FULL 13.7 |
| Kernel release | 6.18.50+rpt-rpi-2712 |
| Kernel build | #1 SMP PREEMPT Debian 1:6.18.50-1+rpt1 (2026-09-11) |
| Storage device | mmcblk0, 29.7G as reported by lsblk |
| Boot partition | 512M, mounted at /boot/firmware |
| Root partition | mmcblk0p2, 29.2G as reported by lsblk |
| Root filesystem usage at capture | 29G total, 6.6G used, 21G available, 24% used |
| Swap device | zram0, 2G |
| Temperature at capture | 57.1 °C; workload and ambient conditions unspecified |
| Throttling flags at capture | 0x0 |
| Python | 3.13.5 |
| Rust toolchain | Not installed, per user; Cargo version not separately checked |
| Power adapter | User reports 5 V, “3000mh”; provisionally interpreted as 3000 mA (3 A), label and cable details unverified |
| Cooling | No cooling system installed, per user |

Source commands: `/proc/device-tree/model`, `/proc/cpuinfo`, `free -h`, `lscpu`, `cat /etc/os-release`, `uname -a`, `lsblk`, `df -h /`, `vcgencmd measure_temp`, `vcgencmd get_throttled`, and `python3 --version`. The board serial is omitted from this shared inventory. Values are transcribed from user-supplied output, without independent device access.

The displayed NUMA-node count is not treated as a count of physical processors or independently controlled devices. The CPU output reports four cores.

### Outstanding details

- [x] OS distribution and version: `cat /etc/os-release`.
- [x] Kernel: `uname -a`.
- [x] Storage devices and root filesystem capacity: `lsblk` and `df -h /`.
- [x] Temperature and recorded power/thermal flags: `vcgencmd measure_temp` and `vcgencmd get_throttled`.
- [x] User-reported adapter and cooling configuration recorded.
- [ ] Confirm adapter label units, brand/model, and cable configuration; validate power and temperature under load.
- [x] Python version: `python3 --version`.
- [ ] Install the selected Rust toolchain and record Rust/Cargo versions; Rust is not currently installed, per user.
- [ ] Sensor inventory, connections, acquisition settings, and calibration configuration.
- [ ] Network interface, connection type, and measured clock synchronization.

## Device 2

| Field | Reported value |
| --- | --- |
| Hostname | gateway |
| Board | Raspberry Pi 4 Model B Rev 1.5 |
| Architecture | aarch64, little endian |
| CPU | 4 Cortex-A72 cores, 1 thread per core |
| CPU stepping | r0p3 |
| Reported CPU frequency range | 600–1800 MHz |
| Frequency boost | Disabled in supplied lscpu output |
| Reported RAM | 7.6 GiB total |
| Available RAM at capture | 7.3 GiB |
| Swap | 2.0 GiB total, unused at capture |
| L1 data cache | 128 KiB total across 4 instances |
| L1 instruction cache | 192 KiB total across 4 instances |
| L2 cache | 1 MiB, 1 instance |
| OS | Debian GNU/Linux 13 (trixie), DEBIAN_VERSION_FULL 13.7 |
| Kernel release | 6.18.50+rpt-rpi-v8 |
| Kernel build | #1 SMP PREEMPT Debian 1:6.18.50-1+rpt1 (2026-09-11) |
| Storage device | mmcblk0, 30G as reported by lsblk |
| Boot partition | 512M, mounted at /boot/firmware |
| Root partition | mmcblk0p2, 29.5G as reported by lsblk |
| Root filesystem usage at capture | 29G total, 6.6G used, 22G available, 24% used |
| Swap device | zram0, 2G |
| Temperature at capture | 43.3 °C; workload and ambient conditions unspecified |
| Throttling flags at capture | 0x50000; historical undervoltage and throttling flags set, no current flags set |
| Python | 3.13.5 |
| Rust toolchain | Not installed, per user; Cargo version not separately checked |
| Power adapter | User reports 5 V, “3000mh”; provisionally interpreted as 3000 mA (3 A), label and cable details unverified |
| Cooling | No cooling system installed, per user |

Source commands: `/proc/device-tree/model`, `free -h`, `lscpu`, `cat /etc/os-release`, `uname -a`, `lsblk`, `df -h /`, `vcgencmd measure_temp`, `vcgencmd get_throttled`, and `python3 --version`, supplied by the user. The displayed NUMA-node count is not a count of independent devices. The supplied vulnerability report marks Speculative Store Bypass and Spectre v2 as vulnerable; this is recorded as system context, without changing the protocol's stated threat model or asserting a mitigation.

The `0x50000` interpretation follows the [official Raspberry Pi get_throttled bit definitions](https://www.raspberrypi.com/documentation/computers/os.html#get_throttled): bits 16 and 18 indicate past undervoltage and throttling. This reading does not establish the cause or timing of the events. Record the power supply and cable configuration and check power stability under representative load before collecting performance benchmarks.

### Outstanding details

- [x] OS distribution and version: `cat /etc/os-release`.
- [x] Kernel: `uname -a`.
- [x] Storage devices and root filesystem capacity: `lsblk` and `df -h /`.
- [x] Temperature and recorded power/thermal flags: `vcgencmd measure_temp` and `vcgencmd get_throttled`.
- [x] User-reported adapter and cooling configuration recorded.
- [ ] Confirm adapter label units, brand/model, and cable configuration; validate power and temperature under load.
- [x] Python version: `python3 --version`.
- [ ] Install the selected Rust toolchain and record Rust/Cargo versions; Rust is not currently installed, per user.
- [ ] Investigate historical undervoltage/throttling and establish power stability under benchmark load.
- [ ] Sensor inventory, connections, acquisition settings, and calibration configuration.
- [ ] Network interface, connection type, and measured clock synchronization.

## Build implications

Use the proposed Rust protocol core and Python acquisition/orchestration split. Validate dependencies on this aarch64 environment before freezing them. Measure actual latency, peak RAM, thermal behavior, and any swap activity on the devices; these specifications alone do not establish performance or optional ZKP feasibility.

The testbed contains a Pi 5 and a Pi 4 with different CPU and memory specifications. Run matched B2/B3 workloads on each device and swap claiming/verifying roles in crossover runs. Report per-device results and role placement; comparing B2 solely on one model with B3 solely on the other would confound implementation effects with hardware differences. Do not describe this setup as two Pi 5 devices.

## Power and cooling follow-up

The user previously operated a 9 g servo on the Pi 4. Servo-related power demand is a possible explanation for its historical flags, but the servo wiring, power source, and event timing are unknown; causation is not established.

The [official power guidance](https://www.raspberrypi.com/documentation/computers/getting-started.html) lists 5 V / 3 A for Pi 4. For Pi 5, Raspberry Pi [recommends a high-quality 5 V / 5 A supply](https://www.raspberrypi.com/products/raspberry-pi-5/); its [USB power delivery guidance](https://pip-assets.raspberrypi.com/categories/685-app-notes-guides-whitepapers/documents/RP-009856-WP-1-USB%20Power%20delivery%20on%20Raspberry%20Pi%205.pdf) explains that a 3 A supply can be used with lower-power peripherals. Adapter ratings alone do not establish delivered voltage stability.

Proceed with software development, then establish a documented power and cooling configuration before collecting confirmatory performance results. Consider a Pi 5 Active Cooler and a suitable fan/heatsink arrangement for Pi 4 for sustained workloads; [official hardware guidance](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html) recommends active cooling for best performance. Log temperatures, clock behavior, and throttling flags during pilot runs.
