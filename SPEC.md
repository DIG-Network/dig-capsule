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

This workspace was lifted verbatim from `dig-store` (epic #744 Phase 1); the crate
names (`digstore-core`, …) are preserved so consumers change only the git URL.

## 1. Capsule identity

- A **capsule** is one immutable store generation: the pair `(store_id, root_hash)`,
  each a 32-byte value.
- Canonical string form is `storeId:rootHash` — lowercase hex, colon-separated
  (`digstore_core::Capsule::{canonical, from_canonical}`). A store is a sequence
  of capsules identified by `store_id`; each capsule is one on-chain-anchored root.
- The content URN is `urn:dig:chia:<store_id>[/<resource_key>]` (root-independent)
  or the display form `urn:dig:chia:<store_id>:<root>/<resource_key>`
  (`digstore_core::urn`). The `retrieval_key` is derived from the canonical URN and
  is the stable per-resource lookup key; this derivation is FROZEN.

## 2. The DIGS data section

- Magic `DIGS` (4 bytes), then a `u8` `format_version` = **1**, then an offset
  table of `(section_id: u16, offset, len)` entries (`digstore_core::codec::section`,
  `digstore_core::datasection`).
- Section ids form a registry (store id, current root, root history, store pubkey,
  trusted host keys, metadata manifest, authentication info, key table, merkle
  leaves, chunks, public manifest, …). See `digstore_core::datasection::SectionId`.
- **Backwards compatibility (HARD, CLAUDE.md §5.1): additive only.** New
  section ids and new optional fields may be added. Existing section ids MUST NOT
  be removed, renumbered, or repurposed; an existing field's meaning/encoding MUST
  NOT change. A reader ignores unknown section ids. `format_version` stays 1; a
  bump means "new writers MAY emit vN", never "readers reject < vN".
- The byte-exact layout is pinned by the golden fixture
  `crates/digstore-compiler/tests/fixtures/golden_data_section.hex` and its
  byte-identical-read test `data_section_golden.rs`. Every format change MUST keep
  that test green.

## 3. Capsule read-crypto

- Per-resource content is chunked (`digstore-chunker`), then each chunk is sealed
  with AES-256-GCM-SIV under a key derived by HKDF from the canonical resource URN
  (and, for private stores, a 32-byte secret salt) (`digstore_core::crypto`,
  `digstore-crypto`). The host serves ciphertext and is BLIND to plaintext.
- Content commitment is a Merkle tree over the sealed chunk leaves; a served
  `ContentResponse` carries a merkle inclusion proof that verifies to the capsule
  root, plus per-chunk ciphertext lengths so a streaming client can split and
  GCM-SIV-open each chunk (`digstore_core::merkle`, `digstore_core::serving`).
- The verifier leaf-binding contract: a served leaf MUST equal
  `sha256(served ciphertext)`. The browser read path (`dig-client-wasm`) recomputes
  the leaf from received bytes and rejects a mismatch.
- The signing/serving-proof crypto (chia-bls) lives in `digstore-crypto`; it is
  native-only and MUST NOT be pulled into the wasm read path (§6).

## 4. The compiler and staging

- The **compiler** (`digstore-compiler`) builds a self-serving wasm capsule module
  from a generation's staged content: it embeds the DIGS data section into a guest
  template and pads to a uniform blob so the module size reveals nothing about the
  plaintext.
- **Staging / local build model** (`digstore-store`): chunk store, generation and
  history model, staging, and diff — the local model a capsule is committed from.
- The **build pipeline** (`digstore-stage`) drives stage → compile, embedding the
  REAL `digstore-guest` wasm (BINDING contract D6) so the compiled module serves
  itself. The guest wasm is produced by
  `cargo build -p digstore-guest --target wasm32-unknown-unknown --release`; an
  out-of-workspace (git-dependency) build supplies it via the `DIGSTORE_GUEST_WASM`
  environment variable (absolute path).
- The **guest/host triad**: `digstore-guest` (the in-module logic:
  `get_content`/`get_proof`), `digstore-host` (the wasmtime runtime that serves a
  compiled module blind), and `digstore-prover` (§13 serving/execution proofs and
  chain anchoring).

## 5. The capsule size ladder

- Capsules are padded to a uniform blob sized by a **size class**
  (`digstore_core::capsule_class::CapsuleClass` / `CapsuleSpec`).
- The ladder is powers of 2 MB from 2 MB up to the first rung ≥ 1 GB:
  `{2, 4, 8, 16, 32, 64, 128, 256, 512, 1024} × 10^6 bytes` (each rung is
  `2 MB · 2^k`, `k = 0..=9`; `1024 MB` is the top).
- **DEFAULT = `CapsuleClass::Mb128`** (128 MB): its content cap is exactly
  `digstore_core::MAX_STORE_BYTES = 128_000_000`, THE single canonical
  capsule-size number (#130). Its uniform blob is 128 MiB
  (`digstore-compiler::FIXED_BLOB_LEN`), and the invariant
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
  `CoinsetChainSource` implementation live in `digstore-prover` — no chain-plane
  dependency.
- `digstore-core` is `no_std`/wasm-clean (no `blst`, no `getrandom`); the native
  BLS crypto is isolated in `digstore-crypto`. This split is load-bearing: it lets
  the wasm read path compile to `wasm32-unknown-unknown`.
- **`@dignetwork/dig-client`** is the browser + Node read-crypto package, built
  from `crates/dig-client-wasm` (depends on `digstore-core` only). It ships BOTH
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
- The `@dignetwork/dig-client` read-crypto MUST produce byte-identical KDF/AEAD/URN
  output to the native `digstore-crypto` (proven by `dig-client-wasm`'s native
  `parity` oracle) and verify inclusion proofs identically to the host.
- Cross-references: the superproject `SYSTEM.md` (store = chain + capsule, the
  cross-repo capsule contract) and the docs.dig.net protocol pages MUST agree with
  this spec; a shared-contract change updates all three in one unit of work.
