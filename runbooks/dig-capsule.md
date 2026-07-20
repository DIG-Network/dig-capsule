# Runbook — dig-capsule (build, test, publish)

The `.dig` capsule data plane: the single **`dig-capsule`** crate (the curated public
API consumers depend on), whose browser + Node read-crypto is shipped as the
`@dignetwork/dig-capsule-wasm` npm package (built from the crate's `wasm` feature;
packaging harness in `wasm-npm/`).

## Prerequisites

- The pinned Rust toolchain (`rust-toolchain.toml`) with the `wasm32-unknown-unknown`
  target, `rustfmt`, and `clippy`.
- `wasm-pack` and Node 20+ (for the wasm package).
- Windows only: the deep worktree path can trip `libz-sys`/`cmake`. Set a SHORT
  `CARGO_TARGET_DIR` (e.g. `C:/t/capsule`) and, if cmake 4.x complains,
  `CMAKE_POLICY_VERSION_MINIMUM=3.5`.

## Build prerequisite — the guest wasm (bites every fresh checkout)

`build.rs` embeds the REAL guest wasm (BINDING contract D6), so it MUST exist before
anything that compiles the stage/compiler engines. The guest is the SAME crate compiled
under the `guest-wasm` feature to wasm32:

    cargo build --no-default-features --features guest-wasm --target wasm32-unknown-unknown --release

This produces `target/wasm32-unknown-unknown/release/dig_capsule.wasm`. If you build with
a custom `CARGO_TARGET_DIR`, `build.rs` looks in the crate's own `target/...` path, so
either copy that artifact there or set `DIGSTORE_GUEST_WASM=<abs path to that .wasm>` so
build.rs picks it up.

## Build + test

    cargo build
    cargo fmt --all --check
    cargo clippy --all-targets -- -D warnings \
      -A clippy::default_constructed_unit_structs -A clippy::field_reassign_with_default
    cargo test

Coverage (>=80% line gate):

    cargo llvm-cov nextest \
      --ignore-filename-regex '(build\.rs|guest/)' --fail-under-lines 80 \
      -E 'not test(=module_validates_and_exports_full_abi)'

(The excluded test shells out to a nested `cargo build` incompatible with coverage
instrumentation; it still runs in the plain `cargo test` job.)

## The wasm npm package (@dignetwork/dig-capsule-wasm)

From `wasm-npm/` (the packaging harness; builds the root crate's `wasm` feature):

    npm run build:pkg        # wasm-pack web + node (--no-default-features --features wasm), assemble ./pkg
    cargo run --manifest-path ../Cargo.toml --no-default-features --features crypto,wasm --example gen_smoke_fixture > smoke_fixture.json
    node scripts/verify-pkg.mjs               # Node end-to-end test
    # real-browser test (run from the repo ROOT, not wasm-npm/):
    wasm-pack test --headless --chrome --test browser -- --no-default-features --features wasm

The assembled `pkg/` is dual-target (Node CommonJS + browser ESM) behind
conditional exports, with one shared `dig_client_bg.wasm` and an SRI anchor.

## Release + publish (orchestrator)

- On merge to `main`, `.github/workflows/release.yml` regenerates the changelog,
  commits it (RELEASE_TOKEN past branch protection), and pushes the `vX.Y.Z` tag. The
  version is the single `[package].version` in `Cargo.toml` (**0.4.0**), so the release
  tags `v0.4.0`. `COMPILER_VERSION` is the literal `1.0.0`, decoupled from this version.
- The pushed tag fires `.github/workflows/publish-npm.yml`, which rebuilds + publishes
  `@dignetwork/dig-capsule-wasm` to npm (OIDC trusted publishing).
- `release.yml` then publishes the single `dig-capsule` crate to **crates.io** (one
  `cargo publish --no-verify`, idempotent + waits for indexing). crates.io publish IS
  the distribution model; consumers depend on just `dig-capsule` from crates.io (never a
  `git = …` dep).

## Disk hygiene

When done (committed + pushed + CI green), delete your own `target/` (and any custom
`CARGO_TARGET_DIR`) — it is regenerable and git-ignored (CLAUDE.md §1.6).
