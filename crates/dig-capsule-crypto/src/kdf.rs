//! HKDF content-key derivation (paper §11.1/§11.4).
//!
//! The implementation now lives in [`dig_capsule_core::crypto`] — the single source
//! of truth shared with the producer and the browser verifier. This module
//! re-exports it so host call-sites (`digstore-cli`, `dig-capsule-store`) are
//! unchanged.

pub use dig_capsule_core::crypto::derive_decryption_key;
