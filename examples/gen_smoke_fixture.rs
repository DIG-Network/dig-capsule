//! Emit a JSON fixture (to stdout) that mirrors EXACTLY what the dighub content
//! path serves for a single-resource generation, so a JS smoke test can drive the
//! `wasm` `verifyInclusion` + `decryptResource` against real host-produced bytes.
//!
//! Run: `cargo run --example gen_smoke_fixture` (needs the default `crypto` feature).

use dig_capsule::crypto::{derive_decryption_key, encrypt_chunk, sha256};
use dig_capsule::format::codec::Encode;
use dig_capsule::format::Bytes32;
use dig_capsule::merkle::{MerkleProof, MerkleTree, ProofStep};
use dig_capsule::urn::{DigUrn, UrnBytes32};

fn rootless_urn(store_id: Bytes32, resource_key: &str) -> DigUrn {
    DigUrn {
        chain: "chia".to_string(),
        store_id: UrnBytes32(store_id.0),
        root_hash: None,
        resource_key: Some(resource_key.to_string()),
    }
}

fn canonical_urn(store_id: Bytes32, resource_key: &str) -> String {
    rootless_urn(store_id, resource_key).canonical()
}

fn b64(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}

fn main() {
    let store = Bytes32([0x2au8; 32]);
    let resource = "index.html";
    let plaintext =
        b"<!doctype html><title>dig smoke</title><p>verified + decrypted in the browser</p>"
            .to_vec();

    let canonical = canonical_urn(store, resource);
    let key = derive_decryption_key(&canonical, None);
    let ct = encrypt_chunk(&key, &plaintext);
    let leaf = sha256(&ct);

    // Real two-leaf generation -> a proof with a genuine sibling step.
    let sibling = Bytes32([0xeeu8; 32]);
    let tree = MerkleTree::from_leaves(vec![leaf, sibling]);
    let root = tree.root();
    let proof = MerkleProof {
        leaf,
        path: vec![ProofStep {
            hash: sibling,
            is_left: false,
        }],
        root,
    };

    // A DIFFERENT (wrong) root to assert the decoy/wrong-store rejection path.
    let wrong_root = Bytes32([0x00u8; 32]);

    println!(
        "{{\
\"store_id\":\"{}\",\
\"resource_key\":\"{}\",\
\"ciphertext_b64\":\"{}\",\
\"proof_b64\":\"{}\",\
\"root\":\"{}\",\
\"wrong_root\":\"{}\",\
\"expected_plaintext\":\"{}\",\
\"expected_retrieval_key\":\"{}\",\
\"expected_urn\":\"{}\"\
}}",
        store.to_hex(),
        resource,
        b64(&ct),
        b64(&proof.to_bytes()),
        root.to_hex(),
        wrong_root.to_hex(),
        String::from_utf8(plaintext.clone())
            .unwrap()
            .replace('"', "\\\""),
        rootless_urn(store, resource).content_key().to_hex(),
        canonical,
    );
}
