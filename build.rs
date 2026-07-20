//! The collapsed crate's single build script (#1270), with THREE jobs — each gated
//! by the feature that needs it, so the base / `guest-wasm` builds do no work:
//!
//! (a) `compile`: assemble the pinned guest WAT template
//!     (`fixtures/digstore_guest_template.wat`) → `OUT_DIR/digstore_guest_template.wasm`
//!     (the compiler's PINNED §19.3 template input; byte-identical across environments).
//! (b) `compile`: embed the REAL self-serving guest wasm so the stage→compile engine
//!     produces a genuinely self-serving module (BINDING contract D6).
//! (c) `risc0`: embed the RISC0 zkVM serving-guest ELF (`guest-risc0/`) via
//!     `risc0_build::embed_methods()`.

fn main() {
    #[cfg(feature = "compile")]
    assemble_template_wat();
    #[cfg(feature = "compile")]
    embed_guest_wasm();
    #[cfg(feature = "risc0")]
    embed_risc0_methods();
}

/// (c) Embed the RISC0 zkVM serving-guest ELF (`guest-risc0/`).
///
/// The guest manifest ships as `guest-risc0/Cargo.toml.template` (NOT `Cargo.toml`)
/// so `cargo publish` includes the guest sources: cargo auto-excludes any subdirectory
/// that holds its own `Cargo.toml` (a nested package), even from an explicit `include`,
/// which would otherwise drop the guest from the published tarball. We materialize the
/// real `Cargo.toml` here (idempotent) just before `embed_methods()`, which reads
/// `[package.metadata.risc0] methods = ["guest-risc0"]` and builds the guest.
#[cfg(feature = "risc0")]
fn embed_risc0_methods() {
    use std::path::PathBuf;

    let guest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("guest-risc0");
    let template = guest_dir.join("Cargo.toml.template");
    let manifest = guest_dir.join("Cargo.toml");
    let bytes = std::fs::read(&template).expect("read guest-risc0/Cargo.toml.template");
    std::fs::write(&manifest, &bytes).expect("materialize guest-risc0/Cargo.toml");
    println!("cargo:rerun-if-changed=guest-risc0/Cargo.toml.template");
    println!("cargo:rerun-if-changed=guest-risc0/src/main.rs");

    risc0_build::embed_methods();
}

/// (a) Assemble `fixtures/digstore_guest_template.wat` → `OUT_DIR/digstore_guest_template.wasm`.
#[cfg(feature = "compile")]
fn assemble_template_wat() {
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("fixtures/digstore_guest_template.wat");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("digstore_guest_template.wasm");

    let wat = std::fs::read_to_string(&src).expect("read template wat");
    let wasm = wat::parse_str(&wat).expect("assemble template wat");
    std::fs::write(&dest, wasm).expect("write template wasm");

    println!("cargo:rerun-if-changed=fixtures/digstore_guest_template.wat");
}

/// (b) Embed the real guest wasm into `OUT_DIR/dig_capsule_guest.wasm` (stable include
/// name for `imp::stage`).
///
/// PRESERVES the `DIGSTORE_GUEST_WASM` contract byte-for-byte: an absolute path to a
/// guest wasm, for out-of-workspace (git/registry-dependency) builds, wins over the
/// in-crate default. The in-crate default is the sibling build artifact produced by
///   cargo build --no-default-features --features guest-wasm \
///     --target wasm32-unknown-unknown --release
/// which lives at `<manifest_dir>/target/wasm32-unknown-unknown/release/dig_capsule.wasm`
/// (the crate's cdylib is `dig_capsule`, so the artifact filename is `dig_capsule.wasm`).
#[cfg(feature = "compile")]
fn embed_guest_wasm() {
    use std::path::PathBuf;

    // Consumer override: an absolute path to the guest wasm, for out-of-crate
    // (git/registry-dependency) builds. Wins over the in-crate default when set.
    let override_path = std::env::var_os("DIGSTORE_GUEST_WASM").map(PathBuf::from);
    println!("cargo:rerun-if-env-changed=DIGSTORE_GUEST_WASM");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Single crate at the repo root: the guest wasm is the sibling `target/` artifact.
    let default_guest = manifest_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("dig_capsule.wasm");

    let guest = override_path.unwrap_or(default_guest);

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("dig_capsule_guest.wasm");

    match std::fs::read(&guest) {
        Ok(bytes) => {
            std::fs::write(&dest, &bytes).expect("write embedded guest wasm");
        }
        Err(e) => {
            panic!(
                "dig-capsule requires the real guest wasm at {} (BINDING contract D6: \
                 the compiled module must serve itself). Build it first:\n  \
                 cargo build --no-default-features --features guest-wasm \
                 --target wasm32-unknown-unknown --release\n\
                 (or, for an out-of-crate/registry-dependency build, set DIGSTORE_GUEST_WASM \
                 to an absolute path to a matching dig_capsule.wasm)\n\
                 underlying error: {e}",
                guest.display()
            );
        }
    }

    println!("cargo:rerun-if-changed={}", guest.display());
}
