//! # dig-capsule — the `.dig` capsule data plane, one curated API
//!
//! A **capsule** is one immutable store generation — the pair `(store_id, root_hash)`
//! — packaged as the `.dig` / DIGS artifact. This crate is the SINGLE front door to
//! everything that creates, manages, reads, or serves that artifact. Depend on just
//! `dig-capsule`; the internal implementation modules under `imp` are `pub(crate)`
//! plumbing you never name directly.
//!
//! This is a PURE facade over the inlined implementation: it re-exports and organizes
//! the modules by concept. It adds no `.dig` format logic and changes no format bytes
//! — the golden fixtures read byte-identically. The on-chain anchor (the CHIP-0035
//! singleton, storeId minting, generations/anti-rollback) and the CLI live in
//! [`dig-store`](https://github.com/DIG-Network/digs), which depends on this crate.
//!
//! ## Where to look — the concept modules
//!
//! Everything below reads without opening any implementation module.
//!
//! | Module | Covers | Feature |
//! |--------|--------|---------|
//! | [`capsule`] | Capsule identity, the size ladder, visibility, generations | base |
//! | [`urn`] | The canonical `urn:dig:chia:…` scheme (`dig-urn-protocol`) + key derivation | `std` |
//! | [`format`] | The DIGS data section, codec, wire types, ABI, hashing | base |
//! | [`merkle`] | The ciphertext-leaf merkle tree + inclusion proofs | base |
//! | [`chunk`] | Content-defined (FastCDC) chunking | base |
//! | [`metadata`] | The metadata + public manifest | base |
//! | [`crypto`] | Native AEAD + Chia-BLS signing/verification | `crypto` |
//! | [`store`] | The local chunk-store / generation / staging model | `store` |
//! | [`compile`] | Files → self-serving `.dig` WASM module | `compile` |
//! | [`stage`] | The stage → compile build pipeline (the files→capsule entry) | `compile` |
//! | [`host`] | The wasmtime runtime that serves a module BLIND | `serve` |
//! | [`prover`] | Serving/execution proofs (§13) + Chia chain anchoring | `serve` |
//! | [`guest`] | The in-module served logic (low-level escape hatch) | `serve` |
//!
//! For the common path, `use dig_capsule::prelude::*;` pulls the most-used items.
//!
//! ## Feature flags — which consumer uses which
//!
//! The base surface (read / format / merkle / chunk / metadata) is always on and pulls
//! no wasmtime and no `blst`, and is `no_std + alloc`-clean so a slim consumer stays
//! light. The canonical URN scheme ([`urn`]) sits just above it under `std` (its
//! `dig-urn-protocol` owner is a `std` crate).
//!
//! - **`default = ["full"]`** — the whole API (`crypto + store + compile + serve`). What
//!   `dig-store` and `dig-node` want.
//! - **`std`** — enables `std` (and the canonical [`urn`] module + `chunk::chunk_stream`).
//! - **`crypto`** — native AES-256-GCM-SIV AEAD + Chia-BLS (implies `std`).
//! - **`store`** — the on-disk generation / staging model (implies `std`).
//! - **`compile`** — the files→capsule pipeline (implies `store` + `crypto`).
//! - **`serve`** — the blind serve triad + serving proofs (implies `crypto`).
//! - **`risc0`** — the real RISC0 serving-proof circuit; OFF by default (needs the
//!   RISC0 toolchain), implies `serve`.
//! - **`guest-wasm`** — the wasm32, no_std self-serving guest cdylib (exports the guest
//!   ABI). Used by the build to produce the embedded guest wasm; not for consumers.
//!
//! A slim reader uses `dig-capsule = { version = "0.3", default-features = false }`
//! → the no_std base only.
//!
//! ## The browser counterpart
//!
//! The browser + Node read-crypto is NOT a Rust dependency: it is the
//! **`@dignetwork/dig-capsule-wasm`** npm package, whose surface
//! (`reconstructUrn`, `retrievalKey`, `deriveKey`, `verifyInclusion`,
//! `decryptResource`, `decryptResourceToText`, `readPublicManifest`, `version`) is
//! installed on `globalThis.digClient`. It produces byte-identical KDF/AEAD/URN/merkle
//! output to the native [`crypto`] path here — the two are the same contract on two
//! runtimes.
//!
//! ## A light read (no I/O)
//!
//! ```
//! use dig_capsule::prelude::*;
//!
//! // The default capsule size class is 128 MB (the single canonical size, #130).
//! let spec = CapsuleClass::DEFAULT.spec();
//! assert_eq!(spec.content_cap_bytes, 128_000_000);
//!
//! // A resource URN derives a stable, root-independent content key.
//! let urn = DigUrn::parse("urn:dig:chia:0000000000000000000000000000000000000000000000000000000000000000").unwrap();
//! let _key = urn.content_key();
//! ```
//!
//! ## The full build path (default features)
//!
//! ```no_run
//! // The files→capsule pipeline and the blind serve entry both resolve through the
//! // facade under the default (full) features — no implementation module is ever named.
//! use dig_capsule::stage::stage_and_compile;
//! use dig_capsule::host::serve_blind;
//! use dig_capsule::compile::Compiler;
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// The inlined implementation (former `dig-capsule-*` member crates). PLUMBING —
// consumers use the curated facade modules below, never `imp::*` directly.
pub(crate) mod imp;

// ---------------------------------------------------------------------------
// Base concept modules (always compiled).
// ---------------------------------------------------------------------------

/// Capsule identity + the size ladder.
///
/// A capsule is the pair `(store_id, root_hash)` ([`Capsule`]); its canonical string
/// is `storeId:rootHash`. Each capsule is padded to a uniform blob sized by a
/// [`CapsuleClass`] so its size reveals nothing about the plaintext — the
/// [`CapsuleClass::DEFAULT`] is 128 MB, the single canonical size.
pub mod capsule {
    pub use crate::imp::core::capsule::Capsule;
    pub use crate::imp::core::capsule_class::{CapsuleClass, CapsuleSpec};
    pub use crate::imp::core::config::{
        Generation, GenerationId, GenerationState, SecretSalt, StoreConfig, TrustedHostKey,
        Visibility, MAX_STORE_BYTES,
    };
}

/// The DIG content URN scheme and its frozen retrieval-key derivation.
///
/// `urn:dig:chia:<store_id>[:<root>][/<resource_key>]` ([`DigUrn`]). The canonical
/// scheme, grammar, and key derivation are owned by the `dig-urn-protocol` crate —
/// the ONE ecosystem definition — and re-exported here so consumers reach them
/// through the facade. Two keys are derived from a URN, both FROZEN and shared
/// byte-for-byte with the browser verifier:
///
/// - [`DigUrn::retrieval_key`] = `SHA-256(canonical())` — the URN-identity key that
///   PINS the root (what the frozen conformance corpus fixes);
/// - [`DigUrn::content_key`] = `SHA-256(canonical_rootless())` — the root-INDEPENDENT
///   key a resolver uses to fetch and to seed the AES key (stable across generations).
///
/// Gated on `std`: `dig-urn-protocol` is a `std` crate (its errors implement
/// `std::error::Error`), so this module is unavailable in the no_std base — every
/// non-base feature (`crypto`/`store`/`compile`/`serve`) implies `std`.
///
/// [`Bytes32`]: crate::format::Bytes32
#[cfg(feature = "std")]
pub mod urn {
    pub use dig_urn_protocol::{
        Bytes32 as UrnBytes32, DigUrn, SecretSalt, UrnParseError, CANONICAL_CHAIN,
        DEFAULT_RESOURCE_KEY, SALT_QUERY_MARKER, URN_ABNF, URN_PREFIX,
    };

    /// The [`Capsule`](crate::capsule::Capsule) a URN pins, if any — the equivalent of
    /// the former `Urn::as_capsule`.
    ///
    /// Returns `Some` only when the URN carries a concrete `root_hash` (that
    /// `(store_id, root_hash)` pair *is* a capsule / one immutable generation); a
    /// rootless URN pins no generation and yields `None`. Pure naming view — it does
    /// not touch `canonical()` / `retrieval_key`.
    pub fn capsule_from_urn(urn: &DigUrn) -> Option<crate::capsule::Capsule> {
        urn.root_hash.map(|root_hash| crate::capsule::Capsule {
            store_id: crate::format::Bytes32(urn.store_id.0),
            root_hash: crate::format::Bytes32(root_hash.0),
        })
    }
}

/// The DIGS on-disk/on-wire format: data section, codec, wire types, ABI, hashing.
///
/// The byte-exact layout is the normative contract (see the repo `SPEC.md`); every
/// change is additive (CLAUDE.md §5.1) so older `.dig` artifacts stay readable. The
/// submodules are re-exported whole so the entire section registry, codec, and wire
/// shapes are reachable here.
pub mod format {
    pub use crate::imp::core::bytes::{Bytes32, Bytes48, Bytes96};
    pub use crate::imp::core::error::{CoreError, ErrorCode};
    pub use crate::imp::core::hash::sha256;
    pub use crate::imp::core::keytable::{KeyTableEntry, PathWalk};
    pub use crate::imp::core::tombstone::{RevocationReason, Tombstone, TombstoneScope};
    pub use crate::imp::core::{abi, codec, datasection, serving, wire};
}

/// The content-commitment merkle tree over sealed chunk leaves + inclusion proofs.
///
/// A served [`MerkleProof`] verifies the served ciphertext to the capsule root; a
/// leaf is domain-separated by [`LEAF_TAG`]/[`NODE_TAG`].
pub mod merkle {
    pub use crate::imp::core::merkle::{
        resource_leaf, MerkleProof, MerkleTree, ProofStep, LEAF_TAG, NODE_TAG,
    };
}

/// Deterministic content-defined (FastCDC-line) chunking.
///
/// Chunk boundaries are byte-identical across platforms so content-addressed dedup is
/// stable. [`ChunkerConfig`] carries the commit defaults.
pub mod chunk {
    pub use crate::imp::chunker::{
        chunk_slice, default_config, hash_data, mask_for_target, Chunk, Chunker, GEAR_TABLE,
    };
    pub use crate::imp::core::ChunkerConfig;

    /// Stream chunking over any `std::io::Read`. `std`-only (the base is no_std).
    #[cfg(feature = "std")]
    pub use crate::imp::chunker::chunk_stream;
}

/// The store metadata manifest and the public file manifest.
pub mod metadata {
    pub use crate::imp::core::manifest::{Author, MetadataManifest};
    pub use crate::imp::core::public_manifest::{
        PublicManifest, PublicManifestEntry, PUBLIC_MANIFEST_SCHEMA_VERSION,
    };
}

// ---------------------------------------------------------------------------
// Feature-gated concept modules.
// ---------------------------------------------------------------------------

/// Native capsule crypto: the AES-256-GCM-SIV chunk seal, HKDF key derivation, and
/// Chia-BLS signing/verification (blst-backed).
///
/// This is the AUTHORITATIVE native crypto. The pure, `blst`-free primitives that the
/// wasm-clean read path uses live under [`crypto::primitives`] — use those only when
/// you specifically need the no-`blst` variants.
#[cfg(feature = "crypto")]
pub mod crypto {
    pub use crate::imp::crypto::*;

    /// The pure (no-`blst`, wasm-clean) chunk-seal + KDF primitives from the format
    /// core. Byte-identical to the browser read path.
    pub mod primitives {
        pub use crate::imp::core::crypto::{decrypt_chunk, derive_decryption_key, encrypt_chunk};
    }
}

/// The local store model: the chunk store, generation + history model, staging, and
/// diff — the on-disk model a capsule is committed from.
///
/// Note: [`store::Clock`] and its `FixedClock`/`SystemClock` are the STORE's clock
/// trait, distinct from [`host::Clock`].
#[cfg(feature = "store")]
pub mod store {
    pub use crate::imp::store::*;
}

/// The compiler: transform a generation's staged content into a single self-serving
/// `.dig` WASM module (deterministic, byte-identical).
///
/// Note: [`compile::CompilerError`] is the compiler's error enum, distinct from the
/// config-level `crate::capsule` compiler error.
#[cfg(feature = "compile")]
pub mod compile {
    pub use crate::imp::compiler::*;
}

/// The stage → compile build pipeline — the primary "files → capsule" entry point.
///
/// [`stage::stage_and_compile`] seals + chunks a file set, builds the ciphertext-leaf
/// merkle tree, persists the generation, and compiles a real self-serving module.
#[cfg(feature = "compile")]
pub mod stage {
    pub use crate::imp::stage::*;
}

/// The wasmtime host runtime that serves a compiled module BLIND (it never decrypts or
/// inspects the served payload).
///
/// Note: [`host::Clock`] and its `FixedClock`/`SystemClock` are the HOST's clock trait,
/// distinct from [`store::Clock`].
#[cfg(feature = "serve")]
pub mod host {
    pub use crate::imp::host::*;
}

/// Serving/execution proofs (§13) and Chia chain anchoring: the [`prover::Prover`] /
/// [`prover::Verifier`] pair, the [`prover::ChainSource`] abstraction with its live
/// [`prover::CoinsetChainSource`], and the mock backends used while the RISC0 circuit
/// matures. `program_hash = SHA-256(module_bytes)`.
#[cfg(feature = "serve")]
pub mod prover {
    pub use crate::imp::prover::*;
}

/// The in-module served logic (the guest half of the serve triad) — the low-level
/// escape hatch for `get_content` / `get_proof` internals. Kept OUT of the [`prelude`]
/// on purpose; most callers use [`host`] instead.
#[cfg(feature = "serve")]
pub mod guest {
    pub use crate::imp::guest::*;
}

// ---------------------------------------------------------------------------
// The curated, collision-free prelude.
// ---------------------------------------------------------------------------

/// The most-used items for `use dig_capsule::prelude::*;`.
///
/// Curated to be COLLISION-FREE: where a name exists in more than one module (e.g.
/// `Clock`, `CompilerError`, `Result`, `DecodeError`), the prelude picks none and you
/// reach for the module-scoped item instead.
pub mod prelude {
    pub use crate::capsule::{Capsule, CapsuleClass, Visibility};
    pub use crate::format::{sha256, Bytes32, Bytes48, Bytes96};
    pub use crate::merkle::{MerkleProof, MerkleTree};
    pub use crate::metadata::MetadataManifest;
    #[cfg(feature = "std")]
    pub use crate::urn::DigUrn;

    #[cfg(feature = "store")]
    pub use crate::store::Store;

    #[cfg(feature = "compile")]
    pub use crate::compile::Compiler;
    #[cfg(feature = "compile")]
    pub use crate::stage::stage_and_compile;

    #[cfg(feature = "serve")]
    pub use crate::host::serve_blind;
}
