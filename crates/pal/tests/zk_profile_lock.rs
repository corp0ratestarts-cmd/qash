//! Stage 3b/3c — ZK profile lock and proof-byte non-entry guard tests.
//!
//! These tests exercise the Domain B ZK verifier boundary:
//! - Only admitted profile IDs pass
//! - Malformed/empty/wrong proofs are rejected
//! - The return type contains only a 32-byte root — never proof bytes

use qash_pal::zk::{
    RejectAllZkVerifier, StaticZkVerifier, ZkBatchRoot, ZkProfileId, ZkProofBundle, ZkVerifier,
    ZkVerifyError,
};

fn bundle(root: [u8; 32]) -> ZkProofBundle {
    ZkProofBundle {
        profile_id: ZkProfileId::Plonky3FriPoseidonQash,
        proof_bytes: vec![0x01, 0x02, 0x03, 0x04],
        shard_proof_count: 2,
        aggregation_proof_count: 1,
        claimed_batch_root: root,
    }
}

fn static_verifier(root: [u8; 32]) -> StaticZkVerifier {
    StaticZkVerifier {
        accepted_profile: ZkProfileId::Plonky3FriPoseidonQash,
        accepted_batch_root: root,
    }
}

// ---------------------------------------------------------------------------
// Profile lock tests
// ---------------------------------------------------------------------------

#[test]
fn unknown_profile_id_is_rejected() {
    let err = ZkProfileId::from_profile_id(0xFFFF_FFFF).unwrap_err();
    assert_eq!(err, ZkVerifyError::UnknownProfile(0xFFFF_FFFF));
}

#[test]
fn known_profile_id_round_trips() {
    let pid = ZkProfileId::from_profile_id(0x0001_0001).unwrap();
    assert_eq!(pid, ZkProfileId::Plonky3FriPoseidonQash);
    assert_eq!(pid.profile_id(), 0x0001_0001);
}

#[test]
fn profile_constants_are_locked_to_domain_a_values() {
    assert_eq!(
        ZkProfileId::Plonky3FriPoseidonQash.profile_id(),
        0x0001_0001
    );
    assert_eq!(ZkProfileId::Plonky3FriPoseidonQash.recursion_depth(), 2);
    assert_eq!(
        ZkProfileId::Plonky3FriPoseidonQash.layer1_aggregation_factor(),
        16
    );
}

// ---------------------------------------------------------------------------
// Malformed proof rejection tests (Stage 3b)
// ---------------------------------------------------------------------------

#[test]
fn empty_proof_rejected() {
    let root = [0x10u8; 32];
    let v = static_verifier(root);
    let mut b = bundle(root);
    b.proof_bytes.clear();
    assert_eq!(v.verify(&b).unwrap_err(), ZkVerifyError::EmptyProof);
}

#[test]
fn wrong_batch_root_rejected() {
    let root = [0x20u8; 32];
    let v = static_verifier(root);
    let mut b = bundle(root);
    b.claimed_batch_root = [0x21u8; 32];
    assert_eq!(v.verify(&b).unwrap_err(), ZkVerifyError::BatchRootMismatch);
}

#[test]
fn zero_shard_count_rejected() {
    let root = [0x30u8; 32];
    let v = static_verifier(root);
    let mut b = bundle(root);
    b.shard_proof_count = 0;
    assert_eq!(v.verify(&b).unwrap_err(), ZkVerifyError::ZeroShardProofs);
}

#[test]
fn zero_aggregation_count_rejected() {
    let root = [0x40u8; 32];
    let v = static_verifier(root);
    let mut b = bundle(root);
    b.aggregation_proof_count = 0;
    assert_eq!(
        v.verify(&b).unwrap_err(),
        ZkVerifyError::ZeroAggregationProofs
    );
}

// ---------------------------------------------------------------------------
// Proof-byte non-entry guard (Stage 3c)
// ---------------------------------------------------------------------------

#[test]
fn verify_return_type_is_32_byte_root_only() {
    let root = [0x50u8; 32];
    let v = static_verifier(root);
    let b = bundle(root);
    let result: ZkBatchRoot = v.verify(&b).unwrap();
    // ZkBatchRoot is a newtype over [u8; 32].
    // Destructure to prove it contains only 32 bytes — no proof blob.
    let ZkBatchRoot(bytes) = result;
    assert_eq!(bytes, root);
    assert_eq!(bytes.len(), 32);
}

#[test]
fn large_proof_bytes_do_not_appear_in_result() {
    let root = [0x60u8; 32];
    let v = static_verifier(root);
    let mut b = bundle(root);
    // 1 MB of proof bytes — these must not appear in the output.
    b.proof_bytes = vec![0xFFu8; 1_000_000];
    let result = v.verify(&b).unwrap();
    // Result is 32 bytes exactly — no proof material.
    assert_eq!(result.as_bytes().len(), 32);
    assert_eq!(*result.as_bytes(), root);
}

#[test]
fn reject_all_verifier_blocks_all_bundles() {
    let b = bundle([0x70u8; 32]);
    assert_eq!(
        RejectAllZkVerifier.verify(&b).unwrap_err(),
        ZkVerifyError::BackendError
    );
}
