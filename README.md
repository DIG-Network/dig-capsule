# dig-capsule

The **DIG capsule data plane** — the `.dig` / DIGS format, the compiler that builds
a capsule from files, the capsule read-crypto, staging, the guest/host serve triad,
and the capsule size ladder. A capsule is the DATA portion of a store; the on-chain
anchor (singleton, storeId, generations) lives in
[`dig-store`](https://github.com/DIG-Network/digs), which depends on this crate.

## Depend on just `dig-capsule`

`dig-capsule` is ONE crate with a curated **facade** (`src/lib.rs`): one clean public
API over feature-gated modules. The internal split lives under `src/imp/` and is an
implementation detail — you use the concept modules, never reach into the internals.
The whole `.dig` format-manager API is learnable from this crate's docs alone.

```toml
[dependencies]
dig-capsule = "0.4"                                   # the whole API (default = full)
# slim readers (e.g. a URN resolver) take base only (+ `std` for the urn scheme):
# dig-capsule = { version = "0.4", default-features = false, features = ["std"] }
```

```rust
use dig_capsule::prelude::*;

let spec = CapsuleClass::DEFAULT.spec();              // 128 MB, the canonical size
// The canonical URN (owned by `dig-urn-protocol`, re-exported through the facade):
let urn = DigUrn::parse("urn:dig:chia:0000000000000000000000000000000000000000000000000000000000000000").unwrap();
let key = urn.content_key();                          // frozen root-independent key derivation
```

The concept modules: `capsule`, `urn`, `format`, `merkle`, `chunk`, `metadata` (base);
`crypto`, `store`, `compile`, `stage`, `host`, `prover`, `guest` (feature-gated). See
the crate rustdoc (`cargo doc --open`) for the full re-export map.

### Feature flags

| Feature | Enables | Typical consumer |
|---------|---------|------------------|
| *(base)* | `no_std`+`alloc`: format / merkle / chunk / metadata — no wasmtime, no `blst`, no `getrandom` | slim embedded readers |
| `std` | lifts out of `no_std`; adds the canonical `urn` scheme + `chunk::chunk_stream` | `dig-urn-resolver` (`default-features = false, features = ["std"]`) |
| `default = ["full"]` | `crypto + store + compile + serve` — the whole API | `dig-store`, `dig-node` |
| `crypto` | native AES-256-GCM-SIV AEAD + Chia-BLS (implies `std`) | |
| `store` | on-disk generation / staging model (implies `crypto`) | |
| `compile` | files→capsule pipeline (implies `store` + `crypto`) | |
| `serve` | blind serve triad + serving proofs (implies `crypto`) | |
| `wasm` | the browser/Node read-crypto surface (`@dignetwork/dig-capsule-wasm`) | |
| `guest-wasm` | the self-serving guest cdylib compiled to wasm32 | build.rs guest embed |
| `risc0` | the real RISC0 serving-proof circuit (OFF by default; needs the RISC0 toolchain) | |

### The browser counterpart

The browser + Node read-crypto is NOT a Rust dependency: it is the
**`@dignetwork/dig-capsule-wasm`** npm package, whose surface (`reconstructUrn`,
`retrievalKey`, `deriveKey`, `verifyInclusion`, `decryptResource`,
`decryptResourceToText`, `readPublicManifest`, `version`) is installed on
`globalThis.digClient`. It produces byte-identical KDF/AEAD/URN/merkle output to the
native `crypto` module here.

## Layout (implementation detail — use the facade modules, not these)

The internal split lives under `src/imp/` and is a `pub(crate)` implementation detail;
the top-level facade modules (`capsule`, `format`, `merkle`, `chunk`, `metadata`,
`urn`, `crypto`, `store`, `compile`, `stage`, `host`, `prover`, `guest`) are the public
surface.

- `src/imp/core` — DIGS format/datasection, capsule identity, codec, sizes, chunk-seal
  + KDF crypto, keytable, manifest, merkle, wire, ABI (`no_std`/wasm-clean). The
  `urn:dig:` scheme is NOT here — it is the canonical `dig-urn-protocol` crate,
  re-exported through the facade `urn` module (requires `std`).
- `src/imp/chunker` — content chunking.
- `src/imp/crypto` — AEAD + chia-bls signing + serving-proof crypto (native, `crypto`).
- `src/imp/store` — chunkstore / generation / history / staging / diff (`store`).
- `src/imp/guest`, `src/imp/host`, `src/imp/prover` — the guest/host serve triad + §13
  serving proofs (`ChainSource` + `CoinsetChainSource`) (`serve`/`host`).
- `src/imp/compiler`, `src/imp/stage` — files → self-serving capsule module + the
  stage → compile build pipeline (`compile`).
- `src/wasm_browser.rs` (the **`wasm`** feature) — the browser + Node read-crypto behind
  the **`@dignetwork/dig-capsule-wasm`** npm package, built with
  `wasm-pack build --no-default-features --features wasm` (packaging harness in
  `wasm-npm/`).
- `guest-risc0/` — the RISC0 serving-proof guest, a NESTED non-published package built
  only under `risc0` (referenced via `[package.metadata.risc0]`).

## Docs

- `SPEC.md` — the normative data-plane contract.
- `runbooks/` — build, test, and publish procedures.

Licensed GPL-2.0-only.
