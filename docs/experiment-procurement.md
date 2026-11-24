# Experiment equipment and procurement

Prepared 24 November 2026 for the two-device verifiable-ADAM CO₂ and light experiments. This register combines the current hardware inventory with earlier Elsevier IoT records and chat history. Earlier cart entries are not evidence of delivery. Purchase quantities are shortfalls after suitable existing stock is checked; no order has been placed.

## Current purchase scope and Hub360 shortlist

The user has confirmed enough jumper wires and ownership of background tools, including screwdrivers, cutters, a multimeter, and a card reader. The immediate purchase scope is ONLY two breadboards, two CO₂ breakouts, two BH1750 breakouts, and cooling for each Pi. Earlier broader tables below describe the complete bench and conditional options, not additional current purchase requests. A Pi 5 supply upgrade is deferred from this shopping scope.

Hub360 listings checked 24 November 2026. Web results include differently dated cached pages and conflicting prices; these are indicative listings, not a confirmed quotation or live stock count.

| Item | Quantity | Observed price per item | Indicative line total | Listing and qualification |
| --- | --- | --- | --- | --- |
| MB102 breadboard, 830 points | 2 | ₦2,200 regular | ₦4,400 | [Product](https://hub360.cc/shop/0082-breadboard-mb102-11890). A separate cached result shows a ₦1,750 promotion; budget at regular price. |
| SCD40 CO₂ breakout candidate | 2 | ₦43,100 promotional / ₦49,000 regular | ₦86,200–₦98,000 | [Product](https://hub360.cc/shop/2731-scd40-co2-temperature-and-humidity-sensor-16987). SCD40 is a proposed substitution, not the protocol's SCD41; confirm actual breakout, headers, safe I²C levels, and available quantity. No SCD41 listing was located in this search. |
| GY-302 BH1750FVI light breakout | 2 | ₦3,700 catalogue / ₦3,200 older product page | Budget ₦7,400 | [Product](https://hub360.cc/shop/1202-gy-302-bh1750fvi-digital-light-sensor-module-11104). The catalogue lists it, but an older product page says Not Available For Sale. Price and stock require confirmation. |
| Pi 5 radiator cooler with PWM fan | 1 | ₦14,000 | ₦14,000 | [Product](https://hub360.cc/shop/2674-raspberry-pi-5-radiator-cooler-active-aluminum-heatsink-with-pwm-fan-for-pi-5-performance-cooling-16930). Listing specifies Pi 5 mounting and four-pin fan header. |
| Pi 4 aluminium case with dual cooling fans | 1 | ₦19,000 | ₦19,000 | [Product](https://hub360.cc/shop/0711-raspberry-pi-4-aluminum-case-with-dual-cooling-fan-metal-shell-10613). Confirm GPIO jumper access and inclusion of fan wiring, thermal interfaces, and mounting parts. |

Indicative total using regular breadboard price, ₦3,700 per BH1750, and the displayed cooler prices: **₦131,000 with promotional SCD40 pricing, or ₦142,800 with regular SCD40 pricing**, excluding delivery. This assumes the SCD40 substitution is selected and both light sensors can be supplied.

Hub360 describes its SCD40 as covering 400–5000 ppm, but [Sensirion specifies its SCD40 accuracy range as 400–2000 ppm](https://sensirion.com/products/catalog/SCD40). Use manufacturer specifications when deciding the substitution and update the protocol/model inventory if SCD40 is selected. Do not label purchased SCD40 units as SCD41.

## Complete working equipment list

Quantities below are total equipment to use, including existing stock. The user has requested breadboards for easy connections; plan two independent sensor breadboards, one per Pi. A 1660-point bench is acceptable if it provides two usable 830-point boards. Wired Ethernet is the proposed benchmark configuration; existing Wi-Fi can support setup and development.

| Item | Total quantity | Purpose and stock decision |
| --- | --- | --- |
| Raspberry Pi 5 | 1 | Existing evidra; run all verification modes and crossover roles. |
| Raspberry Pi 4 | 1 | Existing gateway; run all verification modes and crossover roles. |
| Installed microSD cards | 2 | Reuse current cards for OS, services, and routine logs. |
| Development laptop | 1 | Existing workstation for SSH, builds, orchestration, analysis, and backups. |
| Pi 5 cooling assembly | 1 | Buy dedicated fan/heatsink assembly with mounting and thermal interface. |
| Pi 4 cooling assembly | 1 | Buy board-compatible fan/heatsink assembly. |
| Protective Pi cases or nonconductive mounting bases | 2 | Reuse or buy; compatible with cooling and accessible GPIO. Do not duplicate cases included with cooling kits. |
| Pi 5 power supply | 1 | Recommended 27 W compatible supply upgrade; retain old adapter as spare. |
| Pi 4 power supply | 1 | Validate and reuse existing adapter, replacing only if unsuitable. |
| SCD41 sensor breakouts | 2 | One per Pi; CO₂, temperature, humidity. Buy unless suitable stock is identified. |
| BH1750 sensor breakouts | 2 | One per Pi; light experiments. Buy unless suitable stock is identified. |
| Solderless breadboards | 2 × approximately 830 points | One per Pi; reuse earlier bench if received and suitable. |
| Female-to-male jumper wires | 1 pack of approximately 40 | Pi header to breadboard; top up existing stock. |
| Male-to-male jumper wires | 1 pack of approximately 40 | Breadboard distribution and links; reuse existing stock. |
| Female-to-female jumper wires | 1 pack of approximately 40 | Connections to fitted male sensor headers where needed; reuse existing stock. |
| Sensor connector/header adapters | Enough for 4 breakouts, if required | Select breakouts with fitted breadboard-compatible headers, or order matching connector-to-breadboard adapters. Do not buy both complete wiring alternatives unnecessarily. |
| LED desk lamp | 1 | Reuse or buy; controlled light changes. |
| Opaque light cover and positioning marks | 1 set | Repeatable light blocking without moving sensor mounts. |
| Cat5e/Cat6 patch cables | 2–3 | Two Pis plus administration/uplink as required; reuse suitable cables. |
| Router LAN ports or powered Gigabit switch | 1 network arrangement | Reuse existing network ports; buy a switch only for a port shortfall. |
| Laptop Ethernet adapter | 1 if needed | Only if the laptop lacks a suitable wired interface. |
| Digital multimeter | 1 | Voltage and continuity checks; earlier receipt unconfirmed. |
| microSD reader | 1 shared | Identify existing adapter before buying. |
| Small screwdriver, cutters, wire stripper | 1 shared tool set | Reuse or buy missing tools. |
| Labels, standoffs, ties, sensor supports | 1 shared mounting kit | Fix and identify sensor locations away from Pi heat and direct breath. |
| Power strip | 1 | Enough sockets for both Pis, network equipment, laptop supply, and lamp. |

Breadboard wiring must follow the selected breakout's documented supply and I²C levels. Prefer pre-soldered headers to avoid an additional soldering purchase. Each Pi has its own sensor bus and breadboard; do not join the two Pis' power rails. Check breadboard rail continuity and any split rails before use. Power cooling assemblies through their documented connections, not the sensor breadboard rails.

Software to use: 64-bit Linux and Python on both Pis (already present); a pinned Rust/Cargo toolchain (not installed yet); Git and SSH; a Python virtual environment with the selected sensor drivers and analysis dependencies; a time-synchronization service; and this project's verifier, cache/dependency strategies, replay runner, logging, and analysis tools. Dependency versions and exact packages remain to be selected and pinned. No paid cloud service is required for the initial experiment.

## Equipment to reuse

| Equipment | Recorded quantity | Evidence and disposition |
| --- | --- | --- |
| Raspberry Pi 5, evidra | 1 | Current SSH output; four Cortex-A76 cores, 4.0 GiB reported RAM. Reuse. |
| Raspberry Pi 4, gateway | 1 | Current SSH output; four Cortex-A72 cores, 7.6 GiB reported RAM. Reuse. |
| Installed microSD cards | 2 | Current block-device output; approximately 30G per device. Reuse; no capacity upgrade currently justified. |
| Pi power adapters | 2 | User reports 5 V and “3000mh”, provisionally interpreted as 3 A. Check labels and cable quality. See power decisions below. |
| ESP32 development boards | 2 | Earlier inventory photos reviewed in the chat “Review the purchase list.” Retain as spare equipment; no new ESP32 required. |
| Jumper wires | 2 bags plus previously recorded loose wires | Earlier photo review identifies two bags. The earlier testbed notes specify male–male and female–female packs plus three male–female wires. Exact current connector counts remain unresolved; buy only missing connections. |
| Previously purchased temperature sensor | At least 1; exact count unconfirmed | User confirms purchase. Earlier cart specified MCP9808, but exact delivered model remains unverified. Retain and reuse where useful; do not order a separate temperature-only sensor for the core CO₂ experiment. |
| Small blue breakout boards | 3 | Earlier photo review; one close-up had HV/LV/GND markings consistent with a logic-level converter. Other identities remain unresolved. Do not count them as CO₂ or temperature sensors. |
| Small USB adapter | 1 | Earlier photo review; function unresolved. Check whether it reads microSD before buying a reader. |
| 9 g servo | At least 1 previously used | User reports prior operation on Pi 4. Keep in legacy stock; not required for the present sensing/verification protocol. |
| Small fan discussed in this chat | Quantity and rating unconfirmed | HEIC photo could not be decoded reliably. Do not count it as an installed or adequate cooling solution. |

No cooling system is currently installed on either Pi, per the user. Neither the old stock nor the old cart establishes ownership of SCD41 or BH1750 sensors.

The SCD41 purchase is for direct CO₂ measurements required by the supplied CO₂ protocol; temperature and humidity are additional integrated channels. An existing temperature-only sensor cannot supply CO₂ observations. The user also mentioned SCD40: it is a different CO₂ sensor variant, not the current protocol's selected SCD41. Any substitution needs an explicit specification review before ordering. See Sensirion's [SCD40](https://sensirion.com/products/catalog/SCD40) and [SCD41](https://sensirion.com/products/catalog/SCD41) descriptions. Temperature/light-only development can proceed with available equipment, while physical CO₂ collection waits for an appropriate CO₂ sensor.

## Core purchase requirements

| Item | Total target | Purchase decision | Specification and experiment purpose |
| --- | --- | --- | --- |
| Pi 5 active cooling | 1 set | BUY | Official Pi 5 Active Cooler, or an equivalent documented Pi 5 cooling assembly. Select one solution, with thermal interface and mounting hardware. Keep GPIO accessible. |
| Pi 4 active cooling | 1 set | BUY | Pi 4-specific heatsink and fan assembly, optionally in a ventilated case. Confirm mounting, fan supply/current, and GPIO access. |
| Pi 5 power supply upgrade | 1 | Recommended BUY | Official 27 W USB-C supply or a documented compatible supply supporting the Pi 5's 5 V / 5 A requirement for full power capability. Correct regional mains plug. Existing 3 A adapter can remain a spare. |
| Pi 4 power supply | 1 suitable supply | REUSE IF VALIDATED | Existing reported 5 V / 3 A rating matches the recommended class. Historical undervoltage requires a cable/supply check under representative load; replace if unstable. |
| SCD41 CO₂/temperature/humidity breakout | 2 | BUY unless already held | One per Pi. Select a documented Pi-compatible breakout with safe I²C logic, fitted connectors or soldered headers, and a published driver. Raw sensor and breakout listings are not interchangeable purchases. |
| BH1750 light breakout | 2 | BUY unless already held | One per Pi for repeatable light transitions. Select a documented Pi-compatible I²C board with fitted connectors or soldered headers. |
| Sensor connections | 2 breadboard wiring sets | BUY SHORTFALL | Use fitted breakout headers and existing jumpers where possible. If selecting connector-only QT/Qwiic boards, add the matching breadboard adapters/cables. Confirm the complete wiring path before ordering; a separate full daisy-chain cable kit is not automatically required. |
| LED desk lamp | 1 | REUSE OR BUY SHORTFALL | Ordinary stable lamp for light scenarios, plus an opaque cover and fixed positioning marks. Dedicated precision light equipment is not initially required. |
| Nonconductive mounts and labels | 1 bench set | REUSE OR BUY SHORTFALL | Fix both sensor locations away from Pi exhaust, direct breath, and the lamp's heat. Include standoffs, cable ties, board labels, and sensor inventory labels. |

Cooling and supply choices should be frozen and recorded before confirmatory performance collection. Pi 5 and Pi 4 accessories must match their respective boards. A case with an included adequate cooling assembly is an alternative to a separate cooler; avoid buying duplicate cooling kits without a compatibility reason.

Official accessory references: [Pi 5 Active Cooler](https://www.raspberrypi.com/products/active-cooler/), [Pi 5 fan-equipped case](https://www.raspberrypi.com/products/raspberry-pi-5-case/), [27 W supply](https://www.raspberrypi.com/products/27w-power-supply/), and [Pi hardware cooling guidance](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html). Sensor examples: [SCD41 breakout](https://www.adafruit.com/product/5190) and [BH1750 breakout guide](https://learn.adafruit.com/adafruit-bh1750-ambient-light-sensor/overview). These are compatibility references, not local supplier quotations.

## Bench and network shortfalls to check

| Item | Target | Decision |
| --- | --- | --- |
| Ready-made Cat5e/Cat6 patch cables | 2 for Pis, plus 1 if laptop/uplink needs it | Earlier list requested three; delivery not established. Buy only the missing cables if using wired LAN. |
| Router LAN ports or powered Gigabit switch | Enough ports for both Pis and administration/uplink | Reuse the current network if adequate. Buy a small unmanaged switch only if necessary; no new internet subscription is required. |
| Laptop Ethernet interface | 1 if wired laptop connection is selected | Reuse built-in interface or existing USB adapter if suitable; do not assume the unidentified adapter provides Ethernet. |
| Digital multimeter | 1 | Earlier purchase request only. Reuse if available; otherwise add a basic reputable meter for voltage and continuity checks. |
| Screwdrivers, cutters, wire stripper | 1 shared set | Earlier purchase request only. Buy missing tools; connector-based sensor boards reduce assembly needs. |
| Breadboards | 2 × approximately 830 points | Included by user request. Reuse the earlier 1660-hole bench if received and it supplies two suitable boards; otherwise buy the shortfall. |
| microSD reader | 1 shared | Identify existing USB adapter first; buy only if missing. |
| Power strip | 1 suitable strip | Reuse if enough sockets are available for the Pis, network equipment, and lamp. |

Wi-Fi is also permitted by the experimental protocol. Record the actual network configuration, disruptions, and timing rather than silently carrying over the previous project's wired-only assumption.

## Optional purchases tied to specific claims

| Item | Quantity | Buy only when |
| --- | --- | --- |
| Logging USB-C power measurement equipment | 1–2 measurement paths | Reporting directly measured energy. Verify logging rate, calibration, voltage/current range, and USB-C negotiation compatibility; the meter must not reduce the Pi 5 supply capability. Sequential paired runs may use one instrument; simultaneous per-device measurement needs two. |
| Independent reference CO₂ monitor | 1 | Claiming physical CO₂ measurement accuracy, with documented reference accuracy and a comparison procedure. Protocol correctness alone does not require it. |
| Temperature/humidity reference instrument | 1 | Claiming accuracy of those physical channels. |
| Logic analyser | 1 | I²C debugging cannot be resolved with software checks; verify 3.3 V input compatibility. |
| Spare card, sensor, or backup power | As justified | Replacement or documented continuity needs justify the cost. |

Without appropriate energy measurements, report timing, CPU, RAM, and traffic; do not present estimated energy as direct measurement.

## Earlier cart items outside the current core experiment

The 17 October purchase document lists MCP9808 sensors ×3, a breadboard bench ×1, positional servos ×2, microswitches ×4, LEDs/resistors, and additional actuator equipment. It also requests a PCA9685, fan driver, separate actuator supply, cutoff/fuse, buttons, and model mechanisms. Those are purchase requirements or cart changes, not confirmed deliveries.

Retain any delivered items in legacy stock. They do not replace SCD41/BH1750 sensing and are not additional purchases for the current protocol. No methane sensor, gas source, door/damper mechanism, new Pi, GPU, accelerator, or extra ESP32 is needed for the initial scope. A cooling fan is a host accessory, separate from the old experiment's controlled fan actuator.

## Procurement checklist

- [x] Carry forward the two Pis, cards, adapters, and earlier available-stock descriptions.
- [x] Separate earlier cart/order requests from evidence of ownership.
- [ ] Identify existing breakout boards and USB adapter; reconcile delivered items without assuming cart completion.
- [ ] Confirm adapter labels, connector counts, mounting needs, and the current network path.
- [ ] Select exact cooling and sensor board models, including required cables and assembly work.
- [ ] Obtain itemized supplier quotations with unit price, shipping, availability, and purchase shortfall. No budget total is established yet.
- [ ] Buy core shortfalls first; add measurement instruments only for the claims we will evaluate.
- [ ] On arrival, record model, quantity, sensor inventory ID, receipt date, and a basic functional check.
- [ ] Verify power and cooling under representative workloads and record settings for both devices.

## Record sources

- [Current hardware inventory](hardware-inventory.md): SSH outputs and user reports in this chat.
- [Earlier Pi testbed notes](/Users/greatadams/Documents/ChatGPT/Elsevier%20IoT/docs/raspberry-pi-testbed.md): explicitly owned equipment and proposed requirements.
- Earlier chat “Review the purchase list” (`01a1498a-432e-7821-955c-6ecb0b46520e`): inventory-photo review and cart review, retrieved for this reconciliation.
- Earlier `Updated Raspberry Pi Testbed Purchase List.docx`, dated 17 October 2026, in the Elsevier IoT project's docs directory: cart corrections and proposed additions.
- User-supplied `veri_ADAM_CO2_Experimental_Protocol.docx`: two SCD41s, two BH1750s, natural indoor CO₂, controlled light, and optional energy/reference measurement.
