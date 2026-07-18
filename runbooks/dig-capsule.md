# Runbook — dig-capsule (build, test, publish)

The `.dig` capsule data plane: a Cargo workspace (9 crates) plus the
`@dignetwork/dig-capsule-wasm` wasm npm package (built from `crates/dig-client-wasm`).

## Prerequisites

- The pinned Rust toolchain (`rust-toolchain.toml`) with the `wasm32-unknown-unknown`
  target, `rustfmt`, and `clippy`.
- `wasm-pack` and Node 20+ (for the wasm package).
- Windows only: the deep worktree path can trip `libz-sys`/`cmake`. Set a SHORT
  `CARGO_TARGET_DIR` (e.g. `C:/t/capsule`) and, if cmake 4.x complains,
  `CMAKE_POLICY_VERSION_MINIMUM=3.5`.

## Build prerequisite — the guest wasm (bites every fresh checkout)

`digstore-stage`'s `build.rs` embeds the REAL guest wasm (BINDING contract D6), so
it MUST exist before anything that compiles the stage/compiler engines:

    cargo build -p digstore-guest --target wasm32-unknown-unknown --release

This produces `target/wasm32-unknown-unknown/release/digstore_guest.wasm`. If you
build with a custom `CARGO_TARGET_DIR`, either copy that artifact to the workspace
`target/...` path (some runtime tests read it there) or set
`DIGSTORE_GUEST_WASM=<abs path>` so the build.rs picks it up.

## Build + test the workspace

    cargo build --workspace
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings \
      -A clippy::default_constructed_unit_structs -A clippy::field_reassign_with_default
    cargo test --workspace

Coverage (>=80% line gate):

    cargo llvm-cov nextest --workspace \
      --ignore-filename-regex '(build\.rs|guest/)' --fail-under-lines 80 \
      -E 'not test(=module_validates_and_exports_full_abi)'

(The excluded test shells out to a nested `cargo build` incompatible with coverage
instrumentation; it still runs in the plain `cargo test` job.)

## The wasm npm package (@dignetwork/dig-capsule-wasm)

From `crates/dig-client-wasm` (excluded from the workspace — wasm32-only):

    npm run build:pkg        # wasm-pack web + node, assemble ./pkg
    cargo run --example gen_smoke_fixture > smoke_fixture.json
    node scripts/verify-pkg.mjs               # Node end-to-end test
    wasm-pack test --headless --chrome --test browser   # real-browser test

The assembled `pkg/` is dual-target (Node CommonJS + browser ESM) behind
conditional exports, with one shared `dig_client_bg.wasm` and an SRI anchor.

## Release + publish (orchestrator)

- On merge to `main`, `.github/workflows/release.yml` regenerates the changelog,
  commits it (RELEASE_TOKEN past branch protection), and pushes the `vX.Y.Z` tag
  from the workspace `Cargo.toml` version.
- The published GitHub Release fires `.github/workflows/publish-npm.yml`, which
  rebuilds + publishes `@dignetwork/dig-capsule-wasm` to npm (org `NPM_TOKEN`).
- The Rust crates are consumed by a git-tag pin (crates.io publish is a later goal).

## Disk hygiene

When done (committed + pushed + CI green), delete your own `target/` (and any custom
`CARGO_TARGET_DIR`) — it is regenerable and git-ignored (CLAUDE.md §1.6).
