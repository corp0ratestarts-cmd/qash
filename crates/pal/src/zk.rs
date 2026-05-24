//! Domain B ZK verifier trait and profile lock (Stage 3).
//!
//! Proof bytes are Domain B-only. Domain A never receives proof bytes — only
//! a validated (profile_id, zk_batch_root) pair crosses the A/B boundary.
//!
//! # Profile lock
//!
//! Only `ZkProfileId::Plonky3FriPoseidonQash` is currently admitted. All other
//! profile IDs are rejected. This ensures that a future protocol upgrade (which
//! would change Domain A's `validate_zk_profile` constant) is always gated
//! through a named, auditable variant.
//!
//! # Proof-byte non-entry guard
//!
//! The `ZkVerifier::verify` return type is `Result<ZkBatchRoot, ZkVerifyError>`.
//! `ZkBatchRoot` is a newtype over `[u8; 32]` — a fixed-width commitment root, not
//! a proof blob. Proof bytes are consumed inside the verifier and never returned.

/// The sole proof profile currently supported by the Domain B ZK verifier.
///
/// Matches `ZkProfile::PLONKY3_FRI_POSEIDON_QASH` in Domain A's sharding module.
/// Do not add variants without a corresponding Domain A profile constant and Coq proof obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZkProfileId {
    Plonky3FriPoseidonQash,
}

impl ZkProfileId {
    /// Numeric profile ID, matching Domain A's `ZK_PROFILE_ID_PLONKY3_FRI_POSEIDON_QASH`.
    pub const fn profile_id(self) -> u32 {
        match self {
            ZkProfileId::Plonky3FriPoseidonQash => 0x0001_0001,
        }
    }

    /// Recursion depth, matching Domain A's `ZK_RECURSION_DEPTH`.
    pub const fn recursion_depth(self) -> u8 {
        match self {
            ZkProfileId::Plonky3FriPoseidonQash => 2,
        }
    }

    /// Layer-1 aggregation factor, matching Domain A's `ZK_LAYER1_AGGREGATION_FACTOR`.
    pub const fn layer1_aggregation_factor(self) -> u16 {
        match self {
            ZkProfileId::Plonky3FriPoseidonQash => 16,
        }
    }

    /// Try to construct a `ZkProfileId` from a raw numeric profile_id.
    ///
    /// Returns `Err(ZkVerifyError::UnknownProfile)` for any unrecognised value.
    pub fn from_profile_id(id: u32) -> Result<Self, ZkVerifyError> {
        match id {
            0x0001_0001 => Ok(ZkProfileId::Plonky3FriPoseidonQash),
            _ => Err(ZkVerifyError::UnknownProfile(id)),
        }
    }
}

/// A verified 32-byte ZK batch root — the sole artifact from proof verification
/// that may cross into Domain A.
///
/// Proof bytes are never returned. `ZkBatchRoot` is a fixed-width commitment only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZkBatchRoot(pub [u8; 32]);

impl ZkBatchRoot {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A proof bundle presented to the Domain B verifier.
///
/// `proof_bytes` stays in Domain B — it is consumed by the verifier and
/// must never be forwarded to Domain A.
#[derive(Debug, Clone)]
pub struct ZkProofBundle {
    /// The declared profile for this proof. Must match the profile lock.
    pub profile_id: ZkProfileId,
    /// Proof bytes (STARK proof, aggregation proof, etc.). Domain B only.
    pub proof_bytes: Vec<u8>,
    /// Number of shard proofs in the bundle.
    pub shard_proof_count: u32,
    /// Number of aggregation proofs.
    pub aggregation_proof_count: u32,
    /// Claimed batch root. The verifier must confirm this matches the proof.
    pub claimed_batch_root: [u8; 32],
}

/// Errors from ZK proof verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZkVerifyError {
    /// Profile ID is not in the admitted set.
    UnknownProfile(u32),
    /// Profile lock mismatch: the bundle's profile does not match the verifier's lock.
    ProfileMismatch,
    /// Proof bytes are empty — no proof provided.
    EmptyProof,
    /// Bundle has zero shard proofs.
    ZeroShardProofs,
    /// Bundle has zero aggregation proofs.
    ZeroAggregationProofs,
    /// The claimed batch root does not match what the proof demonstrates.
    BatchRootMismatch,
    /// Proof recursion depth is not 2 (the locked depth).
    WrongRecursionDepth,
    /// Aggregation factor is not 16 (the locked factor).
    WrongAggregationFactor,
    /// Internal verifier error (backend-specific).
    BackendError,
}

/// Domain B ZK verifier trait.
///
/// Implementors receive a `ZkProofBundle` and return either a verified
/// `ZkBatchRoot` (32 bytes — safe to cross into Domain A) or an error.
///
/// INVARIANT: proof bytes MUST NOT appear in the return type.
pub trait ZkVerifier {
    fn verify(&self, bundle: &ZkProofBundle) -> Result<ZkBatchRoot, ZkVerifyError>;
}

/// A static verifier that trusts exactly one (profile, claimed_batch_root) pair.
///
/// Used in tests and integration stubs. Must not be deployed with real proof
/// bytes — it does not actually verify STARK proofs.
pub struct StaticZkVerifier {
    pub accepted_profile: ZkProfileId,
    pub accepted_batch_root: [u8; 32],
}

impl ZkVerifier for StaticZkVerifier {
    fn verify(&self, bundle: &ZkProofBundle) -> Result<ZkBatchRoot, ZkVerifyError> {
        if bundle.profile_id != self.accepted_profile {
            return Err(ZkVerifyError::ProfileMismatch);
        }
        if bundle.proof_bytes.is_empty() {
            return Err(ZkVerifyError::EmptyProof);
        }
        if bundle.shard_proof_count == 0 {
            return Err(ZkVerifyError::ZeroShardProofs);
        }
        if bundle.aggregation_proof_count == 0 {
            return Err(ZkVerifyError::ZeroAggregationProofs);
        }
        if bundle.claimed_batch_root != self.accepted_batch_root {
            return Err(ZkVerifyError::BatchRootMismatch);
        }
        // Proof bytes are consumed (checked non-empty above) and not returned.
        Ok(ZkBatchRoot(bundle.claimed_batch_root))
    }
}

/// A verifier that rejects all proofs — safe default for unconfigured backends.
pub struct RejectAllZkVerifier;

impl ZkVerifier for RejectAllZkVerifier {
    fn verify(&self, _bundle: &ZkProofBundle) -> Result<ZkBatchRoot, ZkVerifyError> {
        Err(ZkVerifyError::BackendError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_bundle(root: [u8; 32]) -> ZkProofBundle {
        ZkProofBundle {
            profile_id: ZkProfileId::Plonky3FriPoseidonQash,
            proof_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
            shard_proof_count: 4,
            aggregation_proof_count: 1,
            claimed_batch_root: root,
        }
    }

    fn verifier(root: [u8; 32]) -> StaticZkVerifier {
        StaticZkVerifier {
            accepted_profile: ZkProfileId::Plonky3FriPoseidonQash,
            accepted_batch_root: root,
        }
    }

    #[test]
    fn valid_bundle_produces_batch_root_not_proof_bytes() {
        let root = [0xABu8; 32];
        let v = verifier(root);
        let result = v.verify(&good_bundle(root)).unwrap();
        // The result is only the 32-byte root — proof bytes are NOT returned.
        assert_eq!(result.as_bytes(), &root);
        assert_eq!(result.as_bytes().len(), 32);
    }

    #[test]
    fn profile_mismatch_rejected() {
        let root = [0x01u8; 32];
        let v = StaticZkVerifier {
            accepted_profile: ZkProfileId::Plonky3FriPoseidonQash,
            accepted_batch_root: root,
        };
        // A bundle claiming a different profile_id (not currently representable
        // without a second variant, so we test via from_profile_id failure).
        let err = ZkProfileId::from_profile_id(0xDEAD_BEEF).unwrap_err();
        assert_eq!(err, ZkVerifyError::UnknownProfile(0xDEAD_BEEF));
    }

    #[test]
    fn empty_proof_bytes_rejected() {
        let root = [0x02u8; 32];
        let v = verifier(root);
        let mut bundle = good_bundle(root);
        bundle.proof_bytes.clear();
        assert_eq!(v.verify(&bundle).unwrap_err(), ZkVerifyError::EmptyProof);
    }

    #[test]
    fn zero_shard_proofs_rejected() {
        let root = [0x03u8; 32];
        let v = verifier(root);
        let mut bundle = good_bundle(root);
        bundle.shard_proof_count = 0;
        assert_eq!(v.verify(&bundle).unwrap_err(), ZkVerifyError::ZeroShardProofs);
    }

    #[test]
    fn zero_aggregation_proofs_rejected() {
        let root = [0x04u8; 32];
        let v = verifier(root);
        let mut bundle = good_bundle(root);
        bundle.aggregation_proof_count = 0;
        assert_eq!(v.verify(&bundle).unwrap_err(), ZkVerifyError::ZeroAggregationProofs);
    }

    #[test]
    fn wrong_batch_root_rejected() {
        let root = [0x05u8; 32];
        let v = verifier(root);
        let mut bundle = good_bundle(root);
        bundle.claimed_batch_root = [0x06u8; 32];
        assert_eq!(v.verify(&bundle).unwrap_err(), ZkVerifyError::BatchRootMismatch);
    }

    #[test]
    fn reject_all_verifier_always_errors() {
        let bundle = good_bundle([0xFF; 32]);
        assert_eq!(RejectAllZkVerifier.verify(&bundle).unwrap_err(), ZkVerifyError::BackendError);
    }

    #[test]
    fn profile_id_constants_match_domain_a_values() {
        // ZkProfileId must stay in sync with Domain A's sharding constants.
        // If these change, both Domain A and Domain B must be updated together.
        assert_eq!(ZkProfileId::Plonky3FriPoseidonQash.profile_id(), 0x0001_0001u32);
        assert_eq!(ZkProfileId::Plonky3FriPoseidonQash.recursion_depth(), 2u8);
        assert_eq!(ZkProfileId::Plonky3FriPoseidonQash.layer1_aggregation_factor(), 16u16);
    }

    #[test]
    fn zk_batch_root_is_32_bytes_not_proof_blob() {
        // Structural test: ZkBatchRoot is a [u8; 32] newtype.
        // If someone adds a proof_bytes field to ZkBatchRoot, this destructuring will fail.
        let r = ZkBatchRoot([0xAAu8; 32]);
        let ZkBatchRoot(bytes) = r;
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn proof_bytes_do_not_appear_in_verify_return() {
        // Verifying a bundle with distinctive proof bytes should produce a result
        // that has no way to reconstruct those proof bytes.
        let root = [0x77u8; 32];
        let v = verifier(root);
        let mut bundle = good_bundle(root);
        bundle.proof_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03];
        let result = v.verify(&bundle).unwrap();
        // Result is only ZkBatchRoot([u8; 32]) — no Vec<u8>, no proof blob.
        assert_eq!(*result.as_bytes(), root);
    }
}
