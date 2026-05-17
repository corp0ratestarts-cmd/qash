#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
extern crate std;

pub mod hash;
pub mod lsh256;
pub mod lsh512;
pub mod fixed_point;
pub mod encoding;
pub mod lyapunov;
pub mod public;
pub mod transition;
pub mod params;
pub mod transaction;
pub mod derive;
pub mod invariants;
// Not yet activated in transition.rs (v1.1.1 feature gate — when WEIGHT_BH > 0).
// Declared here so their unit tests run in CI.
#[allow(dead_code)]
pub mod cascade;
#[allow(dead_code)]
pub(crate) mod blinding;

// Re-exports (ergonomic public API)
pub use fixed_point::FixedPoint;
pub use hash::{h_domain, sha3_256, DomainTag};
pub use lsh256::{lsh256, lsh256_domain, lsh256_parts};
pub use lsh512::{lsh512, lsh512_domain, lsh512_parts};
pub use public::PublicTranscript;
pub use lyapunov::{ValidatorMetrics, LyapunovEval};
pub use transition::{
    EpochState, EpochInput, ValidatorUpdate, HaltReason,
    advance_epoch, encode_full_state_into, decode_full_state,
    MAX_VALIDATORS, FULL_STATE_MAX_BYTES, TransitionResult,
};
pub use derive::{derive_leaf_index, verify_leaf_index};
pub use invariants::{check_state_invariants, InvariantViolation};
