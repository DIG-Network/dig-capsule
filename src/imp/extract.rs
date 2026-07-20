//! wasmparser-only extraction of the embedded DIGS data-section blob from a
//! compiled `.dig` module.
//!
//! This is the ONE place that pulls the DIGS blob out of a wasm module's active
//! data segments — shared by the lightweight `reader` API
//! ([`crate::imp::reader`]) and the heavy `compile` path
//! ([`crate::imp::compiler::inject`], which re-wraps the error into its own
//! `CompilerError`). It needs ONLY `wasmparser`, so it lives above the compiler
//! and is available whenever `reader` is on (`compile` implies `reader`).

use alloc::vec::Vec;
use wasmparser::{DataKind, Operator, Parser, Payload};

/// Failure extracting the DIGS blob from module bytes. Coarse on purpose: the
/// two failure classes a caller can act on are "not a wasm module" and "no DIGS
/// segment present". Blob-content validation is the caller's job (it holds the
/// parsed [`crate::imp::core::datasection::DataView`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractError {
    /// The input is not a parseable wasm module.
    BadWasm,
    /// No active DIGS data segment sits at the expected memory offset.
    NoDataSection,
}

/// Read a `i32.const N` offset (the ONLY offset form Rust/LLVM emits for active
/// wasm32 data segments) from an offset const-expression. Any other shape (or a
/// malformed expression) yields `None`, so the caller simply skips that segment
/// rather than treating attacker-controlled bytes as fatal.
pub(crate) fn const_i32_offset(offset_expr: &wasmparser::ConstExpr) -> Option<i32> {
    let mut ops = offset_expr.get_operators_reader();
    match ops.read().ok()? {
        Operator::I32Const { value } => Some(value),
        _ => None,
    }
}

/// Extract the raw bytes of the active DIGS data segment placed at `mem_offset`.
///
/// The compiler injects the DIGS blob LAST at `DIGS_DATA_OFFSET`, so when more
/// than one segment matches, the last one wins (mirroring wasm instantiation
/// order). The returned bytes are the raw segment payload (possibly padded past
/// the blob's self-describing length) — the caller trims via `DataView`.
///
/// Safe on adversarial input: every slice access is length-guarded and every
/// wasmparser error maps to [`ExtractError::BadWasm`] (never a panic).
pub fn extract_digs_segment(module: &[u8], mem_offset: u32) -> Result<Vec<u8>, ExtractError> {
    let mut found: Option<Vec<u8>> = None;
    for payload in Parser::new(0).parse_all(module) {
        let payload = payload.map_err(|_| ExtractError::BadWasm)?;
        if let Payload::DataSection(reader) = payload {
            for seg in reader {
                let seg = seg.map_err(|_| ExtractError::BadWasm)?;
                let DataKind::Active { offset_expr, .. } = seg.kind else {
                    continue;
                };
                let Some(off) = const_i32_offset(&offset_expr) else {
                    continue;
                };
                if off as u32 == mem_offset && seg.data.len() >= 4 && &seg.data[..4] == b"DIGS" {
                    found = Some(seg.data.to_vec());
                }
            }
        }
    }
    found.ok_or(ExtractError::NoDataSection)
}
