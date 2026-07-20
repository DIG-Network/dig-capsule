//! Asserts the compiled guest wasm module validates and exports the full ABI.
//! Uses wasmparser to validate and to enumerate exports/imports.
//!
//! The bytes under test are the REAL guest wasm embedded at build time
//! (`crate::imp::stage::embedded_guest_wasm()`, produced by
//! `cargo build --no-default-features --features guest-wasm
//!  --target wasm32-unknown-unknown --release`) — hermetic, no runtime cargo build.

fn build_wasm() -> Vec<u8> {
    crate::imp::stage::embedded_guest_wasm().to_vec()
}

#[test]
fn module_validates_and_exports_full_abi() {
    let bytes = build_wasm();
    // Validate the module.
    wasmparser::validate(&bytes).expect("module must be valid wasm");

    // Collect exported function/memory names.
    let mut exports = std::collections::BTreeSet::new();
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        if let wasmparser::Payload::ExportSection(reader) = payload.unwrap() {
            for e in reader {
                exports.insert(e.unwrap().name.to_string());
            }
        }
    }
    for required in [
        "get_store_id",
        "get_current_roothash",
        "get_roothash_history",
        "get_public_key",
        "get_metadata",
        "get_authentication_info",
        "get_content",
        "get_proof",
        "alloc",
        "dealloc",
        "init",
        "memory",
    ] {
        assert!(
            exports.contains(required),
            "missing ABI export: {required} (have: {exports:?})"
        );
    }

    // §5.1 Import section / §6.3 Host Imports: the guest module MUST declare all
    // eight dig_host host functions. LLVM only emits an import that is reachable
    // from an export, so `init` anchors them (see `imports::retain_dig_host_imports`);
    // this guards that retention against silent regression.
    let mut imports = std::collections::BTreeSet::new();
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        if let wasmparser::Payload::ImportSection(reader) = payload.unwrap() {
            for i in reader {
                let i = i.unwrap();
                if i.module == "dig_host" {
                    imports.insert(i.name.to_string());
                }
            }
        }
    }
    for required in [
        "host_get_public_key",
        "host_create_attestation",
        "host_establish_session",
        "host_verify_session",
        "jwks_fetch",
        "host_get_current_time",
        "host_random_bytes",
        "host_read_return_buffer",
    ] {
        assert!(
            imports.contains(required),
            "missing §5.1 dig_host import: {required} (have: {imports:?})"
        );
    }
}
