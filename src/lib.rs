//! # dig-capsule — the canonical DIG capsule standard
//!
//! WU1 stub. The full crate (size constants, `Capsule` identity, `SectionId`
//! registry + DIGS-envelope validator, the `CapsuleClass` seam, the wasm
//! dual-target) is built on this branch — see `SPEC.md`.

#![cfg_attr(not(feature = "std"), no_std)]

/// The crate version (matches `Cargo.toml`), for compatibility checks.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
