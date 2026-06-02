/// Plonky3 FRI-STARK verifier backend for the QASH two-layer recursion profile.
///
/// # Domain boundary
///
/// This module is Domain B. It processes raw proof bytes, circuit witnesses,
/// and Poseidon hashes. The only value returned to callers is a 32-byte
/// `batch_root` extracted after successful verification. That root is the sole
/// output permitted to cross into Domain A via `ZkProofBundle.batch_root`.
///
/// Raw proof bytes, transcript state, commitment polynomials, and STARK
/// intermediate values MUST NOT cross the Domain A boundary — they stay here.
///
/// # Two-layer recursion profile
///
/// Layer 1: per-shard FRI-STARK proofs, each committing to one shard's state
/// transition. Poseidon is used as the algebraic hash inside the circuit.
///
/// Layer 2: a single aggregation proof that verifies all layer-1 proofs
/// recursively and computes the cross-shard `batch_root` as a folded
/// Poseidon hash over the layer-1 batch roots.
///
/// The profile is locked at compile time via `Plonky3FriVerifier::new()`.
/// Changing any field of the locked profile causes the shape-only harness to
/// return `Err(HostedError::InvalidInput("profile mismatch"))` — this is the
/// profile-lock invariant verified by the test suite.
///
/// # Real backend status
///
/// `plonky3.rs` defines Domain B interface types and the non-production shape harness.
/// `backend.rs` contains the real p3-* verifier infrastructure behind the `plonky3` feature.
/// QASH production proof circuit acceptance: post-v1 / not deployment-authoritative.
use crate::hosted::{CanonicalZkProfile, HostedError, ZkProofBundle, ZkProofVerifier};

/// Proof bytes for a single shard's layer-1 FRI-STARK proof.
///
/// Opaque from Domain A's perspective; fully processed within Domain B.
#[derive(Debug, Clone)]
pub struct ShardProofBytes {
    pub shard_id: u32,
    /// Raw STARK proof bytes (FRI opening + merkle paths + round polynomials).
    pub proof_bytes: Vec<u8>,
    /// Poseidon commitment to the shard's public inputs (state_root, receipt_root).
    pub public_input_commitment: [u8; 32],
}

/// Proof bytes for the layer-2 aggregation STARK.
///
/// Aggregates all layer-1 shard proofs; emits the cross-shard batch_root.
#[derive(Debug, Clone)]
pub struct AggregationProofBytes {
    /// Raw STARK aggregation proof bytes.
    pub proof_bytes: Vec<u8>,
    /// Poseidon hash over sorted layer-1 batch roots; becomes `ZkProofBundle.batch_root`.
    pub batch_root: [u8; 32],
    /// Count of layer-1 proofs covered by this aggregation.
    pub layer1_count: u32,
}

/// Backend boundary for the eventual Plonky3 verifier.
///
/// The default implementation is intentionally non-production: it validates
/// byte presence and public-shape fields for tests, but it is not used by the
/// `ZkProofVerifier` trait path.
pub trait FriProofBackend {
    fn verify_shard_shape(&self, shard: &ShardProofBytes) -> Result<[u8; 32], HostedError>;

    fn verify_aggregation_shape(
        &self,
        agg: &AggregationProofBytes,
        expected_layer1_count: u32,
    ) -> Result<[u8; 32], HostedError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NonProductionShapeBackend;

impl FriProofBackend for NonProductionShapeBackend {
    fn verify_shard_shape(&self, shard: &ShardProofBytes) -> Result<[u8; 32], HostedError> {
        validate_proof_bytes_present(&shard.proof_bytes, "shard")?;
        if shard.public_input_commitment == [0u8; 32] {
            return Err(HostedError::InvalidInput(
                "shard public input commitment is zero (uninitialized)",
            ));
        }
        Ok(shard.public_input_commitment)
    }

    fn verify_aggregation_shape(
        &self,
        agg: &AggregationProofBytes,
        expected_layer1_count: u32,
    ) -> Result<[u8; 32], HostedError> {
        validate_proof_bytes_present(&agg.proof_bytes, "aggregation")?;
        if agg.layer1_count != expected_layer1_count {
            return Err(HostedError::InvalidInput(
                "aggregation layer1_count does not match shard proof count",
            ));
        }
        if agg.batch_root == [0u8; 32] {
            return Err(HostedError::InvalidInput(
                "aggregation batch_root is zero (uninitialized)",
            ));
        }
        Ok(agg.batch_root)
    }
}

/// Plonky3 FRI-STARK verifier implementing `ZkProofVerifier`.
///
/// The profile is locked at construction. Until a real Plonky3 backend is
/// wired, the `ZkProofVerifier` trait path fails closed for every bundle.
#[derive(Debug, Clone)]
pub struct Plonky3FriVerifier<B = NonProductionShapeBackend> {
    locked_profile: CanonicalZkProfile,
    backend: B,
}

impl Plonky3FriVerifier<NonProductionShapeBackend> {
    /// Build a verifier locked to the PLONKY3_FRI_POSEIDON_QASH profile.
    ///
    /// This constructor uses the non-production shape backend. The profile
    /// cannot be changed after construction, but the trait verifier still
    /// fails closed until real proof verification is implemented.
    pub fn new() -> Self {
        Plonky3FriVerifier {
            locked_profile: CanonicalZkProfile::pr93_plonky3_fri_poseidon_qash(),
            backend: NonProductionShapeBackend,
        }
    }
}

impl<B: FriProofBackend> Plonky3FriVerifier<B> {
    /// Locked profile accessor (read-only).
    pub fn locked_profile(&self) -> &CanonicalZkProfile {
        &self.locked_profile
    }

    pub fn with_backend(backend: B) -> Self {
        Self {
            locked_profile: CanonicalZkProfile::pr93_plonky3_fri_poseidon_qash(),
            backend,
        }
    }

    pub fn verify_bundle_shape_only(
        &self,
        bundle: &ZkProofBundle,
    ) -> Result<[u8; 32], HostedError> {
        validate_bundle_shape(bundle, &self.locked_profile)?;
        Ok(bundle.batch_root)
    }

    /// Verify a single layer-1 shard proof.
    ///
    /// Checks:
    /// 1. `proof_bytes` is non-empty (empty = malformed).
    /// 2. `public_input_commitment` is non-zero (zero = uninitialized).
    ///
    /// Returns the shard's `public_input_commitment` on success.
    ///
    /// This is a non-production shape check until the real Plonky3 backend is
    /// vetted and added.
    pub fn verify_shard_proof(&self, shard: &ShardProofBytes) -> Result<[u8; 32], HostedError> {
        self.backend.verify_shard_shape(shard)
    }

    /// Verify the layer-2 aggregation proof.
    ///
    /// Checks:
    /// 1. `proof_bytes` is non-empty.
    /// 2. `layer1_count` matches the number of layer-1 proofs provided.
    /// 3. `batch_root` is non-zero.
    ///
    /// Returns the aggregation's `batch_root` on success.
    pub fn verify_aggregation_proof(
        &self,
        agg: &AggregationProofBytes,
        expected_layer1_count: u32,
    ) -> Result<[u8; 32], HostedError> {
        self.backend
            .verify_aggregation_shape(agg, expected_layer1_count)
    }
}

impl Default for Plonky3FriVerifier<NonProductionShapeBackend> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B: FriProofBackend> ZkProofVerifier for Plonky3FriVerifier<B> {
    /// Verify a `ZkProofBundle` against the locked profile.
    ///
    /// Steps:
    /// The production trait path fails closed because `ZkProofBundle` carries
    /// only profile/count/root metadata, not the raw proof bytes needed for
    /// real FRI-STARK verification. Once the vetted Plonky3 backend is added,
    /// this method must verify actual proof material before returning the root.
    fn verify_bundle(&self, bundle: &ZkProofBundle) -> Result<[u8; 32], HostedError> {
        validate_bundle_shape(bundle, &self.locked_profile)?;
        Err(HostedError::InvalidInput(
            "production Plonky3 verifier backend is not enabled",
        ))
    }
}

fn validate_bundle_shape(
    bundle: &ZkProofBundle,
    locked_profile: &CanonicalZkProfile,
) -> Result<(), HostedError> {
    if bundle.profile != *locked_profile {
        return Err(HostedError::InvalidInput(
            "profile mismatch: bundle profile does not match locked profile",
        ));
    }
    if bundle.shard_proof_count == 0 {
        return Err(HostedError::InvalidInput(
            "malformed bundle: shard_proof_count is zero",
        ));
    }
    if bundle.aggregation_proof_count == 0 {
        return Err(HostedError::InvalidInput(
            "malformed bundle: aggregation_proof_count is zero",
        ));
    }
    if bundle.batch_root == [0u8; 32] {
        return Err(HostedError::InvalidInput(
            "malformed bundle: batch_root is zero (uninitialized)",
        ));
    }
    Ok(())
}

fn validate_proof_bytes_present(bytes: &[u8], kind: &'static str) -> Result<(), HostedError> {
    if bytes.is_empty() {
        return Err(HostedError::InvalidInput(match kind {
            "shard" => "shard proof bytes are empty",
            "aggregation" => "aggregation proof bytes are empty",
            _ => "proof bytes are empty",
        }));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hosted::CanonicalZkProfile;

    fn locked_profile() -> CanonicalZkProfile {
        CanonicalZkProfile::pr93_plonky3_fri_poseidon_qash()
    }

    fn valid_bundle() -> ZkProofBundle {
        ZkProofBundle {
            profile: locked_profile(),
            shard_proof_count: 4,
            aggregation_proof_count: 1,
            batch_root: [0xAB; 32],
        }
    }

    // ── Profile-lock tests ──────────────────────────────────────────────────

    #[test]
    fn production_trait_path_fails_closed_until_real_backend_is_enabled() {
        let v = Plonky3FriVerifier::new();
        let bundle = valid_bundle();
        let result = v.verify_bundle(&bundle);
        assert!(
            matches!(result, Err(HostedError::InvalidInput(message)) if message.contains("not enabled"))
        );
    }

    #[test]
    fn profile_lock_shape_harness_accepts_matching_profile() {
        let v = Plonky3FriVerifier::new();
        let bundle = valid_bundle();
        assert!(v.verify_bundle_shape_only(&bundle).is_ok());
    }

    #[test]
    fn profile_lock_rejects_mutated_profile_id() {
        let v = Plonky3FriVerifier::new();
        let mut bundle = valid_bundle();
        bundle.profile.profile_id = bundle.profile.profile_id.wrapping_add(1);
        let result = v.verify_bundle_shape_only(&bundle);
        assert!(result.is_err(), "mutated profile_id must be rejected");
        let msg = match result {
            Err(HostedError::InvalidInput(m)) => m,
            _ => panic!("expected InvalidInput"),
        };
        assert!(
            msg.contains("profile mismatch"),
            "error must mention profile mismatch"
        );
    }

    #[test]
    fn profile_lock_rejects_mutated_recursion_depth() {
        let v = Plonky3FriVerifier::new();
        let mut bundle = valid_bundle();
        bundle.profile.recursion_depth = bundle.profile.recursion_depth.wrapping_add(1);
        assert!(
            v.verify_bundle_shape_only(&bundle).is_err(),
            "mutated recursion_depth must be rejected"
        );
    }

    #[test]
    fn profile_lock_rejects_mutated_aggregation_factor() {
        let v = Plonky3FriVerifier::new();
        let mut bundle = valid_bundle();
        bundle.profile.layer1_aggregation_factor =
            bundle.profile.layer1_aggregation_factor.wrapping_add(1);
        assert!(
            v.verify_bundle_shape_only(&bundle).is_err(),
            "mutated layer1_aggregation_factor must be rejected"
        );
    }

    // ── Malformed-proof rejection tests ─────────────────────────────────────

    #[test]
    fn rejects_zero_shard_proof_count() {
        let v = Plonky3FriVerifier::new();
        let mut bundle = valid_bundle();
        bundle.shard_proof_count = 0;
        let result = v.verify_bundle_shape_only(&bundle);
        assert!(result.is_err());
        let msg = match result {
            Err(HostedError::InvalidInput(m)) => m,
            _ => panic!("expected InvalidInput"),
        };
        assert!(msg.contains("shard_proof_count"));
    }

    #[test]
    fn rejects_zero_aggregation_proof_count() {
        let v = Plonky3FriVerifier::new();
        let mut bundle = valid_bundle();
        bundle.aggregation_proof_count = 0;
        let result = v.verify_bundle_shape_only(&bundle);
        assert!(result.is_err());
        let msg = match result {
            Err(HostedError::InvalidInput(m)) => m,
            _ => panic!("expected InvalidInput"),
        };
        assert!(msg.contains("aggregation_proof_count"));
    }

    #[test]
    fn rejects_zero_batch_root() {
        let v = Plonky3FriVerifier::new();
        let mut bundle = valid_bundle();
        bundle.batch_root = [0u8; 32];
        assert!(
            v.verify_bundle_shape_only(&bundle).is_err(),
            "zero batch_root must be rejected as uninitialized"
        );
    }

    #[test]
    fn shard_proof_rejects_empty_bytes() {
        let v = Plonky3FriVerifier::new();
        let shard = ShardProofBytes {
            shard_id: 0,
            proof_bytes: vec![],
            public_input_commitment: [0xCD; 32],
        };
        assert!(
            v.verify_shard_proof(&shard).is_err(),
            "empty shard proof bytes must be rejected"
        );
    }

    #[test]
    fn shard_proof_rejects_zero_commitment() {
        let v = Plonky3FriVerifier::new();
        let shard = ShardProofBytes {
            shard_id: 0,
            proof_bytes: vec![0x01, 0x02, 0x03],
            public_input_commitment: [0u8; 32],
        };
        assert!(
            v.verify_shard_proof(&shard).is_err(),
            "zero public_input_commitment must be rejected as uninitialized"
        );
    }

    #[test]
    fn aggregation_proof_rejects_empty_bytes() {
        let v = Plonky3FriVerifier::new();
        let agg = AggregationProofBytes {
            proof_bytes: vec![],
            batch_root: [0xAB; 32],
            layer1_count: 4,
        };
        assert!(
            v.verify_aggregation_proof(&agg, 4).is_err(),
            "empty aggregation proof bytes must be rejected"
        );
    }

    #[test]
    fn aggregation_proof_rejects_layer1_count_mismatch() {
        let v = Plonky3FriVerifier::new();
        let agg = AggregationProofBytes {
            proof_bytes: vec![0x01; 64],
            batch_root: [0xAB; 32],
            layer1_count: 4,
        };
        assert!(
            v.verify_aggregation_proof(&agg, 5).is_err(),
            "layer1_count mismatch must be rejected"
        );
    }

    #[test]
    fn aggregation_proof_rejects_zero_batch_root() {
        let v = Plonky3FriVerifier::new();
        let agg = AggregationProofBytes {
            proof_bytes: vec![0x01; 64],
            batch_root: [0u8; 32],
            layer1_count: 4,
        };
        assert!(
            v.verify_aggregation_proof(&agg, 4).is_err(),
            "zero batch_root must be rejected"
        );
    }

    // ── Domain B boundary test ───────────────────────────────────────────────

    /// Verify that the shape harness returns only a 32-byte root — no proof
    /// bytes, no STARK transcript state, no Domain B internals cross over.
    ///
    /// This test enforces the Domain B containment invariant at the type level:
    /// the return type remains `Result<[u8;32], _>`.
    /// Proof bytes (`ShardProofBytes`, `AggregationProofBytes`) have no path
    /// into Domain A from this return type.
    #[test]
    fn proof_bytes_stay_in_domain_b_return_type_is_batch_root_only() {
        let v = Plonky3FriVerifier::new();
        let bundle = valid_bundle();
        let result: Result<[u8; 32], HostedError> = v.verify_bundle_shape_only(&bundle);
        let root = result.expect("valid bundle must verify");
        // The 32-byte root is all that crossed the boundary.
        assert_eq!(root, bundle.batch_root);
        // Compile-time: if this boundary returned proof bytes,
        // this assignment would not compile — the type would be wrong.
    }

    /// `ShardProofBytes` and `AggregationProofBytes` are not exported through
    /// the hosted root boundary. Verify the locked profile is the only
    /// default configuration pathway.
    #[test]
    fn verifier_locked_profile_is_read_only() {
        let v = Plonky3FriVerifier::new();
        let profile = v.locked_profile();
        assert_eq!(*profile, locked_profile());
    }

    // ── Two-layer pipeline integration test ─────────────────────────────────

    #[test]
    fn two_layer_pipeline_succeeds_on_valid_inputs() {
        let v = Plonky3FriVerifier::new();

        let shards: Vec<ShardProofBytes> = (0..4)
            .map(|i| ShardProofBytes {
                shard_id: i,
                proof_bytes: vec![0x01 + i as u8; 128],
                public_input_commitment: {
                    let mut c = [0u8; 32];
                    c[0] = i as u8 + 1;
                    c
                },
            })
            .collect();

        for shard in &shards {
            v.verify_shard_proof(shard)
                .expect("shard proof must verify");
        }

        let agg = AggregationProofBytes {
            proof_bytes: vec![0xFF; 256],
            batch_root: [0xAB; 32],
            layer1_count: shards.len() as u32,
        };
        let batch_root = v
            .verify_aggregation_proof(&agg, shards.len() as u32)
            .expect("aggregation proof must verify");

        let bundle = ZkProofBundle {
            profile: locked_profile(),
            shard_proof_count: shards.len() as u32,
            aggregation_proof_count: 1,
            batch_root,
        };
        let result = v
            .verify_bundle_shape_only(&bundle)
            .expect("bundle shape must verify");
        assert_eq!(result, batch_root);
    }
}
