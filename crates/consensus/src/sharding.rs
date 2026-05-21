//! Deterministic sharded execution commitments.
//!
//! This module is Domain A only: fixed-width public commitments, deterministic
//! shard assignment, cross-shard receipt identifiers, and Epoch Finality Beacon
//! aggregation.  It does not execute transactions or inspect private Domain B
//! payloads.

use crate::hash::{h_domain, DomainTag};

pub const MAX_SHARDS_WIRE: u32 = 1024;
pub const MAX_SHARDS: usize = MAX_SHARDS_WIRE as usize;
pub const ZK_PROFILE_ID_PLONKY3_FRI_POSEIDON_QASH: u32 = 0x0001_0001;
pub const ZK_RECURSION_DEPTH: u8 = 2;
pub const ZK_LAYER1_AGGREGATION_FACTOR: u16 = 16;
pub const ZK_LAYER0_SHARD_VALIDITY: u8 = 0;
pub const ZK_LAYER1_AGGREGATION: u8 = 1;
pub const ZK_LAYER2_EFB: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardingError {
    ZeroShardCount,
    ShardCountTooLarge,
    ShardOutOfRange,
    InvalidShardCount,
    DuplicateShard,
    ShardsNotSorted,
    ReceiptEpochMismatch,
    ReceiptShardMismatch,
    InvalidMerkleProof,
    ReceiptNotIncluded,
    InvalidZkProfile,
    InvalidZkRecursionDepth,
    InvalidZkAggregationFactor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZkProfile {
    pub profile_id: u32,
    pub recursion_depth: u8,
    pub layer1_aggregation_factor: u16,
}

impl ZkProfile {
    pub const PLONKY3_FRI_POSEIDON_QASH: Self = Self {
        profile_id: ZK_PROFILE_ID_PLONKY3_FRI_POSEIDON_QASH,
        recursion_depth: ZK_RECURSION_DEPTH,
        layer1_aggregation_factor: ZK_LAYER1_AGGREGATION_FACTOR,
    };

    pub fn validate(&self) -> Result<(), ShardingError> {
        if self.profile_id != ZK_PROFILE_ID_PLONKY3_FRI_POSEIDON_QASH {
            return Err(ShardingError::InvalidZkProfile);
        }
        if self.recursion_depth != ZK_RECURSION_DEPTH {
            return Err(ShardingError::InvalidZkRecursionDepth);
        }
        if self.layer1_aggregation_factor != ZK_LAYER1_AGGREGATION_FACTOR {
            return Err(ShardingError::InvalidZkAggregationFactor);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardCommitment {
    pub shard_id: u32,
    pub state_root: [u8; 32],
    pub receipt_root: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossShardReceipt {
    pub epoch: u64,
    pub source_shard: u32,
    pub target_shard: u32,
    pub nonce: u64,
    pub payload_hash: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochFinalityBeacon {
    pub epoch: u64,
    pub previous_efb_root: [u8; 32],
    pub shard_count: u32,
    pub aggregate_state_root: [u8; 32],
    pub aggregate_receipt_root: [u8; 32],
    pub zk_batch_root: [u8; 32],
    pub efb_root: [u8; 32],
}

/// Validate the provisional PR #93 ZK profile.
///
/// Domain A does not verify STARK proofs here. It only fixes the deterministic
/// public profile that a Domain B verifier/aggregator must satisfy before its
/// `zk_batch_root` can be admitted into the EFB commitment.
pub fn validate_zk_profile(profile: &ZkProfile) -> Result<(), ShardingError> {
    profile.validate()
}

pub fn assign_shard(
    epoch_seed: &[u8; 32],
    validator_id: &[u8; 48],
    shard_count: u32,
    bond_weight: u64,
) -> Result<u32, ShardingError> {
    if shard_count == 0 {
        return Err(ShardingError::ZeroShardCount);
    }
    if shard_count > MAX_SHARDS_WIRE {
        return Err(ShardingError::ShardCountTooLarge);
    }

    let mut buf = [0u8; 92];
    buf[0..32].copy_from_slice(epoch_seed);
    buf[32..80].copy_from_slice(validator_id);
    buf[80..88].copy_from_slice(&bond_weight.to_be_bytes());
    buf[88..92].copy_from_slice(&shard_count.to_be_bytes());

    let digest = h_domain(DomainTag::ShardAssignment, &buf);
    let mut word = [0u8; 8];
    word.copy_from_slice(&digest[0..8]);
    let slot = u64::from_be_bytes(word) % u64::from(shard_count);
    Ok(slot as u32)
}

pub fn receipt_id(receipt: &CrossShardReceipt) -> [u8; 32] {
    let mut buf = [0u8; 88];
    buf[0..8].copy_from_slice(&receipt.epoch.to_be_bytes());
    buf[8..12].copy_from_slice(&receipt.source_shard.to_be_bytes());
    buf[12..16].copy_from_slice(&receipt.target_shard.to_be_bytes());
    buf[16..24].copy_from_slice(&receipt.nonce.to_be_bytes());
    buf[24..56].copy_from_slice(&receipt.payload_hash);
    // 32 bytes reserved for future transparent inclusion proof commitment.
    h_domain(DomainTag::CrossShardReceipt, &buf)
}

pub fn receipt_is_epoch_anchored(
    receipt: &CrossShardReceipt,
    efb: &EpochFinalityBeacon,
) -> Result<(), ShardingError> {
    if receipt.epoch != efb.epoch {
        return Err(ShardingError::ReceiptEpochMismatch);
    }
    if receipt.source_shard >= efb.shard_count || receipt.target_shard >= efb.shard_count {
        return Err(ShardingError::ReceiptShardMismatch);
    }
    Ok(())
}

pub fn verify_receipt_inclusion(
    receipt: &CrossShardReceipt,
    receipt_root: &[u8; 32],
    leaf_index: u64,
    proof: &[[u8; 32]],
) -> Result<(), ShardingError> {
    if proof.len() > 64 {
        return Err(ShardingError::InvalidMerkleProof);
    }

    let id = receipt_id(receipt);
    let mut leaf_buf = [0u8; 40];
    leaf_buf[0..8].copy_from_slice(&leaf_index.to_be_bytes());
    leaf_buf[8..40].copy_from_slice(&id);
    let mut root = h_domain(DomainTag::LeafHash, &leaf_buf);

    let mut branch_buf = [0u8; 64];
    for (depth, sibling) in proof.iter().enumerate() {
        let bit = (leaf_index >> depth) & 1;
        if bit == 0 {
            branch_buf[0..32].copy_from_slice(&root);
            branch_buf[32..64].copy_from_slice(sibling);
        } else {
            branch_buf[0..32].copy_from_slice(sibling);
            branch_buf[32..64].copy_from_slice(&root);
        }
        root = h_domain(DomainTag::InternalHash, &branch_buf);
    }

    if root == *receipt_root {
        Ok(())
    } else {
        Err(ShardingError::ReceiptNotIncluded)
    }
}

pub fn compute_efb(
    epoch: u64,
    previous_efb_root: [u8; 32],
    shards: &[ShardCommitment],
    zk_batch_root: [u8; 32],
) -> Result<EpochFinalityBeacon, ShardingError> {
    if shards.is_empty() {
        return Err(ShardingError::ZeroShardCount);
    }
    if shards.len() > MAX_SHARDS {
        return Err(ShardingError::ShardCountTooLarge);
    }

    let shard_count = shards.len() as u32;
    validate_sorted_shards(shards, shard_count)?;

    let aggregate_state_root = aggregate_shard_field(shards, true);
    let aggregate_receipt_root = aggregate_shard_field(shards, false);

    let mut buf = [0u8; 112];
    buf[0..8].copy_from_slice(&epoch.to_be_bytes());
    buf[8..40].copy_from_slice(&previous_efb_root);
    buf[40..44].copy_from_slice(&shard_count.to_be_bytes());
    buf[44..76].copy_from_slice(&aggregate_state_root);
    buf[76..108].copy_from_slice(&aggregate_receipt_root);
    buf[108..112].copy_from_slice(&(DomainTag::EpochFinalityBeacon as u32).to_be_bytes());

    let header_root = h_domain(DomainTag::EpochFinalityBeacon, &buf);
    let mut final_buf = [0u8; 64];
    final_buf[0..32].copy_from_slice(&header_root);
    final_buf[32..64].copy_from_slice(&zk_batch_root);
    let efb_root = h_domain(DomainTag::EpochFinalityBeacon, &final_buf);

    Ok(EpochFinalityBeacon {
        epoch,
        previous_efb_root,
        shard_count,
        aggregate_state_root,
        aggregate_receipt_root,
        zk_batch_root,
        efb_root,
    })
}

fn validate_sorted_shards(
    shards: &[ShardCommitment],
    shard_count: u32,
) -> Result<(), ShardingError> {
    let mut expected = 0u32;
    for shard in shards {
        if shard.shard_id >= shard_count {
            return Err(ShardingError::ShardOutOfRange);
        }
        if shard.shard_id < expected {
            if expected > 0 && shard.shard_id == expected - 1 {
                return Err(ShardingError::DuplicateShard);
            }
            return Err(ShardingError::ShardsNotSorted);
        }
        if shard.shard_id > expected {
            return Err(ShardingError::InvalidShardCount);
        }
        expected = match expected.checked_add(1) {
            Some(next) => next,
            None => return Err(ShardingError::InvalidShardCount),
        };
    }
    if expected != shard_count {
        return Err(ShardingError::InvalidShardCount);
    }
    Ok(())
}

fn aggregate_shard_field(shards: &[ShardCommitment], state: bool) -> [u8; 32] {
    let mut root = [0u8; 32];
    let mut buf = [0u8; 68];
    for shard in shards {
        buf[0..32].copy_from_slice(&root);
        buf[32..36].copy_from_slice(&shard.shard_id.to_be_bytes());
        if state {
            buf[36..68].copy_from_slice(&shard.state_root);
        } else {
            buf[36..68].copy_from_slice(&shard.receipt_root);
        }
        root = h_domain(DomainTag::EpochFinalityBeacon, &buf);
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shard(id: u32, state_byte: u8, receipt_byte: u8) -> ShardCommitment {
        ShardCommitment {
            shard_id: id,
            state_root: [state_byte; 32],
            receipt_root: [receipt_byte; 32],
        }
    }

    #[test]
    fn shard_assignment_is_deterministic() {
        let seed = [0x11; 32];
        let validator = [0x22; 48];
        let a = assign_shard(&seed, &validator, 64, 10).unwrap();
        let b = assign_shard(&seed, &validator, 64, 10).unwrap();
        assert_eq!(a, b);
        assert!(a < 64);
    }

    #[test]
    fn shard_assignment_rejects_zero_shards() {
        let seed = [0u8; 32];
        let validator = [0u8; 48];
        assert_eq!(
            assign_shard(&seed, &validator, 0, 1),
            Err(ShardingError::ZeroShardCount)
        );
    }

    #[test]
    fn efb_is_deterministic_for_sorted_shards() {
        let shards = [shard(0, 1, 2), shard(1, 3, 4), shard(2, 5, 6)];
        let a = compute_efb(7, [0u8; 32], &shards, [9u8; 32]).unwrap();
        let b = compute_efb(7, [0u8; 32], &shards, [9u8; 32]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.shard_count, 3);
        assert_ne!(a.efb_root, [0u8; 32]);
    }

    #[test]
    fn zk_profile_locks_pr93_shape_without_verifying_proofs() {
        assert_eq!(
            validate_zk_profile(&ZkProfile::PLONKY3_FRI_POSEIDON_QASH),
            Ok(())
        );

        let wrong_depth = ZkProfile {
            recursion_depth: 3,
            ..ZkProfile::PLONKY3_FRI_POSEIDON_QASH
        };
        assert_eq!(
            validate_zk_profile(&wrong_depth),
            Err(ShardingError::InvalidZkRecursionDepth)
        );

        let wrong_factor = ZkProfile {
            layer1_aggregation_factor: 32,
            ..ZkProfile::PLONKY3_FRI_POSEIDON_QASH
        };
        assert_eq!(
            validate_zk_profile(&wrong_factor),
            Err(ShardingError::InvalidZkAggregationFactor)
        );
    }

    #[test]
    fn efb_rejects_unsorted_or_missing_shards() {
        let unsorted = [shard(1, 1, 1), shard(0, 2, 2)];
        assert_eq!(
            compute_efb(1, [0u8; 32], &unsorted, [0u8; 32]),
            Err(ShardingError::InvalidShardCount)
        );

        let missing = [shard(0, 1, 1), shard(2, 2, 2)];
        assert_eq!(
            compute_efb(1, [0u8; 32], &missing, [0u8; 32]),
            Err(ShardingError::ShardOutOfRange)
        );

        let duplicate = [shard(0, 1, 1), shard(0, 2, 2)];
        assert_eq!(
            compute_efb(1, [0u8; 32], &duplicate, [0u8; 32]),
            Err(ShardingError::DuplicateShard)
        );
    }

    #[test]
    fn receipt_id_binds_epoch_shards_nonce_and_payload() {
        let base = CrossShardReceipt {
            epoch: 10,
            source_shard: 1,
            target_shard: 2,
            nonce: 99,
            payload_hash: [0xAA; 32],
        };
        let mut changed = base;
        changed.nonce = 100;
        assert_ne!(receipt_id(&base), receipt_id(&changed));
    }

    #[test]
    fn receipt_epoch_anchor_rejects_replay() {
        let shards = [shard(0, 1, 2), shard(1, 3, 4), shard(2, 5, 6)];
        let efb = compute_efb(11, [0u8; 32], &shards, [0u8; 32]).unwrap();
        let receipt = CrossShardReceipt {
            epoch: 10,
            source_shard: 0,
            target_shard: 1,
            nonce: 1,
            payload_hash: [0xAA; 32],
        };
        assert_eq!(
            receipt_is_epoch_anchored(&receipt, &efb),
            Err(ShardingError::ReceiptEpochMismatch)
        );
    }

    #[test]
    fn receipt_inclusion_verifies_merkle_path() {
        let receipt = CrossShardReceipt {
            epoch: 10,
            source_shard: 1,
            target_shard: 2,
            nonce: 99,
            payload_hash: [0xAA; 32],
        };
        let leaf_index = 2u64;
        let proof = [[0x11; 32], [0x22; 32], [0x33; 32]];
        let root = test_receipt_root(&receipt, leaf_index, &proof);

        assert_eq!(
            verify_receipt_inclusion(&receipt, &root, leaf_index, &proof),
            Ok(())
        );
    }

    #[test]
    fn receipt_inclusion_rejects_tampered_path() {
        let receipt = CrossShardReceipt {
            epoch: 10,
            source_shard: 1,
            target_shard: 2,
            nonce: 99,
            payload_hash: [0xAA; 32],
        };
        let leaf_index = 2u64;
        let proof = [[0x11; 32], [0x22; 32], [0x33; 32]];
        let root = test_receipt_root(&receipt, leaf_index, &proof);
        let mut tampered = proof;
        tampered[1] = [0x44; 32];

        assert_eq!(
            verify_receipt_inclusion(&receipt, &root, leaf_index, &tampered),
            Err(ShardingError::ReceiptNotIncluded)
        );
    }

    #[test]
    fn receipt_inclusion_rejects_overdeep_path() {
        let receipt = CrossShardReceipt {
            epoch: 10,
            source_shard: 1,
            target_shard: 2,
            nonce: 99,
            payload_hash: [0xAA; 32],
        };
        let proof = [[0u8; 32]; 65];

        assert_eq!(
            verify_receipt_inclusion(&receipt, &[0u8; 32], 0, &proof),
            Err(ShardingError::InvalidMerkleProof)
        );
    }

    fn test_receipt_root(
        receipt: &CrossShardReceipt,
        leaf_index: u64,
        proof: &[[u8; 32]],
    ) -> [u8; 32] {
        let id = receipt_id(receipt);
        let mut leaf_buf = [0u8; 40];
        leaf_buf[0..8].copy_from_slice(&leaf_index.to_be_bytes());
        leaf_buf[8..40].copy_from_slice(&id);
        let mut root = h_domain(DomainTag::LeafHash, &leaf_buf);

        let mut branch_buf = [0u8; 64];
        for (depth, sibling) in proof.iter().enumerate() {
            let bit = (leaf_index >> depth) & 1;
            if bit == 0 {
                branch_buf[0..32].copy_from_slice(&root);
                branch_buf[32..64].copy_from_slice(sibling);
            } else {
                branch_buf[0..32].copy_from_slice(sibling);
                branch_buf[32..64].copy_from_slice(&root);
            }
            root = h_domain(DomainTag::InternalHash, &branch_buf);
        }
        root
    }
}
