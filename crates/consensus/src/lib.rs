#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
extern crate std;

pub mod encoding;
pub mod fixed_point;
pub mod hash;
pub mod lyapunov;
pub mod public;
pub mod transition;
pub mod params;
pub mod transaction;

// Re-exports (ergonomic public API)
pub use blinding::{BlindingMode, blind_cascade_input, derive_dilithium_blinding_scalar, split_chunk_key, reconstruct_chunk_key};
pub use cascade::{h_cascade, h_cascade_keyed, h_cascade_derive};
pub use cascade::{DOM_SEP_L1, DOM_SEP_L2, DOM_SEP_L3, DOM_SEP_L4, DOM_SEP_L5, DOM_SEP_L6, DOM_SEP_L7};
pub use fixed_point::FixedPoint;
pub use hash::{h_domain, sha3_256, DomainTag};
pub use public::PublicTranscript;
pub use lyapunov::{ValidatorMetrics, LyapunovEval};
pub use transition::{
    EpochState, EpochInput, ValidatorUpdate, HaltReason,
    advance_epoch, encode_full_state_into, decode_full_state,
    MAX_VALIDATORS, FULL_STATE_MAX_BYTES,
};
