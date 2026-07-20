//! Shared test helpers for dig-capsule-host integration tests.

use crate::imp::core::types::{Bytes32, Bytes48};
use crate::imp::core::ChiaBlockRef;
use crate::imp::crypto::bls::BlsSecretKey;
use crate::imp::host::{FixedClock, HostDeps};
use crate::imp::prover::{MockChainSource, MockProver};
use std::sync::Arc;

/// Build HostDeps with a deterministic BLS key, mock chain, and mock prover.
/// `clock` is shared (FixedClock clones share their counter) so tests can advance it.
pub fn test_deps(clock: FixedClock) -> HostDeps {
    let sk = BlsSecretKey::from_seed(&[42u8; 32]);
    let pk: Bytes48 = sk.public_key().to_bytes();

    // A separate (deterministic) key + a known chain block back the mock prover.
    let prover_sk = BlsSecretKey::from_seed(&[7u8; 32]);
    let prover_pk = prover_sk.public_key();
    let block = ChiaBlockRef {
        header_hash: Bytes32([0x55u8; 32]),
        height: 100,
        timestamp: 1_700_000_000,
    };
    let chain = MockChainSource::new(vec![block.clone()], 1_700_000_000);
    let prover = MockProver::new(prover_sk, prover_pk, block);

    HostDeps {
        store_id: Bytes32([0u8; 32]),
        bls_secret: sk,
        bls_public: pk,
        clock: Arc::new(clock),
        chain: Arc::new(chain),
        prover: Arc::new(prover),
        rng_seed: Some([99u8; 32]),
        instance_id: Bytes32([1u8; 32]),
        attestation: None,
    }
}
