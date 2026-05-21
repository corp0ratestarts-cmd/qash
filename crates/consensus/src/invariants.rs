//! Executable protocol invariants for EpochState.
//!
//! `check_state_invariants` converts the implicit assumptions baked into
//! `advance_epoch` into explicit, callable checks. Callers can use it:
//!   - as a pre-condition check before submitting a state to `advance_epoch`
//!   - as a post-condition check after decoding a state from the wire
//!   - in tests to assert that transitions never produce invalid states
//!
//! All invariants here correspond to §A4 (metric bounds) and §A6
//! (halt monotonicity) from docs/spec/02_transition_axioms.md.

use crate::fixed_point::SCALE;
use crate::lyapunov::{ValidatorMetrics, WINDOW_SIZE};
use crate::transition::{EpochState, MAX_VALIDATORS_WIRE};

/// Violation kinds returned by `check_state_invariants`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantViolation {
    /// validator_count exceeds MAX_VALIDATORS_WIRE.
    ValidatorCountExceedsMax,
    /// Divergence D_i is outside [0, SCALE] for some validator i.
    DivergenceOutOfBounds,
    /// Conflict C_i is outside [0, SCALE] for some validator i.
    ConflictOutOfBounds,
    /// Slash accumulator Σ_i is negative for some validator i.
    SlashAccumNegative,
    /// Convergence window fill count exceeds WINDOW_SIZE.
    WindowFillExceedsMax,
}

/// Check that `state` satisfies all Domain A structural invariants.
///
/// Returns `Ok(())` if the state is well-formed, or
/// `Err(InvariantViolation)` describing the first violation found.
///
/// This function is deterministic and has no side effects. It is safe to
/// call on any `EpochState`, including halted states.
pub fn check_state_invariants(state: &EpochState) -> Result<(), InvariantViolation> {
    if state.validator_count > MAX_VALIDATORS_WIRE {
        return Err(InvariantViolation::ValidatorCountExceedsMax);
    }

    let vc = state.validator_count as usize;
    for i in 0..vc {
        check_validator_metrics(&state.validators[i])?;
    }

    let (filled, _) = state.convergence_window.raw_parts();
    if filled as usize > WINDOW_SIZE {
        return Err(InvariantViolation::WindowFillExceedsMax);
    }

    Ok(())
}

fn check_validator_metrics(v: &ValidatorMetrics) -> Result<(), InvariantViolation> {
    let d = v.divergence.raw();
    if !(0..=SCALE).contains(&d) {
        return Err(InvariantViolation::DivergenceOutOfBounds);
    }
    let c = v.conflict.raw();
    if !(0..=SCALE).contains(&c) {
        return Err(InvariantViolation::ConflictOutOfBounds);
    }
    if !v.slash_accum.is_non_negative() {
        return Err(InvariantViolation::SlashAccumNegative);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed_point::FixedPoint;
    use crate::lyapunov::ConvergenceWindow;
    use crate::transition::{HaltReason, MAX_VALIDATORS};

    fn valid_state() -> EpochState {
        EpochState {
            epoch: 0,
            halt_reason: HaltReason::None,
            entropy_seed: [0u8; 32],
            validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
            validator_count: 4,
            convergence_window: ConvergenceWindow::new(),
            nonces: [0u64; MAX_VALIDATORS],
            validator_ids: [[0u8; 48]; MAX_VALIDATORS],
            cascade_health: 0,
            state_root: [0u8; 32],
            receipt_root: [0u8; 32],
            efb_root: [0u8; 32],
            causal_fingerprint: [0u8; 32],
        }
    }

    #[test]
    fn valid_genesis_passes() {
        assert!(check_state_invariants(&valid_state()).is_ok());
    }

    #[test]
    fn valid_halted_state_passes() {
        let mut s = valid_state();
        s.halt_reason = HaltReason::LyapunovViolation;
        assert!(check_state_invariants(&s).is_ok());
    }

    #[test]
    fn divergence_above_scale_fails() {
        let mut s = valid_state();
        s.validators[0].divergence = FixedPoint::from_raw(SCALE + 1);
        assert_eq!(
            check_state_invariants(&s),
            Err(InvariantViolation::DivergenceOutOfBounds)
        );
    }

    #[test]
    fn divergence_negative_fails() {
        let mut s = valid_state();
        s.validators[0].divergence = FixedPoint::from_raw(-1);
        assert_eq!(
            check_state_invariants(&s),
            Err(InvariantViolation::DivergenceOutOfBounds)
        );
    }

    #[test]
    fn conflict_above_scale_fails() {
        let mut s = valid_state();
        s.validators[0].conflict = FixedPoint::from_raw(SCALE + 1);
        assert_eq!(
            check_state_invariants(&s),
            Err(InvariantViolation::ConflictOutOfBounds)
        );
    }

    #[test]
    fn slash_accum_negative_fails() {
        let mut s = valid_state();
        s.validators[0].slash_accum = FixedPoint::from_raw(-1);
        assert_eq!(
            check_state_invariants(&s),
            Err(InvariantViolation::SlashAccumNegative)
        );
    }

    #[test]
    fn validator_count_exceeds_max_fails() {
        let mut s = valid_state();
        s.validator_count = MAX_VALIDATORS_WIRE + 1;
        assert_eq!(
            check_state_invariants(&s),
            Err(InvariantViolation::ValidatorCountExceedsMax)
        );
    }

    #[test]
    fn max_valid_metrics_passes() {
        let mut s = valid_state();
        s.validators[0].divergence = FixedPoint::from_raw(SCALE);
        s.validators[0].conflict = FixedPoint::from_raw(SCALE);
        s.validators[0].slash_accum = FixedPoint::from_raw(SCALE);
        assert!(check_state_invariants(&s).is_ok());
    }

    #[test]
    fn inactive_slots_beyond_vc_not_checked() {
        // Slots beyond validator_count may hold stale non-zero values from a
        // previous genesis (array is fixed-size); only active slots [0..vc) matter.
        let mut s = valid_state();
        s.validators[4].divergence = FixedPoint::from_raw(SCALE + 999);
        assert!(check_state_invariants(&s).is_ok());
    }
}
