//! The facade-coverage contract: every symbol a `dig-capsule` consumer needs is
//! reachable from `dig_capsule::<concept>::…` WITHOUT ever naming the `pub(crate)
//! mod imp` plumbing tree.
//!
//! # Why this file exists
//!
//! `dig-capsule` is one crate behind one curated facade (#1270). Nine downstream
//! crates — `digstore-{core,chunker,crypto,store,guest,prover,host,compiler,stage}`
//! — are being repointed onto that facade, and each symbol the facade fails to
//! carry is a repoint that cannot compile. This file enumerates the union of those
//! nine crates' public surfaces, spelled through the facade, so the requirement is
//! machine-checked instead of assumed.
//!
//! # How it fails
//!
//! Every entry below is a `use` of a public path. Deleting or renaming a re-export
//! in `src/lib.rs` therefore breaks the BUILD of this test, not merely an
//! assertion — a regression cannot be silently absorbed. The bodies are
//! deliberately empty: naming the path IS the assertion, and adding runtime checks
//! here would test the implementation rather than the surface. The one runtime
//! assertion present ([`base_constants_match_the_canonical_urn_owner`]) covers a
//! claim the `use` cannot make: that the no_std constant copies equal the canonical
//! `dig-urn-protocol` values they mirror.
//!
//! # Layout
//!
//! One module per concept module of the facade, gated exactly as the facade gates
//! it, so this file compiles under every feature tier the crate ships.

#![allow(unused_imports)]

// ---------------------------------------------------------------------------
// Base surface — always compiled (the `digstore-core` / `digstore-chunker` half).
// ---------------------------------------------------------------------------

mod capsule_concept {
    use dig_capsule::capsule::{
        Capsule, CapsuleClass, CapsuleSpec, CompilationStats, CompilerError, Generation,
        GenerationId, GenerationState, HostImportsConfig, SecretSalt, StoreConfig, TrustedHostKey,
        Visibility, MAX_STORE_BYTES,
    };

    #[cfg(feature = "std")]
    use dig_capsule::capsule::CompilationResult;
}

mod format_concept {
    use dig_capsule::format::{abi, codec, datasection, serving, wire};
    use dig_capsule::format::{
        is_error, pack_ptr_len, sha256, unpack_ptr_len, Bytes32, Bytes48, Bytes96, CoreError,
        Decode, DecodeError, Decoder, Encode, Encoder, ErrorCode, KeyTableEntry, PathWalk,
        RevocationReason, Tombstone, TombstoneScope, CHAIN, CHIA_BLS_SCHEME, DEFAULT_RESOURCE_KEY,
    };

    // The wire shapes `digstore-core` re-exported at its root; consumers name them
    // through the `wire` module here.
    use dig_capsule::format::wire::{
        AttestationChallenge, AttestationResponse, AuthenticationInfo, ChiaBlockRef,
        ContentResponse, ExecutionProof, ProofPrelude, ProofResponse, ATTEST_DST,
    };
}

mod merkle_concept {
    use dig_capsule::merkle::{
        resource_leaf, MerkleProof, MerkleTree, ProofStep, LEAF_TAG, NODE_TAG,
    };
}

mod chunk_concept {
    use dig_capsule::chunk::{
        chunk_slice, default_config, hash_data, mask_for_target, Chunk, Chunker, ChunkerConfig,
        GEAR_TABLE,
    };

    #[cfg(feature = "std")]
    use dig_capsule::chunk::chunk_stream;
}

mod metadata_concept {
    use dig_capsule::metadata::{
        Author, MetadataManifest, PublicManifest, PublicManifestEntry,
        PUBLIC_MANIFEST_SCHEMA_VERSION,
    };
}

mod crypto_primitives_concept {
    // Base-available (no `blst`, no `getrandom`) so a slim reader can open a chunk.
    use dig_capsule::crypto::primitives::{decrypt_chunk, derive_decryption_key, encrypt_chunk};
}

// ---------------------------------------------------------------------------
// Feature-gated surfaces.
// ---------------------------------------------------------------------------

#[cfg(feature = "std")]
mod urn_concept {
    use dig_capsule::urn::{
        capsule_from_urn, DigUrn, SecretSalt, UrnBytes32, UrnParseError, CANONICAL_CHAIN,
        DEFAULT_RESOURCE_KEY, SALT_QUERY_MARKER, URN_ABNF, URN_PREFIX,
    };
}

#[cfg(feature = "reader")]
mod reader_concept {
    use dig_capsule::reader::ModuleReadError;
}

#[cfg(feature = "crypto")]
mod crypto_concept {
    use dig_capsule::crypto::{aead, bls, error, fixtures, kdf};
    use dig_capsule::crypto::{
        attestation_signing_message, bls_keygen, bls_sign, bls_verify, decrypt_and_unwrap,
        decrypt_chunk, derive_decryption_key, encrypt_chunk, node_signing_message,
        push_signing_message, request_signing_message, sha256, sign_attestation, sign_node,
        sign_push, sign_request, sign_tombstone, tombstone_signing_message, validate_public_key,
        verify_push, verify_request, verify_tombstone, BlsError, BlsFixture, BlsFixtureSet,
        CryptoError, KdfFixture, KdfFixtureSet, TamperError, CHIA_BLS_SCHEME, CRYPTO_VERSION,
    };
}

#[cfg(feature = "store")]
mod store_concept {
    use dig_capsule::store::{
        build_public_manifest, load_config, save_config, ChunkRef, ChunkStore, Clock, FixedClock,
        GenerationDiff, GenerationManifest, KeyTableRecord, Result as StoreResult, RootHistory,
        StagedRecord, StagingArea, Store, StoreError, StorePaths, SystemClock,
    };
}

#[cfg(feature = "compile")]
mod compile_concept {
    use dig_capsule::compile::{
        assert_host_imports, assert_memory_ceiling, atomic_write_module, baked_template_bytes,
        build_chunk_index_and_key_table, default_uniform_blob_len, deterministic_filler,
        encode_data_section, extract_data_section, extract_data_section_blob, inject_data_section,
        load_template, obfuscate, output_filename, rekey_module_trusted, verify_module_root,
        ChunkIndex, CompilationResult, CompilationStats, CompileOutcome, Compiler, CompilerConfig,
        CompilerError, CompilerStats, DataSectionInputs, GenerationView, KeyTable, ModuleIdentity,
        ResourceView, Result as CompileResult, Template, COMPILER_VERSION, DATA_SECTION_MEM_OFFSET,
        FIXED_BLOB_LEN, MAX_MEMORY_PAGES, REQUIRED_EXPORTS, REQUIRED_HOST_IMPORTS,
        UNIFORM_BLOB_LEN_ENV,
    };
}

#[cfg(feature = "compile")]
mod stage_concept {
    use dig_capsule::stage::{
        build_prepared, canonical_resource_urn, chunker_config, embedded_guest_wasm,
        empty_manifest, ephemeral_config, finalize, manifest_from_json, no_auth, stage_and_compile,
        CompiledCapsule, FinalizeOptions, PreparedCommit, StageError,
    };
}

#[cfg(feature = "serve")]
mod host_concept {
    use dig_capsule::host::{
        request_for_retrieval_key, serve_blind, serve_blind_with, AttestationBackend,
        BlindServeConfig, BlindServeDeps, BlsAttestationBackend, Clock, ExecutionLimits,
        FixedClock, HostDeps, HostError, HostKeys, HostRng, HostRuntime, HostState, ReturnBuffer,
        RuntimeState, Session, SessionTable, SharedBackend, SystemClock, MAX_MEMORY_BYTES,
        WASM_PAGE_SIZE,
    };
}

#[cfg(feature = "serve")]
mod prover_concept {
    use dig_capsule::prover::{
        bound_public_output, build_public_input, parse_public_input, signing_message, ChainSource,
        CoinsetChainSource, MockChainSource, MockProver, MockVerifier, Prover, ProverError,
        Result as ProverResult, ServingInputs, Verifier, DEFAULT_FRESHNESS_WINDOW_SECS, NONCE_LEN,
    };
    use dig_capsule::prover::{
        chain, coinset, commitment, error, mock, mock_chain, prover, serving_inputs,
    };
}

#[cfg(feature = "serve")]
mod guest_concept {
    use dig_capsule::guest::{
        allocator, attestation, content, datasection, decoy, host, jwt, metadata,
        obfuscation_hooks, oblivious, packing, proof, request, session, temporal,
    };
}

// ---------------------------------------------------------------------------
// The one claim a `use` cannot make.
// ---------------------------------------------------------------------------

/// The no_std constant copies in [`dig_capsule::format`] MUST equal the canonical
/// `dig-urn-protocol` values re-exported by [`dig_capsule::urn`].
///
/// They exist only because the `urn` module needs `std` and the guest does not have
/// it. A `use` proves both names resolve; it cannot prove they agree, and a skew
/// here would make a guest derive a retrieval key the resolver never asks for.
#[cfg(feature = "std")]
#[test]
fn base_constants_match_the_canonical_urn_owner() {
    assert_eq!(
        dig_capsule::format::CHAIN,
        dig_capsule::urn::CANONICAL_CHAIN
    );
    assert_eq!(
        dig_capsule::format::DEFAULT_RESOURCE_KEY,
        dig_capsule::urn::DEFAULT_RESOURCE_KEY
    );
}
