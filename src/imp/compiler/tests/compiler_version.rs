//! §5: "Compiler version 1.0.0; module format version 1."
//!
//! The module-format-version half is already carried by the DIGS blob header
//! byte (== 1). This test pins the COMPILER-VERSION half the spec mandates: the
//! compiler must carry the exact version string "1.0.0" and a successful
//! compilation must record that version in its outcome.
//!
//! NOTE (#1270): `COMPILER_VERSION` is DELIBERATELY DECOUPLED from the crate
//! version. The members were collapsed into the single `dig-capsule` crate
//! (v0.3.0); `COMPILER_VERSION` stays the frozen literal `"1.0.0"` because it is
//! byte-recorded into every compile outcome (SPEC §5). The old assertions that
//! tied it to `env!("CARGO_PKG_VERSION")` only held while the crate version WAS
//! 1.0.0 and are removed.


use super::common::{sample_generations, sample_manifest, store_id, store_pubkey, trusted_keys};
use crate::imp::compiler::{Compiler, CompilerConfig, COMPILER_VERSION};

#[test]
fn compiler_version_constant_is_exactly_the_spec_value() {
    // §5: "Compiler version 1.0.0".
    assert_eq!(COMPILER_VERSION, "1.0.0");
}

#[test]
fn compile_outcome_records_compiler_version() {
    let dir = std::env::temp_dir().join(format!("digc-ver-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let gens = sample_generations();
    let cfg = CompilerConfig {
        output_dir: dir.clone(),
        obfuscate: false,
        optimize: false,
        template_override: None,
        // Small uniform budget keeps the emitted module tiny/fast.
        uniform_blob_len: 64 * 1024,
    };
    let outcome = Compiler::compile(
        &cfg,
        store_id(),
        store_pubkey(),
        &gens,
        sample_manifest(),
        super::common::no_auth(),
        &trusted_keys(),
        None,
        None,
    )
    .expect("compiles");

    // §5: the emitted artifact carries the compiler version it was built by.
    assert_eq!(outcome.detail.compiler_version, "1.0.0");
    assert_eq!(outcome.detail.compiler_version, COMPILER_VERSION);

    std::fs::remove_dir_all(&dir).ok();
}
