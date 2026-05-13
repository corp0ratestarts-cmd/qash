#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
extern crate std;

pub mod encoding;
pub mod fixed_point;
pub mod hash;
pub mod lyapunov;
pub mod params;
pub mod transition;

// Re-exports (ergonomic public API)
pub use fixed_point::FixedPoint;
pub use hash::{
    h_consensus_domain, h_domain, sha3_256, ConsensusDigestSet, DomainTag, HashPrimitive,
    PrimitiveDigest,
};
pub use lyapunov::{LyapunovEval, ValidatorMetrics};
pub use transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};
