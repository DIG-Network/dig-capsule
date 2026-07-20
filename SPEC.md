# dig-capsule — the `.dig` capsule DATA-PLANE specification

Normative contract for the DIG **capsule** data plane: the `.dig` / DIGS format,
the capsule read-crypto, the compiler that builds a capsule from files, staging,
the guest/host serve triad, and the capsule size ladder. An independent
reimplementation can be built against this document.

A **store** is a capsule bound to its on-chain singleton. This crate owns the
**capsule** half — the `.dig` artifact itself and everything that creates,
manages, or reads it. The on-chain anchor (the CHIP-0035 singleton, storeId
minting, generations/anti-rollback), the §21 remote, and the CLI live in
`dig-store` and depend on this crate. See the superproject `SYSTEM.md` →
"store = chain + capsule".

This data plane was lifted from `dig-store` (epic #744 Phase 1) and collapsed into
ONE crate (#1270): the former `dig-capsule-*` member crates are now feature-gated
modules under `src/imp/`, and only `dig-capsule` publishes to crates.io.

## 0. The public entry point (the `dig-capsule` crate)

**`dig-capsule`** is a SINGLE crate with a curated public facade (`src/lib.rs`) over
feature-gated modules. The internal split lives under `src/imp/` (`core`, `chunker`,
`crypto`, `store`, `compiler`, `stage`, `guest`, `host`, `prover`) and is an
implementation detail — consumers use only the top-level concept modules
(`capsule`/`urn`/`format`/`merkle`/`chunk`/`metadata` in the base; `crypto`/`store`/
`compile`/`stage`/`host`/`prover`/`guest` behind feature flags). Features:
`default = full` (`crypto`+`store`+`compile`+`serve`); `std` lifts the crate out of
`no_std` and enables the canonical `urn` scheme + `chunk::chunk_stream` (both require
`std`); `wasm` is the browser/Node read-crypto surface; `guest-wasm` compiles the
self-serving guest cdylib to wasm32; `risc0` builds the real serving-proof circuit;
`reader` is the lightweight, wasmtime-free capsule reader (§1.1) — it pulls ONLY
`wasmparser` above the `no_std` core (NO wasmtime/chia-bls/store), and `compile`
implies it so the DIGS-blob extraction path is shared, never duplicated.
The base (no default features) is a `no_std`+`alloc` core with NO `blst`/`getrandom`.
The collapse changes no bytes — the normative contract below and the golden fixtures
read identically. `COMPILER_VERSION` is the literal `1.0.0`, DECOUPLED from the crate
version.

## 1. Capsule identity

- A **capsule** is one immutable store generation: the pair `(store_id, root_hash)`,
  each a 32-byte value.
- Canonical string form is `storeId:rootHash` — lowercase hex, colon-separated
  (`dig_capsule::capsule::Capsule::{canonical, from_canonical}`). A store is a sequence
  of capsules identified by `store_id`; each capsule is one on-chain-anchored root.
- The content URN is `urn:dig:chia:<store_id>[/<resource_key>]` (root-independent)
  or the display form `urn:dig:chia:<store_id>:<root>/<resource_key>`. The canonical
  `urn:dig:` scheme, its byte-level grammar, and the key derivation are OWNED by the
  **`dig-urn-protocol`** crate (the single ecosystem source of truth); `dig-capsule`
  CONSUMES it and re-exports `DigUrn` through the facade `urn` module. Two keys derive
  from a URN, both FROZEN: `retrieval_key = SHA-256(canonical())` (the URN-identity key
  that PINS the root, fixed by the frozen conformance corpus) and
  `content_key = SHA-256(canonical_rootless())` (the root-INDEPENDENT per-resource
  lookup + AES-seed key). The `.dig` format serializes NEITHER — a URN is never a
  section field, so consuming the canonical crate changes no format byte.

### 1.1 Reading a capsule from module bytes (the `reader` feature)

- `dig_capsule::capsule::Capsule::from_module_bytes(&[u8]) -> Result<Capsule,
  dig_capsule::reader::ModuleReadError>` recovers the canonical
  `(store_id, root_hash)` directly from a compiled `.dig` wasm module, WITHOUT the
  wasmtime serve engine. It reads the embedded `StoreId` and `CurrentRoot` sections
  from the DIGS data segment (§2) and is FAIL-CLOSED: it recomputes the merkle root
  from the embedded `MerkleNodes` leaves and returns `RootMismatch` unless it equals
  `CurrentRoot`, so a forged `CurrentRoot` cannot pass.
- **`store_id` is NOT self-verifiable from the module bytes.** It is the store's
  on-chain Chia launcher id, baked in at compile time; nothing in the bytes binds
  them to that launcher. A caller that trusts the returned `store_id` MUST cross-check
  it against a trusted anchor it already holds — the URN it resolved, the on-chain
  singleton, or a `ChainState` it independently verified. The read proves the module
  is a self-consistent build, NOT that `root_hash` is the publisher's latest
  authorized root (the chain is the authority for that — §3, §4).
- `ModuleReadError` is a catalogued enum (`BadWasm`, `NoDataSection`, `BadBlob`,
  `MissingSection(SectionId)`, `BadSectionLen`, `RootMismatch`); the reader never
  panics on adversarial input. The `reader` feature pulls ONLY `wasmparser` above the
  `no_std` core; `compile` implies it (the DIGS-blob extraction is shared, §4).

## 2. The DIGS data section

- Magic `DIGS` (4 bytes), then a `u8` `format_version` = **1**, then an offset
  table of `(section_id: u16, offset, len)` entries (`dig_capsule::format::codec::section`,
  `dig_capsule::format::datasection`).
- Section ids form a registry (store id, current root, root history, store pubkey,
  trusted host keys, metadata manifest, authentication info, key table, merkle
  leaves, chunks, public manifest, …). See `dig_capsule::format::datasection::SectionId`.
- **Backwards compatibility (HARD, CLAUDE.md §5.1): additive only.** New
  section ids and new optional fields may be added. Existing section ids MUST NOT
  be removed, renumbered, or repurposed; an existing field's meaning/encoding MUST
  NOT change. A reader ignores unknown section ids. `format_version` stays 1; a
  bump means "new writers MAY emit vN", never "readers reject < vN".
- The byte-exact layout is pinned by the golden fixture
  `tests/fixtures/golden_data_section.hex` and its byte-identical-read test. Every
  format change MUST keep that test green.

## 3. Capsule read-crypto

- Per-resource content is chunked (`chunk` module), then each chunk is sealed
  with AES-256-GCM-SIV under a key derived by HKDF from the canonical resource URN
  (and, for private stores, a 32-byte secret salt) (`dig_capsule::crypto`). The host
  serves ciphertext and is BLIND to plaintext.
- Content commitment is a Merkle tree over the sealed chunk leaves; a served
  `ContentResponse` carries a merkle inclusion proof that verifies to the capsule
  root, plus per-chunk ciphertext lengths so a streaming client can split and
  GCM-SIV-open each chunk (`dig_capsule::merkle`, `dig_capsule::format::serving`).
- The verifier leaf-binding contract: a served leaf MUST equal
  `sha256(served ciphertext)`. The browser read path (the `wasm` feature) recomputes
  the leaf from received bytes and rejects a mismatch.
- The signing/serving-proof crypto (chia-bls) lives behind the `crypto` feature; it is
  native-only and MUST NOT be pulled into the wasm read path (§6).

## 4. The compiler and staging

- The **compiler** (`compile` feature) builds a self-serving wasm capsule module
  from a generation's staged content: it embeds the DIGS data section into a guest
  template and pads to a uniform blob so the module size reveals nothing about the
  plaintext.
- **Staging / local build model** (`store` feature): chunk store, generation and
  history model, staging, and diff — the local model a capsule is committed from.
- The **build pipeline** (`stage`, under the `compile` feature) drives stage →
  compile, embedding the REAL guest wasm (BINDING contract D6) so the compiled module
  serves itself. The guest wasm is the same crate compiled under `guest-wasm`:
  `cargo build --no-default-features --features guest-wasm --target wasm32-unknown-unknown --release`
  (emits `target/wasm32-unknown-unknown/release/dig_capsule.wasm`, which `build.rs`
  embeds). An out-of-crate (crates.io/registry-dependency) build supplies it via the
  `DIGSTORE_GUEST_WASM` environment variable (absolute path).
- The **guest/host triad**: the guest (the in-module logic `get_content`/`get_proof`),
  the host (`host` feature — the wasmtime runtime that serves a compiled module blind),
  and the prover (`serve` feature — §13 serving/execution proofs and chain anchoring).
  The real RISC0 serving-proof circuit is the `guest-risc0/` NESTED package, built
  only under the `risc0` feature (referenced via `[package.metadata.risc0]`, NOT a
  `[dependencies]` path entry, so it is never published as a separate crate).

## 5. The capsule size ladder

- Capsules are padded to a uniform blob sized by a **size class**
  (`dig_capsule::capsule::CapsuleClass` / `CapsuleSpec`).
- The ladder is powers of 2 MB from 2 MB up to the first rung ≥ 1 GB:
  `{2, 4, 8, 16, 32, 64, 128, 256, 512, 1024} × 10^6 bytes` (each rung is
  `2 MB · 2^k`, `k = 0..=9`; `1024 MB` is the top).
- **DEFAULT = `CapsuleClass::Mb128`** (128 MB): its content cap is exactly
  `dig_capsule::capsule::MAX_STORE_BYTES = 128_000_000`, THE single canonical
  capsule-size number (#130). Its uniform blob is 128 MiB
  (`dig_capsule::compile::FIXED_BLOB_LEN`), and the invariant
  `uniform_blob_len >= content_cap_bytes` holds for every rung.
- **Behaviour (this phase): only the DEFAULT is produced.** The compiler and the
  uniform-blob padding emit exactly the 128 MB default, byte-for-byte as before.
  The other rungs are declared (a stable, additively-expandable enum) but inert.
  Actual multi-size production — per-class padding, the per-capsule size as an
  additive property every older reader parses as the 128 MB class, and pricing —
  is a follow-up feature (a child of #744).

## 6. Layering and the wasm target

- **Acyclic (HARD).** The data plane depends only on itself; it has NO dependency
  on any `dig-store` chain-plane crate (`digstore-chain`, `digstore-remote`,
  `digstore-cli`, `dig-resolver`). A dependency back into `dig-store` is a defect.
- The `ChainSource` trait (the serving proof's chain-read abstraction) and its live
  `CoinsetChainSource` implementation live behind the `serve` feature (the prover
  module) — no chain-plane dependency.
- The base data plane is `no_std`/wasm-clean (no `blst`, no `getrandom`); the native
  BLS crypto is isolated behind the `crypto` feature. This split is load-bearing: it
  lets the `wasm` read path compile to `wasm32-unknown-unknown`.
- **`@dignetwork/dig-capsule-wasm`** is the browser + Node read-crypto package, built
  from the `dig-capsule` crate's **`wasm`** feature (a `std` build with no
  chia-bls/wasmtime/blst, via `wasm-pack build --no-default-features --features wasm`).
  It ships BOTH
  wasm-bindgen targets in ONE package behind conditional exports: `node` resolves
  to the CommonJS entry, `browser`/`import`/`default` to the ESM entry. The wasm
  binary is byte-identical across targets (one SRI anchor). The public surface
  (`reconstructUrn`, `retrievalKey`, `deriveKey`, `decryptChunk`,
  `encryptResource`, `decryptResource`, `decryptResourceToText`, `verifyInclusion`,
  `readPublicManifest`, `version`) is installed on `globalThis.digClient`; that
  global identifier is a consumption contract (the on.dig.net loader, hub.dig.net)
  and is preserved.

## 7. Conformance

- A conforming reader decodes every released DIGS format version byte-identically
  (§2); the golden-fixture test is the gate.
- The `@dignetwork/dig-capsule-wasm` read-crypto MUST produce byte-identical KDF/AEAD/URN
  output to the native `crypto` path (proven by the native `parity` test, run under
  `--features wasm`) and verify inclusion proofs identically to the host.
- Cross-references: the superproject `SYSTEM.md` (store = chain + capsule, the
  cross-repo capsule contract) and the docs.dig.net protocol pages MUST agree with
  this spec; a shared-contract change updates all three in one unit of work.
