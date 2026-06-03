//! Pure QASH consensus crate.
//!
//! Domain A rules apply to all code in this crate:
//!   - no unsafe
//!   - no f32/f64
//!   - no usize/isize in state struct fields or wire arithmetic
//!   - all arithmetic checked; overflow → HaltReason::ArithOverflow
//!   - no wall-clock, no entropy, no oracle, no governance
//!   - replay-invariant across all authorized ISAs

#![no_std]

pub mod economics;
pub mod public;
pub mod transition;
