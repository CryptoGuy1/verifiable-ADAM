use serde::Deserialize;
use veri_adam_protocol::*;

#[derive(Deserialize)]
struct Vector {
    measurement: Measurement,
    canonical_hex: String,
    sha256: String,
}

#[test]
fn independent_binary_encoding_vector() {
    let vector: Vector =
        serde_json::from_str(include_str!("../../../tests/vectors/measurement-v1.json")).unwrap();
    vector.measurement.validate().unwrap();
    assert_eq!(
        hex::encode(vector.measurement.canonical_bytes()),
        vector.canonical_hex
    );
    assert_eq!(vector.measurement.id(), vector.sha256);
}

#[test]
fn rfc8032_empty_message_signature_vector() {
    // RFC 8032 section 7.1, TEST 1. Public standard test material, not a device key.
    let seed =
        decode::<32>("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60").unwrap();
    let public = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    let signature = concat!(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155",
        "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
    );
    assert_eq!(public_key(&seed), public);
    assert_eq!(sign(b"", &seed), signature);
    verify(b"", signature, public).unwrap();
    assert!(verify(b"changed", signature, public).is_err());
}

#[test]
fn lower_case_fixed_width_hex_and_identifier_bounds() {
    assert!(decode::<32>(&"A0".repeat(32)).is_err());
    assert!(decode::<32>(&"00".repeat(31)).is_err());
    assert!(identifier("a\n").is_err());
    assert!(identifier(&"x".repeat(65)).is_err());
    assert!(identifier("pi-a:sensor_1").is_ok());
}
