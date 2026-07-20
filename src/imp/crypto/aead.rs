//! AES-256-GCM-SIV chunk seal (paper §11.2).
//!
//! The implementation now lives in [`crate::imp::core::crypto`] — the single source
//! of truth shared with the producer and the browser verifier. This module
//! re-exports `encrypt_chunk` unchanged and wraps `decrypt_chunk` to keep the
//! host-facing typed [`TamperError`] (so existing host call-sites are unchanged).
//! See `crate::imp::core::crypto::FIXED_NONCE` for the fixed-nonce / determinism
//! rationale.

use crate::imp::crypto::error::TamperError;

pub use crate::imp::core::crypto::encrypt_chunk;

/// Decrypt and authenticate a chunk. A failed tag check is a [`TamperError`].
pub fn decrypt_chunk(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, TamperError> {
    crate::imp::core::crypto::decrypt_chunk(key, ciphertext).map_err(|()| TamperError)
}
