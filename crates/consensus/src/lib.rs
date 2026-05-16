#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
extern crate std;

pub mod hash;
pub mod lsh256;
pub mod fixed_point;
pub mod encoding;
pub mod lyapunov;
pub mod public;
pub mod transition;
pub mod params;
pub mod transaction;
pub mod derive;
pub mod invariants;

// Re-exports (ergonomic public API)
pub use fixed_point::FixedPoint;
pub use hash::{h_domain, sha3_256, DomainTag};
pub use lsh256::{lsh256, lsh256_domain, lsh256_parts};
pub use public::PublicTranscript;
pub use lyapunov::{ValidatorMetrics, LyapunovEval};
pub use transition::{
    EpochState, EpochInput, ValidatorUpdate, HaltReason,
    advance_epoch, encode_full_state_into, decode_full_state,
    MAX_VALIDATORS, FULL_STATE_MAX_BYTES, TransitionResult,
};
pub use derive::{derive_leaf_index, verify_leaf_index};
pub use invariants::{check_state_invariants, InvariantViolation};
