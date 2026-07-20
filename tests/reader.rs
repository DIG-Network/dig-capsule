//! Integration tests for the lightweight, wasmtime-free capsule reader
//! (`Capsule::from_module_bytes`, the `reader` feature).
//!
//! These exercise the reader through the crate's PUBLIC facade only. They need
//! the `compile` feature to BUILD test modules (wrap a DIGS blob into a wasm
//! module via `inject_data_section`) — `compile` implies `reader`, so the same
//! run covers both. Run with `--features compile` (or the default `full`).

#![cfg(all(feature = "reader", feature = "compile"))]

use dig_capsule::capsule::Capsule;
use dig_capsule::compile::{inject_data_section, DATA_SECTION_MEM_OFFSET};
use dig_capsule::format::datasection::{encode_blob, encode_merkle_nodes, SectionId};
use dig_capsule::format::Bytes32;
use dig_capsule::merkle::MerkleTree;
use dig_capsule::reader::ModuleReadError;

/// A minimal wasm template with a single memory (the reader only needs the
/// injected DIGS data segment; the code section is irrelevant to it).
fn empty_template() -> Vec<u8> {
    wat::parse_str(r#"(module (memory (export "memory") 1))"#).expect("wat compiles")
}

/// Wrap a raw DIGS blob into a real wasm module at the canonical offset — the
/// exact placement `from_module_bytes` scans for.
fn module_with_blob(blob: &[u8]) -> Vec<u8> {
    inject_data_section(&empty_template(), blob, DATA_SECTION_MEM_OFFSET).expect("inject ok")
}

/// Build a self-consistent DIGS blob: `CurrentRoot` == merkle root of `leaves`.
fn consistent_blob(store_id: [u8; 32], leaves: &[Bytes32]) -> Vec<u8> {
    let root = MerkleTree::from_leaves(leaves.to_vec()).root();
    encode_blob(&[
        (SectionId::StoreId as u16, store_id.to_vec()),
        (SectionId::CurrentRoot as u16, root.0.to_vec()),
        (SectionId::MerkleNodes as u16, encode_merkle_nodes(leaves)),
    ])
}

#[test]
fn reads_canonical_capsule_from_a_real_compiled_module() {
    let store_id = [0xAB; 32];
    let leaves = vec![Bytes32([0x33; 32]), Bytes32([0x44; 32])];
    let expected_root = MerkleTree::from_leaves(leaves.clone()).root();

    let module = module_with_blob(&consistent_blob(store_id, &leaves));
    let capsule = Capsule::from_module_bytes(&module).expect("valid module reads");

    assert_eq!(capsule.store_id, Bytes32(store_id));
    assert_eq!(capsule.root_hash, expected_root);
}

#[test]
fn not_wasm_bytes_yield_bad_wasm_never_panic() {
    let garbage = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03];
    assert_eq!(
        Capsule::from_module_bytes(&garbage),
        Err(ModuleReadError::BadWasm)
    );
}

#[test]
fn truncated_wasm_yields_bad_wasm() {
    let module = module_with_blob(&consistent_blob([1; 32], &[Bytes32([2; 32])]));
    let truncated = &module[..module.len() / 2];
    assert_eq!(
        Capsule::from_module_bytes(truncated),
        Err(ModuleReadError::BadWasm)
    );
}

#[test]
fn module_without_digs_segment_yields_no_data_section() {
    let module = wat::parse_str(r#"(module (memory (export "memory") 1))"#).unwrap();
    assert_eq!(
        Capsule::from_module_bytes(&module),
        Err(ModuleReadError::NoDataSection)
    );
}

#[test]
fn unknown_blob_version_yields_bad_blob() {
    // A segment that starts with the DIGS magic (so it IS located) but declares
    // an unknown version — `DataView::parse` rejects it.
    let mut blob = consistent_blob([1; 32], &[Bytes32([2; 32])]);
    blob[4] = 0xFF; // version byte
    let module = module_with_blob(&blob);
    assert_eq!(
        Capsule::from_module_bytes(&module),
        Err(ModuleReadError::BadBlob)
    );
}

#[test]
fn missing_store_id_yields_missing_section() {
    let leaves = vec![Bytes32([0x33; 32])];
    let root = MerkleTree::from_leaves(leaves.clone()).root();
    let blob = encode_blob(&[
        (SectionId::CurrentRoot as u16, root.0.to_vec()),
        (SectionId::MerkleNodes as u16, encode_merkle_nodes(&leaves)),
    ]);
    let module = module_with_blob(&blob);
    assert_eq!(
        Capsule::from_module_bytes(&module),
        Err(ModuleReadError::MissingSection(SectionId::StoreId))
    );
}

#[test]
fn missing_current_root_yields_missing_section() {
    let leaves = vec![Bytes32([0x33; 32])];
    let blob = encode_blob(&[
        (SectionId::StoreId as u16, vec![1u8; 32]),
        (SectionId::MerkleNodes as u16, encode_merkle_nodes(&leaves)),
    ]);
    let module = module_with_blob(&blob);
    assert_eq!(
        Capsule::from_module_bytes(&module),
        Err(ModuleReadError::MissingSection(SectionId::CurrentRoot))
    );
}

#[test]
fn missing_merkle_nodes_yields_missing_section() {
    let blob = encode_blob(&[
        (SectionId::StoreId as u16, vec![1u8; 32]),
        (SectionId::CurrentRoot as u16, vec![2u8; 32]),
    ]);
    let module = module_with_blob(&blob);
    assert_eq!(
        Capsule::from_module_bytes(&module),
        Err(ModuleReadError::MissingSection(SectionId::MerkleNodes))
    );
}

#[test]
fn wrong_store_id_length_yields_bad_section_len() {
    let leaves = vec![Bytes32([0x33; 32])];
    let root = MerkleTree::from_leaves(leaves.clone()).root();
    let blob = encode_blob(&[
        (SectionId::StoreId as u16, vec![7u8; 31]), // 31, not 32
        (SectionId::CurrentRoot as u16, root.0.to_vec()),
        (SectionId::MerkleNodes as u16, encode_merkle_nodes(&leaves)),
    ]);
    let module = module_with_blob(&blob);
    assert_eq!(
        Capsule::from_module_bytes(&module),
        Err(ModuleReadError::BadSectionLen)
    );
}

#[test]
fn wrong_current_root_length_yields_bad_section_len() {
    let blob = encode_blob(&[
        (SectionId::StoreId as u16, vec![1u8; 32]),
        (SectionId::CurrentRoot as u16, vec![2u8; 33]), // 33, not 32
        (
            SectionId::MerkleNodes as u16,
            encode_merkle_nodes(&[Bytes32([3; 32])]),
        ),
    ]);
    let module = module_with_blob(&blob);
    assert_eq!(
        Capsule::from_module_bytes(&module),
        Err(ModuleReadError::BadSectionLen)
    );
}

/// FAIL-CLOSED: a tampered `CurrentRoot` (with `MerkleNodes` intact) is rejected
/// — a forged root cannot survive a read.
#[test]
fn tampered_current_root_yields_root_mismatch() {
    let leaves = vec![Bytes32([0x33; 32]), Bytes32([0x44; 32])];
    let mut root = MerkleTree::from_leaves(leaves.clone()).root();
    root.0[0] ^= 0x01; // flip one byte of the committed root
    let blob = encode_blob(&[
        (SectionId::StoreId as u16, vec![0xAB; 32]),
        (SectionId::CurrentRoot as u16, root.0.to_vec()),
        (SectionId::MerkleNodes as u16, encode_merkle_nodes(&leaves)),
    ]);
    let module = module_with_blob(&blob);
    assert_eq!(
        Capsule::from_module_bytes(&module),
        Err(ModuleReadError::RootMismatch)
    );
}

/// §5.1 stability lock: the CURRENT reader recovers the canonical capsule from
/// the frozen golden data-section fixture (a self-consistent blob whose
/// `CurrentRoot` == the merkle root of leaves `[0x33; 32], [0x44; 32]`, and
/// whose `StoreId` == `[0xAB; 32]`). An older released blob must never stop
/// reading.
#[test]
fn golden_fixture_round_trips_through_reader() {
    let hex = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/golden_data_section.hex"
    ))
    .trim();
    let blob = hex::decode(hex).expect("golden fixture is valid hex");
    let module = module_with_blob(&blob);

    let capsule = Capsule::from_module_bytes(&module).expect("golden module reads");

    let expected_root =
        MerkleTree::from_leaves(vec![Bytes32([0x33; 32]), Bytes32([0x44; 32])]).root();
    assert_eq!(capsule.store_id, Bytes32([0xAB; 32]));
    assert_eq!(capsule.root_hash, expected_root);
}

/// The error type is `Display` + `Debug` + a standard `Error` (catalogued, §6.2).
#[test]
fn error_is_displayable() {
    let e = ModuleReadError::RootMismatch;
    assert!(!format!("{e}").is_empty());
    let _: &dyn std::error::Error = &e;
}
