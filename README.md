# dig-capsule

The **DIG capsule data plane** — the `.dig` / DIGS format, the compiler that builds
a capsule from files, the capsule read-crypto, staging, the guest/host serve triad,
and the capsule size ladder. A capsule is the DATA portion of a store; the on-chain
anchor (singleton, storeId, generations) lives in
[`dig-store`](https://github.com/DIG-Network/digs), which depends on this crate.

## Depend on just `dig-capsule`

`dig-capsule` is a curated **facade**: one clean public API over the workspace's
`dig-capsule-*` member crates. Consumers depend on this ONE crate and use its concept
modules — you never reach into `dig-capsule-core` / `-crypto` / `-store` / … directly.
The whole `.dig` format-manager API is learnable from this crate's docs alone.

```toml
[dependencies]
dig-capsule = "0.3"                                   # the whole API (default = full)
# slim readers (e.g. a URN resolver) take base only:
# dig-capsule = { version = "0.3", default-features = false }
```

```rust
use dig_capsule::prelude::*;

let spec = CapsuleClass::DEFAULT.spec();              // 128 MB, the canonical size
let urn = Urn::parse("urn:dig:chia:00").unwrap();
let key: Bytes32 = urn.retrieval_key();               // frozen retrieval-key derivation
```

The concept modules: `capsule`, `urn`, `format`, `merkle`, `chunk`, `metadata` (base);
`crypto`, `store`, `compile`, `stage`, `host`, `prover`, `guest` (feature-gated). See
the crate rustdoc (`cargo doc --open`) for the full re-export map.

### Feature flags

| Feature | Enables | Typical consumer |
|---------|---------|------------------|
| *(base)* | read / format / urn / merkle / chunk / metadata — no wasmtime, no `blst` | `dig-urn-resolver` (`default-features = false`) |
| `default = ["full"]` | `crypto + store + compile + serve` — the whole API | `dig-store`, `dig-node` |
| `crypto` | native AES-256-GCM-SIV AEAD + Chia-BLS | |
| `store` | on-disk generation / staging model | |
| `compile` | files→capsule pipeline (implies `store` + `crypto`) | |
| `serve` | blind serve triad + serving proofs (implies `crypto`) | |
| `risc0` | the real RISC0 serving-proof circuit (OFF by default; needs the RISC0 toolchain) | |

### The browser counterpart

The browser + Node read-crypto is NOT a Rust dependency: it is the
**`@dignetwork/dig-capsule-wasm`** npm package, whose surface (`reconstructUrn`,
`retrievalKey`, `deriveKey`, `verifyInclusion`, `decryptResource`,
`decryptResourceToText`, `readPublicManifest`, `version`) is installed on
`globalThis.digClient`. It produces byte-identical KDF/AEAD/URN/merkle output to the
native `crypto` module here.

## Layout (implementation detail — depend on the facade, not these)

- `crates/dig-capsule-core` — DIGS format/datasection, capsule identity, codec, sizes,
  chunk-seal + KDF crypto, keytable, manifest, merkle, URN, wire, ABI (`no_std`/
  wasm-clean).
- `crates/dig-capsule-chunker` — content chunking.
- `crates/dig-capsule-crypto` — AEAD + chia-bls signing + serving-proof crypto (native).
- `crates/dig-capsule-store` — chunkstore / generation / history / staging / diff.
- `crates/dig-capsule-guest`, `crates/dig-capsule-host`, `crates/dig-capsule-prover` — the
  guest/host serve triad + §13 serving proofs (`ChainSource` + `CoinsetChainSource`).
- `crates/dig-capsule-compiler` — files → self-serving capsule module.
- `crates/dig-capsule-stage` — the stage → compile build pipeline.
- `crates/dig-capsule-wasm` — the browser + Node read-crypto behind the
  **`@dignetwork/dig-capsule-wasm`** npm package (excluded from the workspace; wasm32-only).

## Docs

- `SPEC.md` — the normative data-plane contract.
- `runbooks/` — build, test, and publish procedures.

Licensed GPL-2.0-only.
