# dig-capsule

The **DIG capsule data plane** — the `.dig` / DIGS format, the compiler that builds
a capsule from files, the capsule read-crypto, staging, the guest/host serve triad,
and the capsule size ladder. A capsule is the DATA portion of a store; the on-chain
anchor (singleton, storeId, generations) lives in
[`dig-store`](https://github.com/DIG-Network/dig-store), which depends on this crate.

Lifted from `dig-store` (epic #744). The crate names (`digstore-core`, …) are
preserved verbatim so consumers change only the git URL, not their `use` paths.

## Layout

- `crates/digstore-core` — DIGS format/datasection, capsule identity, codec, sizes,
  chunk-seal + KDF crypto, keytable, manifest, merkle, URN, wire, ABI (`no_std`/
  wasm-clean).
- `crates/digstore-chunker` — content chunking.
- `crates/digstore-crypto` — AEAD + chia-bls signing + serving-proof crypto (native).
- `crates/digstore-store` — chunkstore / generation / history / staging / diff.
- `crates/digstore-guest`, `crates/digstore-host`, `crates/digstore-prover` — the
  guest/host serve triad + §13 serving proofs (`ChainSource` + `CoinsetChainSource`).
- `crates/digstore-compiler` — files → self-serving capsule module.
- `crates/digstore-stage` — the stage → compile build pipeline.
- `crates/dig-client-wasm` — the browser + Node read-crypto behind the
  **`@dignetwork/dig-capsule-wasm`** npm package (excluded from the workspace; wasm32-only).

## Docs

- `SPEC.md` — the normative data-plane contract.
- `runbooks/` — build, test, and publish procedures.

Licensed GPL-2.0-only.

<!-- WIP: rename digstore-* members -> dig-capsule-* + crates.io publish (#1247) -->
