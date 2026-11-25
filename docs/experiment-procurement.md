# Revised experiment purchase list

Updated 25 November 2026. This is the current shopping scope and supersedes the [earlier equipment register](archive/experiment-procurement-before-single-sensor-2026-11-25.md). No order has been placed. The user has selected two breadboards, one SCD40 and one BH1750 light sensor, with cooling packs removed.

## Items to buy

| Item | Quantity | Budget unit price | Line total | Supplier |
| --- | --- | --- | --- | --- |
| MB102 solderless breadboard, 830 points | 2 | NGN 2,200 | NGN 4,400 | [Hub360 breadboard](https://hub360.cc/shop/0082-breadboard-mb102-11890) |
| SCD40 CO2 temperature and humidity sensor breakout | 1 | NGN 49,000 regular price | NGN 49,000 | [Hub360 SCD40](https://hub360.cc/shop/2731-scd40-co2-temperature-and-humidity-sensor-16987) |
| GY-302 BH1750FVI digital light sensor breakout | 1 | NGN 3,700 catalogue budget | NGN 3,700 | [Hub360 BH1750](https://hub360.cc/shop/1202-gy-302-bh1750fvi-digital-light-sensor-module-11104) |

**Budget total: NGN 57,100 excluding delivery.** If Hub360 honours the advertised SCD40 promotional price of NGN 43,100, the total becomes **NGN 51,200** with the other budget prices unchanged.

Prices were reviewed on 25 November 2026 through supplier pages, not a checkout quotation. Hub360's [catalogue](https://hub360.cc/shop/page/151) lists the BH1750 at NGN 3,700, while an older product page shows NGN 3,200 and “Not Available For Sale”. The breadboard page shows NGN 2,200; the SCD40 page shows NGN 43,100 promotional and NGN 49,000 regular. Confirm stock, exact breakout, fitted headers, Pi-compatible electrical connections, final prices and delivery before payment. No seller contact or purchase has been made.

Cooling packs, a second CO2 sensor, a second light sensor, power upgrades, extra jumper wires and background tools are not part of this order. No replacement cooling purchase is added.

## Existing equipment to reuse

- Raspberry Pi 5 `evidra` and Raspberry Pi 4 `gateway`, with their installed cards and existing adapters.
- Development computer and the installed local Qwen model.
- Existing jumper wires and background equipment, including screwdriver, cutters, multimeter and card reader, confirmed available by the user.
- Previously purchased temperature sensor where useful; exact model remains unconfirmed. The SCD40 is still needed for direct CO2 sensing.

See [hardware inventory](hardware-inventory.md) for the user-supplied device specifications. Removing cooling from this purchase list does not record a cooling installation or hardware benchmark validation.

## Experiment plan using one live CO2 sensor

1. **Complete the integrated local software.** Develop weighted ADAM-style fusion, authenticated decision/action records, real private fused-classification proofs, dependency integration, audit/governance and matched comparisons using recorded or synthetic development inputs. These are planned additions, not capabilities completed by this procurement update.
2. **Bring up the real sensors.** Connect the SCD40 and BH1750 to one Pi and capture real CO2/light observations, with recorded model, units, timestamps and acquisition settings. Use the breadboards for easy prototyping across the two Pis. The second board does not imply a second live CO2 channel.
3. **Use dataset replay for additional channels.** Select suitable licensed datasets or replay our own saved observations. Record dataset source/version/license, original capture time, replay time and channel type. Retain the two-Pi deployment plan for sensing, agent services, aggregation and verification; agents do not each require a physical sensor.
4. **Separate experiment types.** Label runs as live acquisition, dataset replay, synthetic simulation or hybrid. Use identical frozen replay inputs for B1/B2/B3 and the equivalent indexed-cache control. Use the same committed inputs for the private proof comparison and record its added cost separately.
5. **Bound the claims.** A signature made during replay authenticates our replay process, not the original data acquisition. One live sensor plus replay is not two independent simultaneous sensors. Unrelated public data does not establish calibration weights for our SCD40. Continue with explicitly synthetic reference/calibration fixtures until suitable paired reference observations exist.

No paid dataset or cloud service is included in this budget; dataset selection and licensing checks remain open. A hybrid room demonstration must not present unrelated historical readings as simultaneous physical measurements of the current room.

## Protocol change

The original experimental refinement names two SCD41s and two BH1750s. The current purchase decision is one SCD40 and one BH1750, supported by datasets. Update the implementation experiment configuration and record this departure from the original physical protocol. Do not label SCD40 as SCD41 or assume an external client's approval of the revised validation design.

## Purchase and arrival checklist

- [ ] Obtain final supplier stock confirmation and itemized prices for exactly these quantities.
- [ ] Confirm breakouts have usable headers/connectors and documented Pi-compatible supply and I2C levels.
- [ ] Record delivery charges and final total before payment.
- [ ] On arrival, record model, quantity and sensor identifiers and perform a basic acquisition check.
- [ ] Preserve original dataset provenance and explicitly distinguish replay signatures from original acquisition signatures.

The updated Word memo is `output/docx/Verifiable_ADAM_Revised_Purchase_List_2026_11_25.docx`. The previous Word purchase list is retained as a historical artifact and is superseded by this revision.
