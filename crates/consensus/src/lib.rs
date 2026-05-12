#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
extern crate std;

pub mod hash;
pub mod fixed_point;
pub mod encoding;
pub mod lyapunov;
pub mod transition;
pub mod params;

// Re-exports (ergonomic public API)
pub use fixed_point::FixedPoint;
pub use hash::{h_domain, sha3_256, DomainTag};
pub use lyapunov::{ValidatorMetrics, LyapunovEval};
pub use transition::{EpochState, EpochInput, ValidatorUpdate, HaltReason, advance_epoch, MAX_VALIDATORS};
