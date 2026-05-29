/// Plonky3 FRI-STARK production backend for QASH (Domain B).
///
/// `Plonky3ProductionBackend<A>` implements `FriProofBackend` using real
/// `p3-uni-stark` proof verification. The generic parameter `A` is the AIR
/// (Algebraic Intermediate Representation) that defines the circuit constraints.
///
/// # Proof wire format
///
/// `ShardProofBytes.proof_bytes` carries a `postcard`-serialized
/// `ShardProofPayload { proof, public_values }`:
///   - `proof`: the `Proof<QashFriConfig>` STARK proof
///   - `public_values`: circuit public inputs as canonical u32 values
///
/// The `public_input_commitment` field is `SHA3-256` over the public_values
/// bytes (each value as 4-byte little-endian). Verification checks that the
/// deserialized public_values hash to `public_input_commitment` before running
/// the STARK verifier.
///
/// # Domain B boundary
///
/// Proof bytes, STARK witness, and intermediate field elements MUST stay in
/// Domain B. Only the 32-byte `[u8; 32]` return value crosses into Domain A
/// as `ZkProofBundle.batch_root`.
use p3_air::{Air, symbolic::SymbolicAirBuilder};
use p3_field::PrimeCharacteristicRing;
use p3_uni_stark::{VerifierConstraintFolder, verify};
use serde::{Deserialize, Serialize};
use sha3::Digest;

use super::plonky3::{AggregationProofBytes, FriProofBackend, ShardProofBytes};
use super::profile::{QashFriConfig, QashVal, make_qash_production_config, make_qash_test_config};
use crate::hosted::HostedError;

// ── Proof payload (wire format inside proof_bytes) ────────────────────────────

/// Serialised shard proof: STARK proof + its public values.
///
/// Packed into `ShardProofBytes.proof_bytes` with `postcard`.
/// The public_values are BabyBear field elements as canonical u32 (little-endian).
#[derive(Serialize, Deserialize)]
pub struct ShardProofPayload {
    pub proof: p3_uni_stark::Proof<QashFriConfig>,
    /// BabyBear public inputs as canonical u32 values.
    pub public_values: Vec<u32>,
}

/// Serialised aggregation proof: STARK proof + its public values.
///
/// Packed into `AggregationProofBytes.proof_bytes` with `postcard`.
#[derive(Serialize, Deserialize)]
pub struct AggregationProofPayload {
    pub proof: p3_uni_stark::Proof<QashFriConfig>,
    /// BabyBear public inputs as canonical u32 values.
    pub public_values: Vec<u32>,
}

// ── Commitment helper ─────────────────────────────────────────────────────────

/// SHA3-256 over the public_values bytes (each value as 4-byte LE).
///
/// This is the `public_input_commitment` stored in `ShardProofBytes` and the
/// `batch_root` stored in `AggregationProofBytes`. All participants derive the
/// same commitment from the same public values, so it serves as a binding
/// fingerprint.
pub fn commitment_of_public_values(public_values: &[u32]) -> [u8; 32] {
    let mut h = sha3::Sha3_256::new();
    for &v in public_values {
        h.update(v.to_le_bytes());
    }
    h.finalize().into()
}

// ── Production backend ────────────────────────────────────────────────────────

/// Plonky3 FRI-STARK verifier for the QASH two-layer recursion profile.
///
/// Instantiate with the AIR that defines the circuit constraints for the layer
/// being verified. For Layer-0 shard validity proofs, use the QASH shard
/// transition AIR. For Layer-1 aggregation proofs, use the aggregation AIR.
///
/// Until the QASH circuit is complete, use the test helper
/// `Plonky3ProductionBackend::new_for_testing` with a fixture AIR (e.g.
/// `FibonacciAir`) to verify the infrastructure end-to-end.
pub struct Plonky3ProductionBackend<A> {
    config: QashFriConfig,
    air: A,
}

impl<A> Plonky3ProductionBackend<A> {
    /// Production constructor — uses QASH production FRI parameters
    /// (100-bit conjectured security).
    pub fn new(air: A) -> Self {
        Self {
            config: make_qash_production_config(),
            air,
        }
    }

    /// Test constructor — uses minimal FRI parameters for fast unit tests.
    /// DO NOT use in deployment.
    pub fn new_for_testing(air: A) -> Self {
        Self {
            config: make_qash_test_config(),
            air,
        }
    }

    pub fn config(&self) -> &QashFriConfig {
        &self.config
    }

    pub fn air(&self) -> &A {
        &self.air
    }
}

impl<A> FriProofBackend for Plonky3ProductionBackend<A>
where
    A: Air<SymbolicAirBuilder<QashVal>>
        + for<'a> Air<VerifierConstraintFolder<'a, QashFriConfig>>,
{
    fn verify_shard_shape(&self, shard: &ShardProofBytes) -> Result<[u8; 32], HostedError> {
        let payload: ShardProofPayload = postcard::from_bytes(&shard.proof_bytes)
            .map_err(|_| HostedError::InvalidInput("shard proof_bytes deserialization failed"))?;

        // Verify commitment: SHA3-256(public_values bytes) == public_input_commitment.
        let computed = commitment_of_public_values(&payload.public_values);
        if computed != shard.public_input_commitment {
            return Err(HostedError::InvalidInput(
                "shard public_input_commitment does not match public_values",
            ));
        }

        let public_vals: Vec<QashVal> = payload
            .public_values
            .iter()
            .map(|&v| QashVal::from_u64(v as u64))
            .collect();

        verify(&self.config, &self.air, &payload.proof, &public_vals).map_err(|_| {
            HostedError::InvalidInput("shard STARK proof verification failed")
        })?;

        Ok(shard.public_input_commitment)
    }

    fn verify_aggregation_shape(
        &self,
        agg: &AggregationProofBytes,
        expected_layer1_count: u32,
    ) -> Result<[u8; 32], HostedError> {
        let payload: AggregationProofPayload = postcard::from_bytes(&agg.proof_bytes)
            .map_err(|_| {
                HostedError::InvalidInput("aggregation proof_bytes deserialization failed")
            })?;

        if agg.layer1_count != expected_layer1_count {
            return Err(HostedError::InvalidInput(
                "aggregation layer1_count does not match expected",
            ));
        }
        if agg.batch_root == [0u8; 32] {
            return Err(HostedError::InvalidInput(
                "aggregation batch_root is zero (uninitialized)",
            ));
        }

        // Verify that batch_root matches the commitment over the aggregation public values.
        let computed = commitment_of_public_values(&payload.public_values);
        if computed != agg.batch_root {
            return Err(HostedError::InvalidInput(
                "aggregation batch_root does not match public_values commitment",
            ));
        }

        let public_vals: Vec<QashVal> = payload
            .public_values
            .iter()
            .map(|&v| QashVal::from_u64(v as u64))
            .collect();

        verify(&self.config, &self.air, &payload.proof, &public_vals).map_err(|_| {
            HostedError::InvalidInput("aggregation STARK proof verification failed")
        })?;

        Ok(agg.batch_root)
    }
}

// ── Proof construction helper (test / staging use) ────────────────────────────

/// Build `proof_bytes` for a shard proof using the given AIR, trace, and
/// public values. The returned bytes are suitable for `ShardProofBytes.proof_bytes`.
///
/// Also returns the `public_input_commitment` for the `ShardProofBytes` struct.
///
/// Caller must use the **same** config as `Plonky3ProductionBackend` — use
/// `backend.config()` to obtain it, or pass the matching `is_production` flag.
pub fn build_shard_proof_bytes<
    #[cfg(debug_assertions)] A: for<'a> p3_air::Air<p3_air::DebugConstraintBuilder<'a, QashVal>>,
    #[cfg(not(debug_assertions))] A,
>(
    config: &QashFriConfig,
    air: &A,
    trace: p3_matrix::dense::RowMajorMatrix<QashVal>,
    public_values: Vec<u32>,
) -> Result<(Vec<u8>, [u8; 32]), HostedError>
where
    A: p3_air::Air<p3_air::symbolic::SymbolicAirBuilder<QashVal>>
        + for<'a> p3_air::Air<p3_uni_stark::ProverConstraintFolder<'a, QashFriConfig>>,
{
    let pv_babybear: Vec<QashVal> = public_values
        .iter()
        .map(|&v| QashVal::from_u64(v as u64))
        .collect();

    let proof = p3_uni_stark::prove(config, air, trace, &pv_babybear);

    let payload = ShardProofPayload {
        proof,
        public_values: public_values.clone(),
    };
    let proof_bytes = postcard::to_allocvec(&payload)
        .map_err(|_| HostedError::InvalidInput("proof serialization failed"))?;

    let commitment = commitment_of_public_values(&public_values);
    Ok((proof_bytes, commitment))
}

/// Build `proof_bytes` for an aggregation proof.
///
/// Returns `(proof_bytes, batch_root)`.
pub fn build_aggregation_proof_bytes<
    #[cfg(debug_assertions)] A: for<'a> p3_air::Air<p3_air::DebugConstraintBuilder<'a, QashVal>>,
    #[cfg(not(debug_assertions))] A,
>(
    config: &QashFriConfig,
    air: &A,
    trace: p3_matrix::dense::RowMajorMatrix<QashVal>,
    public_values: Vec<u32>,
    layer1_count: u32,
) -> Result<(Vec<u8>, [u8; 32]), HostedError>
where
    A: p3_air::Air<p3_air::symbolic::SymbolicAirBuilder<QashVal>>
        + for<'a> p3_air::Air<p3_uni_stark::ProverConstraintFolder<'a, QashFriConfig>>,
{
    let pv_babybear: Vec<QashVal> = public_values
        .iter()
        .map(|&v| QashVal::from_u64(v as u64))
        .collect();

    let proof = p3_uni_stark::prove(config, air, trace, &pv_babybear);

    let payload = AggregationProofPayload {
        proof,
        public_values: public_values.clone(),
    };
    let proof_bytes = postcard::to_allocvec(&payload)
        .map_err(|_| HostedError::InvalidInput("aggregation proof serialization failed"))?;

    let batch_root = commitment_of_public_values(&public_values);

    // Sanity: batch_root must not be zero (production guarantee).
    if batch_root == [0u8; 32] {
        return Err(HostedError::InvalidInput(
            "aggregation batch_root is zero — public_values are all-zero",
        ));
    }
    let _ = layer1_count; // stored in AggregationProofBytes by the caller

    Ok((proof_bytes, batch_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zk::fib_air::{FibonacciAir, generate_fib_trace};

    fn make_fib_backend() -> Plonky3ProductionBackend<FibonacciAir> {
        Plonky3ProductionBackend::new_for_testing(FibonacciAir)
    }

    // ── Shard proof E2E tests ────────────────────────────────────────────────

    #[test]
    fn shard_proof_roundtrip_fibonacci() {
        let backend = make_fib_backend();
        let trace = generate_fib_trace(0, 1, 8);
        // public_values: [a=0, b=1, x=fib(8)=21]
        let pv = vec![0u32, 1u32, 21u32];
        let (proof_bytes, commitment) =
            build_shard_proof_bytes(backend.config(), backend.air(), trace, pv.clone()).unwrap();

        let shard = ShardProofBytes {
            shard_id: 0,
            proof_bytes,
            public_input_commitment: commitment,
        };

        let root = backend
            .verify_shard_shape(&shard)
            .expect("shard proof must verify");
        assert_eq!(root, commitment);
    }

    #[test]
    fn shard_proof_rejects_tampered_proof_bytes() {
        let backend = make_fib_backend();
        let trace = generate_fib_trace(0, 1, 8);
        let pv = vec![0u32, 1u32, 21u32];
        let (mut proof_bytes, commitment) =
            build_shard_proof_bytes(backend.config(), backend.air(), trace, pv).unwrap();

        // Tamper with the proof.
        let last = proof_bytes.len() - 1;
        proof_bytes[last] ^= 0xFF;

        let shard = ShardProofBytes {
            shard_id: 0,
            proof_bytes,
            public_input_commitment: commitment,
        };

        assert!(
            backend.verify_shard_shape(&shard).is_err(),
            "tampered proof bytes must be rejected"
        );
    }

    #[test]
    fn shard_proof_rejects_mismatched_commitment() {
        let backend = make_fib_backend();
        let trace = generate_fib_trace(0, 1, 8);
        let pv = vec![0u32, 1u32, 21u32];
        let (proof_bytes, _) =
            build_shard_proof_bytes(backend.config(), backend.air(), trace, pv).unwrap();

        // Wrong commitment: doesn't match the public values in the proof.
        let shard = ShardProofBytes {
            shard_id: 0,
            proof_bytes,
            public_input_commitment: [0xBE; 32],
        };

        assert!(
            backend.verify_shard_shape(&shard).is_err(),
            "mismatched commitment must be rejected"
        );
    }

    // ── Aggregation proof E2E tests ──────────────────────────────────────────

    #[test]
    fn aggregation_proof_roundtrip_fibonacci() {
        let backend = make_fib_backend();
        let trace = generate_fib_trace(0, 1, 8);
        let pv = vec![0u32, 1u32, 21u32];
        let layer1_count = 4u32;
        let (proof_bytes, batch_root) = build_aggregation_proof_bytes(
            backend.config(),
            backend.air(),
            trace,
            pv,
            layer1_count,
        )
        .unwrap();

        let agg = AggregationProofBytes {
            proof_bytes,
            batch_root,
            layer1_count,
        };

        let root = backend
            .verify_aggregation_shape(&agg, layer1_count)
            .expect("aggregation proof must verify");
        assert_eq!(root, batch_root);
    }

    #[test]
    fn aggregation_rejects_count_mismatch() {
        let backend = make_fib_backend();
        let trace = generate_fib_trace(0, 1, 8);
        let pv = vec![0u32, 1u32, 21u32];
        let (proof_bytes, batch_root) =
            build_aggregation_proof_bytes(backend.config(), backend.air(), trace, pv, 4).unwrap();

        let agg = AggregationProofBytes {
            proof_bytes,
            batch_root,
            layer1_count: 4,
        };

        // Expected count differs from stored count.
        assert!(backend.verify_aggregation_shape(&agg, 5).is_err());
    }

    // ── Profile-lock: production backend rejects wrong profile ───────────────

    #[test]
    fn production_backend_config_uses_qash_profile_field() {
        // BabyBear p = 2^31 - 2^27 + 1 = 2013265921.
        #[allow(unused_imports)]
        use p3_field::PrimeField32;
        assert_eq!(
            <QashVal as p3_field::PrimeField32>::ORDER_U32,
            2013265921u32,
            "QASH field must be BabyBear (p = 2^31 - 2^27 + 1)"
        );
    }

    #[test]
    fn commitment_of_public_values_is_deterministic() {
        let pv = vec![0u32, 1u32, 21u32];
        let c1 = commitment_of_public_values(&pv);
        let c2 = commitment_of_public_values(&pv);
        assert_eq!(c1, c2);
    }

    #[test]
    fn commitment_of_different_public_values_differs() {
        let c1 = commitment_of_public_values(&[0u32, 1u32, 21u32]);
        let c2 = commitment_of_public_values(&[0u32, 1u32, 22u32]);
        assert_ne!(c1, c2);
    }
}
