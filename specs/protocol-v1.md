# Provisional protocol version 1

This document specifies the implemented first software milestone. It is an experimental starting contract, not the final preregistered protocol. The independent reference and production checker must agree on the frozen corpus before later strategies are compared.

## Trusted inputs and record identities

The claimant provides a bundle containing a signed decision, signed measurements, and signed ballots. The verifier separately receives a policy, a registry, and a request. The claimant must not control those trusted inputs.

The registry maps each unique signer ID to a node label, exactly one role, an Ed25519 public key, and a revocation flag. Public keys must be unique, valid, non-weak points. Multiple logical roles on one node use separate keys in this fixture. Node labels are trusted administrative metadata, not cryptographic evidence of independent physical devices. Registry version 0, empty registries, duplicate identities/keys, and more than 32 members are rejected.

The registry version is a trusted snapshot identifier. The operator must not assign different registry contents to the same version. Key distribution, version authenticity, revocation transport, hardware keys, and private key storage are later work. Historical checking requires an appropriate trusted historical registry/policy snapshot; it does not bypass a revocation in the supplied snapshot.

Each request specifies an expected event ID and 32-byte nonce. Measurements, the decision, and ballots bind to that context. Cross-event or cross-nonce reuse is rejected. Repeating an identical verification request is idempotent and is permitted; the software does not implement a consumed-nonce database or single-use action authorization. Production nonce generation and durable replay state are not yet provided.

## Records and values

All JSON structs reject unknown fields. Duplicate named struct fields fail deserialization. Input files are bounded to 1 MiB by the CLI; this is a file-based prototype, not a network parser. A library caller must enforce its own input-size limits. JSON member order and whitespace are not signed.

Identifiers contain 1–64 ASCII letters, digits, or `-_.:`. Hashes, keys, signatures, and nonces use fixed-width lowercase hexadecimal. Hashes, keys, and nonces are 32 bytes; signatures are 64 bytes. Protocol version is 1. Policy, registry, sample sequence, and calibration versions are positive integers.

| Measurement field | Meaning and representation |
| --- | --- |
| `event_id`, `nonce` | Signed event challenge context |
| `signer_id` | Registered sensor-role identity; node is resolved from trusted registry |
| `sequence` | Positive sample sequence, authenticated but not yet compared to persistent per-device history |
| `captured_at_ms` | Unsigned millisecond timestamp; intended as Unix UTC milliseconds for future live acquisition. Synthetic fixtures use a labeled virtual epoch. |
| `co2_ppm` | Unsigned integer, 0–40,000 protocol encoding bound; this is not a sensor accuracy specification |
| `temperature_millic` | Signed integer thousandths of °C, −40,000 through 125,000 encoding bound |
| `humidity_millipercent` | Unsigned integer thousandths of percent RH, 0–100,000 |
| `light_millilux` | Unsigned 32-bit integer thousandths of lux |
| `calibration_version` | Positive metadata version; calibration quality is not verified |
| `policy_hash` | SHA-256 of the canonical policy bytes |

The measurement ID is SHA-256 of its canonical payload bytes, excluding its signature. The signature authenticates those same bytes. A hash is a public integrity identifier, not a hiding commitment or a ZKP. No private-reading format exists yet.

The decision contains the event context, policy hash, registry version, coordinator identity, strictly sorted unique measurement IDs, and a boolean advisory. Its hash excludes its signature. Ballots sign their own context, decision hash, voter identity, and advisory. This ordering avoids a circular decision/ballot hash. The final matching ballot count is recomputed, not trusted from a coordinator summary. Adding or removing ballots changes the bundle's available quorum evidence without changing the signed proposal.

## Deterministic rule and quorum

The policy explicitly lists required sensor identities, eligible voters, coordinator, threshold, evidence TTL, crew mode, and versions. All policy roles must be authorized and unrevoked in the trusted registry. There must be exactly one signed measurement per required source; no partial sensor support or substituted source is allowed in this milestone.

Compute `sum(co2_ppm) >= threshold_ppm * number_of_sources` using unsigned 64-bit integers. Equality produces an advisory. All sources have equal weight; floating-point mean rounding is avoided. A false advisory value is a valid no-advisory classification, not a verification failure. Missing sources are rejected rather than treated as low CO₂.

The coordinator must sign the recomputed classification. Every supplied ballot must have a valid signature, authorized unique voter, and matching decision/event context. A signed dissenting opinion is allowed, but does not count toward the proposed classification. An invalid extra ballot is rejected even when enough other ballots agree.

Nominal mode has exactly three eligible voters and requires two matching ballots. Degraded mode must be explicitly selected in the trusted policy, has exactly two voters, and requires both. Missing or disagreeing ballots never reduce the required count. This tests quorum semantics only; the simple fusion rule does not implement the full original ADAM reasoner or aggregation policy.

## Currentness and outcomes

Verifier time is supplied by the trusted request. A future captured timestamp is rejected. Expiry is `captured_at_ms + policy.ttl_ms`, using checked arithmetic; overflow is rejected. Evidence is fresh strictly before expiry, and expired exactly at expiry. No clock-skew allowance is implemented yet.

For current checks, `heads` maps each required sensor identity to its authenticated latest accepted measurement ID. It must be maintained by a trusted acquisition/checkpoint layer, not filled from an unverified claim. Missing or mismatched required heads cause UNCONFIRMED. Unrelated heads do not affect acceptance. A replacement requires a newly signed and consistently bound bundle; B1 does not update a proposal automatically.

Validation proceeds through configuration, context, signatures, sources, classification, and ballots before deciding currentness. Thus a forged expired bundle is REJECTED, not merely EXPIRED. Insufficient matching ballots take precedence over expiry and produce UNCONFIRMED.

| Outcome | Meaning |
| --- | --- |
| `verified_current` | All processing checks pass, quorum is sufficient, required evidence is fresh and matches trusted heads |
| `verified_historical` | All processing checks and quorum pass; expiry and present-day replacement do not establish current support |
| `expired` | Processing/quorum checks pass, but required evidence has reached its TTL in a current check |
| `unconfirmed` | Matching quorum is insufficient, or a current required checkpoint is missing/replaced |
| `rejected` | Structure, authorization, signature, event binding, source set, classification, time bounds, or policy binding fails |

The support horizon is the minimum expiry over all required measurements. No physical action is released. A live authority service will need atomic snapshot checking, monotonic scheduling, and invalidation around release; this offline checker cannot guarantee those properties.

## Canonical binary encoding

Integers are big-endian, fixed-width, with no padding. Signed temperature uses big-endian two's complement i32. A boolean is u8 0 or 1. A string is u32 byte length followed by UTF-8 bytes. A string list is u32 element count followed by strings in the given order. Hex fields are encoded as their lowercase ASCII strings, including their length prefixes, not decoded binary. The length-prefixed domain string always comes first.

| Record domain | Subsequent fields in exact order |
| --- | --- |
| `veri-adam/policy/v1` | protocol_version u16; policy_version u64; threshold_ppm u32; ttl_ms u64; sources string list; voters string list; coordinator string; crew_mode u8 (0 nominal, 1 degraded) |
| `veri-adam/measurement/v1` | protocol_version u16; event_id string; nonce string; signer_id string; sequence u64; captured_at_ms u64; co2_ppm u32; temperature_millic i32; humidity_millipercent u32; light_millilux u32; calibration_version u64; policy_hash string |
| `veri-adam/decision/v1` | protocol_version u16; event_id string; nonce string; policy_hash string; registry_version u64; coordinator_id string; measurement_ids string list; advisory u8 |
| `veri-adam/ballot/v1` | protocol_version u16; event_id string; nonce string; decision_hash string; voter_id string; advisory u8 |

Signatures use Ed25519 over these complete bytes and strict verification from ed25519-dalek. IDs use SHA-256. The fixed measurement vector under `tests/vectors` was encoded independently with Python's `struct` and `hashlib`; the Rust test compares complete bytes and digest. A second test checks the public RFC 8032 empty-message signature vector. Demo seeds are public deterministic test material and must never be provisioned onto actual devices.

Implementation references: [ed25519-dalek strict verification](https://docs.rs/ed25519-dalek/3.0.0/ed25519_dalek/struct.VerifyingKey.html#method.verify_strict) and [RFC 8032 test vectors](https://www.rfc-editor.org/rfc/rfc8032#section-7.1).
