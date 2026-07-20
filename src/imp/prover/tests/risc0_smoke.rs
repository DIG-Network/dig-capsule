#![cfg(feature = "risc0")]
//! REAL risc0 prove -> verify roundtrip tests for the `risc0` execution-proof
//! backend (#1277). These exercise the ACTUAL [`Risc0Prover`]/[`Risc0Verifier`]
//! code path — build [`ExecutorEnv`], run the guest ELF, decode the journal, and
//! verify the receipt — NOT the mock prover (only the CHAIN leg is mocked, via
//! [`MockChainSource`]).
//!
//! Proving is slow with the real prover, so run these under `RISC0_DEV_MODE=1`,
//! where the same code path executes the guest and produces a (dev) receipt in
//! seconds. CI runs them exactly this way (see the `risc0 (execution proofs)`
//! job in `ci.yml`). The four cases assert:
//!   * VALID           -> verify PASSES and `public_output` matches the recompute.
//!   * TAMPERED PROOF  -> [`ProverError::ZkProofInvalid`] (corrupt receipt bytes).
//!   * WRONG program   -> [`ProverError::ProgramHashMismatch`].
//!   * WRONG journal   -> [`ProverError::PublicOutputMismatch`] (tampered output)
//!                        and [`ProverError::PublicInputMismatch`] (tampered input).
use crate::imp::core::{Bytes32, ChiaBlockRef, ExecutionProof};
use crate::imp::crypto::bls;
use crate::imp::prover::build_public_input;
use crate::imp::prover::risc0_backend::{Risc0Prover, Risc0Verifier};
use crate::imp::prover::{MockChainSource, Prover, ProverError, ServingInputs, Verifier};

/// The shared inputs the four cases build on. The block, program hash, root, and
/// serving inputs are fixed so each case differs only in the single value it tampers.
struct Fixture {
    program_hash: Bytes32,
    root: Bytes32,
    public_input: Vec<u8>,
    block: ChiaBlockRef,
    serving: ServingInputs,
    prover: Risc0Prover,
}

impl Fixture {
    fn new() -> Self {
        let sk = bls::SecretKey::from_seed(&[7u8; 32]);
        let pk = sk.public_key();
        let block = ChiaBlockRef {
            header_hash: Bytes32([0x55u8; 32]),
            height: 42,
            timestamp: 1_000_000,
        };
        let program_hash = Bytes32([0xAAu8; 32]);
        let root = Bytes32([0xBBu8; 32]);
        let public_input = build_public_input(&[0x33u8; 32], &block);
        let serving = ServingInputs {
            retrieval_key: Bytes32([1u8; 32]),
            roothash: root,
            chunk_ciphertext: vec![vec![0xDE, 0xAD], vec![0xBE, 0xEF]],
        };
        let prover = Risc0Prover::new(sk, pk, block.clone());
        Self {
            program_hash,
            root,
            public_input,
            block,
            serving,
            prover,
        }
    }

    /// A real receipt: build the env, run the guest, and bincode-serialize the
    /// receipt into an [`ExecutionProof`] via the actual prover.
    fn prove(&self) -> ExecutionProof {
        self.prover
            .prove(self.program_hash, &self.public_input, &self.serving)
            .expect("risc0 proving must succeed")
    }

    fn chain(&self) -> MockChainSource {
        MockChainSource::new(vec![self.block.clone()], self.block.timestamp + 100)
    }
}

/// VALID: the honest proof verifies, and the committed `public_output` equals the
/// independent recompute of `SHA-256(roothash || concat(chunks))`.
#[test]
fn risc0_prove_verify_smoke() {
    let fx = Fixture::new();
    let proof = fx.prove();
    assert_eq!(proof.public_output, fx.serving.compute_public_output());

    Risc0Verifier::default()
        .verify(&proof, fx.program_hash, &[fx.root], &fx.chain())
        .expect("risc0 proof must verify");
}

/// (a) TAMPERED PROOF: corrupting the serialized receipt bytes must be caught.
/// Truncating the bincode blob reliably fails receipt deserialization even in
/// dev mode, surfacing as [`ProverError::ZkProofInvalid`].
#[test]
fn risc0_tampered_proof_rejected() {
    let fx = Fixture::new();
    let mut proof = fx.prove();
    // Truncate the receipt bytes: bincode deserialize can no longer reconstruct
    // the Receipt, so verify fails before it ever reaches the zk check.
    proof.proof.truncate(proof.proof.len() / 2);

    let err = Risc0Verifier::default()
        .verify(&proof, fx.program_hash, &[fx.root], &fx.chain())
        .unwrap_err();
    assert!(
        matches!(err, ProverError::ZkProofInvalid(_)),
        "expected ZkProofInvalid, got {err:?}"
    );
}

/// (b) WRONG program_hash: verifying against a program hash the caller did not
/// prove against is rejected up front with [`ProverError::ProgramHashMismatch`].
#[test]
fn risc0_wrong_program_hash_rejected() {
    let fx = Fixture::new();
    let proof = fx.prove();
    let wrong_program_hash = Bytes32([0xCCu8; 32]);

    let err = Risc0Verifier::default()
        .verify(&proof, wrong_program_hash, &[fx.root], &fx.chain())
        .unwrap_err();
    assert!(
        matches!(err, ProverError::ProgramHashMismatch { .. }),
        "expected ProgramHashMismatch, got {err:?}"
    );
}

/// (c1) WRONG journal — tampered output: claiming a different `public_output`
/// than the guest committed is caught by the journal comparison as
/// [`ProverError::PublicOutputMismatch`].
#[test]
fn risc0_tampered_output_rejected() {
    let fx = Fixture::new();
    let mut proof = fx.prove();
    proof.public_output = Bytes32([0xEEu8; 32]); // claim an output the guest never committed

    let err = Risc0Verifier::default()
        .verify(&proof, fx.program_hash, &[fx.root], &fx.chain())
        .unwrap_err();
    assert!(
        matches!(err, ProverError::PublicOutputMismatch),
        "expected PublicOutputMismatch, got {err:?}"
    );
}

/// (c2) WRONG journal — tampered public input: swapping in a different-nonce
/// public input (same block, so the block check still passes) breaks the
/// `SHA-256(public_input)` the guest committed, caught as
/// [`ProverError::PublicInputMismatch`].
#[test]
fn risc0_tampered_public_input_rejected() {
    let fx = Fixture::new();
    let mut proof = fx.prove();
    // Same block, different nonce -> parses fine, block check passes, but the
    // journal's committed public-input hash no longer matches.
    proof.public_input = build_public_input(&[0x99u8; 32], &fx.block);

    let err = Risc0Verifier::default()
        .verify(&proof, fx.program_hash, &[fx.root], &fx.chain())
        .unwrap_err();
    assert!(
        matches!(err, ProverError::PublicInputMismatch),
        "expected PublicInputMismatch, got {err:?}"
    );
}
