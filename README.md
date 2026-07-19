# dig-capsule

The **DIG capsule data plane** — the `.dig` / DIGS format, the compiler that builds
a capsule from files, the capsule read-crypto, staging, the guest/host serve triad,
and the capsule size ladder. A capsule is the DATA portion of a store; the on-chain
anchor (singleton, storeId, generations) lives in
[`dig-store`](https://github.com/DIG-Network/dig-store), which depends on this crate.

Lifted from `dig-store` (epic #744). The crate names (`dig-capsule-core`, …) are
preserved verbatim so consumers change only the git URL, not their `use` paths.

## Layout

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
