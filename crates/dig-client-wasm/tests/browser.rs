//! Real-browser smoke test for the `@dignetwork/dig-client` wasm surface.
//!
//! Runs the compiled wasm-bindgen exports inside a headless browser
//! (`wasm-pack test --headless --chrome`), proving the read-crypto contract the
//! browser consumers (the on.dig.net loader, hub.dig.net) depend on works in a
//! genuine browser runtime -- not just natively. The Node CommonJS entry is
//! covered separately by `scripts/verify-pkg.mjs`, and the full proof-gated
//! decrypt round-trip by the native `parity` oracle.

#![cfg(target_arch = "wasm32")]

use dig_client_wasm::{derive_key, reconstruct_urn, retrieval_key, verify_inclusion, version};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

const STORE_ID: &str = "ab00000000000000000000000000000000000000000000000000000000000000";

#[wasm_bindgen_test]
fn version_is_nonempty_in_browser() {
    assert!(
        !version().is_empty(),
        "version() must return a semver string"
    );
}

#[wasm_bindgen_test]
fn urn_and_retrieval_key_render_in_browser() {
    let urn = reconstruct_urn(STORE_ID, "index.html").expect("urn");
    assert!(urn.contains(STORE_ID), "URN carries the store id: {urn}");
    let rk = retrieval_key(STORE_ID, "index.html").expect("retrieval key");
    assert_eq!(rk.len(), 64, "retrieval key is 32-byte lowercase hex");
    assert!(rk.chars().all(|c| c.is_ascii_hexdigit()));
}

#[wasm_bindgen_test]
fn key_derivation_is_deterministic_in_browser() {
    // The KDF must be a pure function of (store_id, resource_key, salt) so a
    // browser reader derives the SAME key the compiler sealed under.
    let k1 = derive_key(STORE_ID, "index.html", None).expect("derive");
    let k2 = derive_key(STORE_ID, "index.html", None).expect("derive");
    assert_eq!(k1, k2, "same inputs derive the same key");
    let k3 = derive_key(STORE_ID, "other.html", None).expect("derive");
    assert_ne!(k1, k3, "different resource keys derive different keys");
}

#[wasm_bindgen_test]
fn verify_inclusion_rejects_garbage_proof_in_browser() {
    let root = "00".repeat(32);
    let ok = verify_inclusion(b"not-a-real-ciphertext", "", &root).unwrap_or(false);
    assert!(!ok, "empty proof must not verify against an arbitrary root");
}
