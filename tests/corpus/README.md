# Synthetic protocol corpus

`demo-v1` contains public, deterministic software fixtures, not acquired sensor readings or hardware experiment results. All signing seeds used to create them are intentionally public in `crates/protocol/src/demo.rs`. Never provision those seeds on a device.

`expected.json` lists independently specified acceptance outcomes and CLI exit codes. The normal bundle's two readings are 900 and 850 ppm, each captured at virtual millisecond 1000. Equal-weight fusion exceeds the experimental 800 ppm threshold. The TTL is 10,000 ms, giving expiry at 11,000 ms. Three registered logical ballots agree, with a required quorum of two. Those voters are mapped to two node labels only.

The changed bundle alters one signed reading without signing again, so it must be rejected. The expiry request checks the authentic bundle exactly at 11,000 ms and must expire it. The historical request at the same time verifies past processing only, with no current-support claim.

The CLI regression suite consumes these committed files directly. The broader adversarial suite constructs mutations and checks hand-declared outcomes against production and reference paths. Its deterministic sweep checks 64 reading configurations at six time points. This suite is a development corpus, not the held-out confirmatory dataset.
