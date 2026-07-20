use dig_capsule_crypto::sha256;

#[test]
fn sha256_known_answer_abc() {
    // FIPS 180-2 test vector for "abc".
    let got = sha256(b"abc");
    let expected =
        hex::decode("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad").unwrap();
    assert_eq!(&got.0[..], &expected[..]);
}

#[test]
fn sha256_known_answer_empty() {
    let got = sha256(b"");
    let expected =
        hex::decode("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855").unwrap();
    assert_eq!(&got.0[..], &expected[..]);
}

#[test]
fn crate_advertises_its_version() {
    assert_eq!(dig_capsule_crypto::CRYPTO_VERSION, 1);
}

#[test]
fn sha256_of_canonical_urn_equals_retrieval_key() {
    use dig_capsule_core::Bytes32;
    use dig_urn_protocol::{Bytes32 as UrnBytes32, DigUrn};

    let urn = DigUrn {
        chain: "mainnet".to_string(),
        store_id: UrnBytes32([0x11; 32]),
        root_hash: None,
        resource_key: Some("file.txt".to_string()),
    };
    let canonical = urn.canonical();
    let direct: Bytes32 = dig_capsule_crypto::sha256(canonical.as_bytes());
    // `retrieval_key` is `SHA-256(canonical())` — same bytes, different newtype.
    let via_urn: [u8; 32] = urn.retrieval_key().0;
    assert_eq!(direct.0, via_urn);
}
