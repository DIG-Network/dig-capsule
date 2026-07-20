//! Tests for the canonical `Capsule` identity = `(storeId, rootHash)`.
//!
//! A capsule is one immutable store generation; canonical string is
//! `storeId:rootHash` (lowercase hex : lowercase hex). See SYSTEM.md → capsule.

use dig_capsule_core::codec::{Decode, Encode};
use dig_capsule_core::{Bytes32, Capsule};
use dig_urn_protocol::DigUrn;

fn store_id() -> Bytes32 {
    Bytes32([0x11; 32])
}
fn root_hash() -> Bytes32 {
    Bytes32([0x22; 32])
}

/// The URN → [`Capsule`] bridge (equivalent of the facade's `capsule_from_urn`):
/// a rooted URN names the `(store_id, root_hash)` capsule; a rootless one names none.
fn capsule_from_urn(urn: &DigUrn) -> Option<Capsule> {
    urn.root_hash.map(|root_hash| Capsule {
        store_id: Bytes32(urn.store_id.0),
        root_hash: Bytes32(root_hash.0),
    })
}

#[test]
fn canonical_is_store_id_colon_root_hash() {
    let cap = Capsule {
        store_id: store_id(),
        root_hash: root_hash(),
    };
    assert_eq!(
        cap.canonical(),
        format!("{}:{}", store_id().to_hex(), root_hash().to_hex())
    );
}

#[test]
fn canonical_roundtrips_from_canonical() {
    let cap = Capsule {
        store_id: store_id(),
        root_hash: root_hash(),
    };
    let s = cap.canonical();
    let back = Capsule::from_canonical(&s).expect("parse canonical");
    assert_eq!(back, cap);
}

#[test]
fn display_matches_canonical() {
    let cap = Capsule {
        store_id: store_id(),
        root_hash: root_hash(),
    };
    assert_eq!(format!("{cap}"), cap.canonical());
}

#[test]
fn from_canonical_rejects_missing_colon() {
    // A single segment (no ':') is not a capsule.
    assert!(Capsule::from_canonical(&store_id().to_hex()).is_err());
}

#[test]
fn from_canonical_rejects_three_segments() {
    let s = format!(
        "{}:{}:{}",
        store_id().to_hex(),
        root_hash().to_hex(),
        root_hash().to_hex()
    );
    assert!(Capsule::from_canonical(&s).is_err());
}

#[test]
fn from_canonical_rejects_short_hex() {
    let s = format!("{}:{}", "11", root_hash().to_hex());
    assert!(Capsule::from_canonical(&s).is_err());
}

#[test]
fn from_canonical_rejects_long_hex() {
    let s = format!("{}:{}", store_id().to_hex(), root_hash().to_hex() + "00");
    assert!(Capsule::from_canonical(&s).is_err());
}

#[test]
fn from_canonical_rejects_non_hex() {
    let s = format!("{}:{}", "zz".repeat(32), root_hash().to_hex());
    assert!(Capsule::from_canonical(&s).is_err());
}

#[test]
fn from_canonical_rejects_empty_segment() {
    // Trailing colon → empty second segment.
    let s = format!("{}:", store_id().to_hex());
    assert!(Capsule::from_canonical(&s).is_err());
    // Leading colon → empty first segment.
    let s = format!(":{}", root_hash().to_hex());
    assert!(Capsule::from_canonical(&s).is_err());
}

#[test]
fn capsule_codec_roundtrips() {
    let cap = Capsule {
        store_id: store_id(),
        root_hash: root_hash(),
    };
    let bytes = cap.to_bytes();
    let back = Capsule::from_bytes(&bytes).expect("decode");
    assert_eq!(back, cap);
}

#[test]
fn capsule_codec_is_two_raw_bytes32() {
    // Capsule encoding mirrors Urn's field-by-field codec: two raw Bytes32, no
    // length prefix → exactly 64 bytes.
    let cap = Capsule {
        store_id: store_id(),
        root_hash: root_hash(),
    };
    let bytes = cap.to_bytes();
    assert_eq!(bytes.len(), 64);
    assert_eq!(&bytes[0..32], &[0x11; 32]);
    assert_eq!(&bytes[32..64], &[0x22; 32]);
}

// --- URN → Capsule bridge ---

#[test]
fn urn_with_root_yields_capsule() {
    let sid = store_id().to_hex();
    let rh = root_hash().to_hex();
    let urn = DigUrn::parse(&format!("urn:dig:mainnet:{sid}:{rh}/a/b")).unwrap();
    let cap = capsule_from_urn(&urn).expect("urn with root has a capsule");
    // The capsule's canonical string equals the `storeId:rootHash` portion of the
    // URN's canonical string.
    assert_eq!(cap.canonical(), format!("{sid}:{rh}"));
    assert_eq!(cap.store_id, store_id());
    assert_eq!(cap.root_hash, root_hash());
}

#[test]
fn rootless_urn_yields_no_capsule() {
    let sid = store_id().to_hex();
    let urn = DigUrn::parse(&format!("urn:dig:mainnet:{sid}/index.html")).unwrap();
    assert!(capsule_from_urn(&urn).is_none());
}
