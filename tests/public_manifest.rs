//! The `readPublicManifest` wasm reader: decode a `.dig` data-section blob's
//! normalized public manifest to JSON, and tolerate its absence (older `.dig` /
//! private store).
//!
//! Exercises the actual `wasm` surface (`dig_capsule::wasm_browser::read_public_manifest`)
//! natively, so it is gated on the `wasm` feature. Run with
//! `cargo test --features wasm`. The success / absent paths construct no `JsError`,
//! so they run fine on the native target; the malformed-blob error path (a thrown
//! `JsError`) is only exercisable under wasm and is covered by
//! `imp::core::datasection::DataView::parse`'s own reject tests.

#![cfg(feature = "wasm")]

use dig_capsule::format::datasection::{encode_blob, encode_public_manifest, SectionId};
use dig_capsule::format::Bytes32;
use dig_capsule::metadata::{PublicManifest, PublicManifestEntry};
use dig_capsule::wasm_browser::read_public_manifest;

fn blob_with_manifest(pm: &PublicManifest) -> Vec<u8> {
    encode_blob(&[
        (SectionId::StoreId as u16, vec![1u8; 32]),
        (SectionId::PublicManifest as u16, encode_public_manifest(pm)),
    ])
}

#[test]
fn reads_embedded_manifest_as_json() {
    let pm = PublicManifest::new(vec![PublicManifestEntry {
        path: "index.html".into(),
        latest_root: Bytes32([0xab; 32]),
        generation_index: 3,
        sha256_latest: Bytes32([0xcd; 32]),
        version_count: 2,
    }]);
    let blob = blob_with_manifest(&pm);
    let json = read_public_manifest(&blob)
        .expect("valid blob")
        .expect("manifest present");
    assert!(json.contains("\"path\""));
    assert!(json.contains("\"index.html\""));
    assert!(json.contains("\"latest_root\""));
    assert!(json.contains(&"ab".repeat(32)));
    assert!(json.contains("\"generation_index\""));
    assert!(json.contains("\"sha256_latest\""));
    assert!(json.contains(&"cd".repeat(32)));
    assert!(json.contains("\"version_count\""));
    assert!(json.contains("\"schema_version\""));
}

#[test]
fn absent_manifest_is_null() {
    // A blob with no PublicManifest section (older .dig / private store) → None.
    let blob = encode_blob(&[(SectionId::StoreId as u16, vec![1u8; 32])]);
    assert!(read_public_manifest(&blob).expect("valid blob").is_none());
}
