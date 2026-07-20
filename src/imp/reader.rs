//! The lightweight, wasmtime-free capsule reader.
//!
//! Recovers the canonical [`Capsule`] `(store_id, root_hash)` from compiled
//! `.dig` module bytes using ONLY `wasmparser` + the no_std core — no wasmtime,
//! no chia-bls, no store. This is the slim path a URN resolver / node RPC /
//! descriptor check uses to learn what a module claims to be without standing up
//! the full serve engine.
//!
//! # Fail-closed integrity
//!
//! The reader recomputes the merkle root from the embedded `MerkleNodes` leaves
//! and REJECTS ([`ModuleReadError::RootMismatch`]) unless it equals the embedded
//! `CurrentRoot`. A forged `CurrentRoot` therefore cannot survive a read: the
//! returned `root_hash` is always internally consistent with the module's
//! committed leaves.
//!
//! # What this does NOT prove (caller's responsibility)
//!
//! `store_id` is the store's on-chain Chia launcher id. It is baked into the
//! module at compile time and is NOT self-verifiable from the module bytes alone
//! (nothing in the bytes binds them to that launcher). A caller that trusts the
//! returned `store_id` MUST cross-check it against the trusted anchor it already
//! holds — the URN it resolved, the on-chain singleton, or a `ChainState` it
//! independently verified. Likewise, this proves the module is a self-consistent
//! build, NOT that `root_hash` is the publisher's latest authorized root (the
//! chain is the authority for that).

use crate::imp::core::bytes::Bytes32;
use crate::imp::core::capsule::Capsule;
use crate::imp::core::datasection::{decode_merkle_leaves, DataView, SectionId, DIGS_DATA_OFFSET};
use crate::imp::core::merkle::MerkleTree;
use crate::imp::extract::{extract_digs_segment, ExtractError};

/// Why reading a [`Capsule`] from module bytes failed.
///
/// Deterministic and catalogued (CLAUDE.md §6.2): each variant names exactly one
/// failure class, so a caller can branch on it without parsing a message string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleReadError {
    /// The input is not a parseable wasm module.
    BadWasm,
    /// The module carries no DIGS data segment at the canonical offset.
    NoDataSection,
    /// The DIGS blob header/offset-table is malformed (bad magic, unknown
    /// version, or an out-of-bounds section row).
    BadBlob,
    /// A required section is absent from the blob.
    MissingSection(SectionId),
    /// A fixed-width section (`StoreId` / `CurrentRoot`) is not 32 bytes.
    BadSectionLen,
    /// The merkle root recomputed from `MerkleNodes` does not equal the embedded
    /// `CurrentRoot` — the module's committed content is inconsistent or the
    /// `CurrentRoot` was tampered with. FAIL-CLOSED: the read is rejected.
    RootMismatch,
}

impl core::fmt::Display for ModuleReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ModuleReadError::BadWasm => f.write_str("input is not a parseable wasm module"),
            ModuleReadError::NoDataSection => {
                f.write_str("no DIGS data segment at the canonical offset")
            }
            ModuleReadError::BadBlob => f.write_str("malformed DIGS data-section blob"),
            ModuleReadError::MissingSection(id) => {
                write!(f, "missing required data section: {id:?}")
            }
            ModuleReadError::BadSectionLen => {
                f.write_str("fixed-width section has an unexpected length")
            }
            ModuleReadError::RootMismatch => {
                f.write_str("recomputed merkle root does not match embedded CurrentRoot")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ModuleReadError {}

impl Capsule {
    /// Recover the canonical `Capsule { store_id, root_hash }` from compiled
    /// `.dig` module bytes — the lightweight, wasmtime-free reader.
    ///
    /// Reads the embedded `StoreId` and `CurrentRoot` sections and, FAIL-CLOSED,
    /// recomputes the merkle root from the embedded `MerkleNodes` leaves and
    /// rejects [`ModuleReadError::RootMismatch`] unless it equals `CurrentRoot`
    /// (so a forged `CurrentRoot` cannot pass). The returned `root_hash` is thus
    /// internally consistent with the module's committed content.
    ///
    /// SECURITY: `store_id` is the on-chain launcher id and is NOT self-verifiable
    /// from the module bytes — the caller MUST cross-check it against a trusted
    /// anchor (URN / on-chain singleton / verified `ChainState`). See the module
    /// docs.
    ///
    /// # Errors
    /// See [`ModuleReadError`]. Never panics on adversarial input.
    ///
    /// # Example
    /// ```no_run
    /// use dig_capsule::capsule::Capsule;
    /// # fn read(module_bytes: &[u8]) {
    /// let capsule = Capsule::from_module_bytes(module_bytes).expect("valid module");
    /// // Now cross-check capsule.store_id against your trusted anchor before trusting it.
    /// # let _ = capsule;
    /// # }
    /// ```
    pub fn from_module_bytes(module: &[u8]) -> Result<Capsule, ModuleReadError> {
        let raw = extract_digs_segment(module, DIGS_DATA_OFFSET).map_err(|e| match e {
            ExtractError::BadWasm => ModuleReadError::BadWasm,
            ExtractError::NoDataSection => ModuleReadError::NoDataSection,
        })?;
        let view = DataView::parse(&raw).map_err(|_| ModuleReadError::BadBlob)?;

        let store_id = read_bytes32(&view, SectionId::StoreId)?;
        let current_root = read_bytes32(&view, SectionId::CurrentRoot)?;

        let merkle_body = view
            .section(SectionId::MerkleNodes)
            .ok_or(ModuleReadError::MissingSection(SectionId::MerkleNodes))?;
        let leaves = decode_merkle_leaves(merkle_body).map_err(|_| ModuleReadError::BadBlob)?;
        let recomputed = MerkleTree::from_leaves(leaves).root();
        if recomputed != current_root {
            return Err(ModuleReadError::RootMismatch);
        }

        Ok(Capsule {
            store_id,
            root_hash: current_root,
        })
    }
}

/// Read a required 32-byte section body as a [`Bytes32`], mapping absence and a
/// wrong length to their catalogued errors.
fn read_bytes32(view: &DataView<'_>, id: SectionId) -> Result<Bytes32, ModuleReadError> {
    let body = view
        .section(id)
        .ok_or(ModuleReadError::MissingSection(id))?;
    let arr: [u8; 32] = body
        .try_into()
        .map_err(|_| ModuleReadError::BadSectionLen)?;
    Ok(Bytes32(arr))
}
