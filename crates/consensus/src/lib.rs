#![no_std]
#![forbid(unsafe_code)]
// Domain A boundary enforcement: ban std types that introduce non-determinism.
// The full list lives in crates/consensus/clippy.toml.
#![deny(clippy::disallowed_types)]

#[cfg(test)]
extern crate std;

pub mod capability;
pub mod causal_order;
pub mod derive;
pub mod domain;
pub mod encoding;
pub mod envelope;
pub mod fixed_point;
pub mod hash;
pub mod invariants;
pub mod lineage;
pub mod lsh256;
pub mod lsh512;
pub mod lyapunov;
pub mod params;
pub mod public;
pub mod sharding;
pub mod transaction;
pub mod transition;
// Not yet activated in transition.rs (v1.1.1 feature gate — when WEIGHT_BH > 0).
// Declared here so their unit tests run in CI.
#[allow(dead_code)]
pub mod blinding;
#[allow(dead_code)]
pub mod cascade;

// Re-exports (ergonomic public API)
pub use capability::{validate_capability, Capability};
pub use causal_order::{compute_sort_key, sort_key_from_payload};
pub use derive::{derive_leaf_index, verify_leaf_index};
pub use domain::{CapToken, DomainA};
pub use envelope::{Envelope, PROTOCOL_VERSION_V1_0, PROTOCOL_VERSION_V1_1, PROTOCOL_VERSION_V1_2};
pub use fixed_point::FixedPoint;
pub use hash::{h_domain, sha3_256, DomainTag};
pub use invariants::{check_state_invariants, InvariantViolation};
pub use lineage::{SkipListHeader, SKIPLIST_DEPTH, SKIP_DISTANCES};
pub use lsh256::{lsh256, lsh256_domain, lsh256_parts};
pub use lsh512::{lsh512, lsh512_domain, lsh512_parts};
pub use lyapunov::{LyapunovEval, ValidatorMetrics};
pub use public::PublicTranscript;
pub use sharding::{
    assign_shard, compute_efb, receipt_id, receipt_is_epoch_anchored, validate_zk_profile,
    verify_receipt_inclusion, CrossShardReceipt, EpochFinalityBeacon, ShardCommitment,
    ShardingError, ZkProfile, MAX_SHARDS, MAX_SHARDS_WIRE, ZK_LAYER0_SHARD_VALIDITY,
    ZK_LAYER1_AGGREGATION, ZK_LAYER1_AGGREGATION_FACTOR, ZK_LAYER2_EFB,
    ZK_PROFILE_ID_PLONKY3_FRI_POSEIDON_QASH, ZK_RECURSION_DEPTH,
};
pub use transition::advance_epoch_sharded;
pub use transition::EpochShardingInput;
pub use transition::{
    advance_epoch, decode_full_state, encode_full_state_into, try_encode_full_state_into,
    EpochInput, EpochState, HaltReason, TransitionResult, ValidatorUpdate, FULL_STATE_MAX_BYTES,
    MAX_VALIDATORS,
};
