//! Internal implementation modules for the collapsed `dig-capsule` crate (#1270).
//!
//! Each module here is the former `dig-capsule-*` member crate, inlined verbatim
//! (a mechanical source move — zero logic/byte change). They are `pub(crate)`
//! PLUMBING: consumers depend ONLY on the curated public facade in
//! [`crate`] (`capsule`, `urn`, `format`, `merkle`, `chunk`, `metadata`,
//! `crypto`, `store`, `compile`, `stage`, `host`, `prover`, `guest`) — never on
//! `imp::*` directly. The feature gates below mirror the facade's exactly.
//!
//! These lints are allowed for the WHOLE `imp` tree because it is inlined member
//! source preserved verbatim: each former crate keeps its full public re-export
//! surface (some items the curated facade doesn't surface are now internally unused
//! — `unused_imports`/`dead_code`), and a former top crate `foo` with an inner
//! `foo.rs` now nests as `imp::foo::foo` (`clippy::module_inception`). None is a
//! real defect; pruning would shrink the preserved member surface (#1270).
#![allow(unused_imports, dead_code, clippy::module_inception)]

// The always-on no_std+alloc base (former dig-capsule-core / dig-capsule-chunker).
pub(crate) mod chunker;
pub(crate) mod core;

// The wasmparser-only DIGS-blob extraction (shared by `reader` + `compile`) and
// the lightweight, wasmtime-free capsule reader. `compile` implies `reader`, so
// the extraction path is shared, never duplicated.
#[cfg(feature = "reader")]
pub(crate) mod extract;
#[cfg(feature = "reader")]
pub(crate) mod reader;

// Native capsule crypto (AEAD + Chia-BLS). Base facade module `crypto` gates on this.
#[cfg(feature = "crypto")]
pub(crate) mod crypto;

// The local store / generation / staging model.
#[cfg(feature = "store")]
pub(crate) mod store;

// The files -> self-serving `.dig` WASM compiler + the stage->compile pipeline.
#[cfg(feature = "compile")]
pub(crate) mod compiler;
#[cfg(feature = "compile")]
pub(crate) mod stage;

// The blind serve triad: wasmtime host runtime + serving proofs.
#[cfg(feature = "serve")]
pub(crate) mod host;
#[cfg(feature = "serve")]
pub(crate) mod prover;

// The in-module served guest logic. Compiled under `serve` (native rlib) AND
// under `guest-wasm` (the wasm32 cdylib that EXPORTS the guest ABI).
#[cfg(any(feature = "serve", feature = "guest-wasm"))]
pub(crate) mod guest;
