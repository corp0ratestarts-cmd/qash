//! Lyapunov evaluation primitives (deterministic, no heap).

use crate::fixed_point::{FixedPoint, OverflowError, SCALE};

/// Protocol-facing window size (u32). Used in wire encoding.
pub const WINDOW_SIZE_WIRE: u32 = 3;
/// Array-sizing alias (usize required by Rust array syntax; not stored in state).
pub const WINDOW_SIZE: usize = WINDOW_SIZE_WIRE as usize;

pub const WEIGHT_D:    FixedPoint = FixedPoint::from_raw(400_000);
pub const WEIGHT_C:    FixedPoint = FixedPoint::from_raw(350_000);
pub const WEIGHT_S:    FixedPoint = FixedPoint::from_raw(250_000);
pub const EPSILON:     FixedPoint = FixedPoint::from_raw(20_000);
/// Φ_safety halt threshold (ADR-001 accepted). Raw i128 ≥ this value triggers H7.
/// Derived: W_S · Σ slash_i ≥ 500_000_000 ≡ aggregate slash energy ≥ 2·10⁹ raw units.
pub const PHI_MAX_SAFE: FixedPoint = FixedPoint::from_raw(500_000_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyapunovError {
    Overflow,
    UnboundedMetric,
}

impl From<OverflowError> for LyapunovError {
    fn from(_: OverflowError) -> Self { LyapunovError::Overflow }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatorMetrics {
    pub divergence: FixedPoint,   // D in [0, SCALE]
    pub conflict: FixedPoint,     // C in [0, SCALE]
    pub slash_accum: FixedPoint,  // Σ >= 0 (monotone; not bounded by protocol)
}

impl ValidatorMetrics {
    pub const ZERO: ValidatorMetrics = ValidatorMetrics {
        divergence: FixedPoint::ZERO,
        conflict: FixedPoint::ZERO,
        slash_accum: FixedPoint::ZERO,
    };

    #[inline]
    pub fn metrics_bounded(&self) -> bool {
        let s = SCALE;
        self.divergence.raw() >= 0 && self.divergence.raw() <= s &&
        self.conflict.raw()  >= 0 && self.conflict.raw()  <= s
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvergenceWindow {
    pub(crate) values: [FixedPoint; WINDOW_SIZE],
    pub(crate) filled: u8,
}

impl Default for ConvergenceWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ConvergenceWindow {
    pub const fn new() -> Self {
        ConvergenceWindow {
            values: [FixedPoint::ZERO; WINDOW_SIZE],
            filled: 0,
        }
    }

    pub fn push(&mut self, v: FixedPoint) {
        let mut i: usize = WINDOW_SIZE;
        while i > 1 {
            i -= 1;
            self.values[i] = self.values[i - 1];
        }
        self.values[0] = v;

        if (self.filled as usize) < WINDOW_SIZE {
            self.filled += 1;
        }
    }

    pub fn is_full(&self) -> bool {
        (self.filled as usize) >= WINDOW_SIZE
    }

    pub fn min_value(&self) -> FixedPoint {
        if self.filled == 0 {
            return FixedPoint::ZERO;
        }
        let mut min = self.values[0];
        let mut i: usize = 1;
        while i < (self.filled as usize) {
            min = min.min(self.values[i]);
            i += 1;
        }
        min
    }

    /// Access raw window internals. Used by golden replay tests.
    /// Not part of the consensus transition API.
    pub fn raw_parts(&self) -> (u8, &[FixedPoint; WINDOW_SIZE]) {
        (self.filled, &self.values)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LyapunovEval {
    /// V_convergence = Σ(α·D + β·C). Used for δ_window check.
    pub v_convergence: FixedPoint,
    /// Φ_safety = γ·Σ(slash_i). ADR-001/002: sum aggregation.
    pub phi_safety: FixedPoint,
    /// V_total = V_convergence + Φ_safety. Informational.
    pub v_total: FixedPoint,
    /// δ_window = V_convergence - min(preceding window).
    pub delta_window: FixedPoint,
    /// Whether δ_window > ε (triggers H1 halt).
    pub halt_triggered: bool,
    /// Whether Φ_safety ≥ PHI_MAX_SAFE (triggers H7 halt).
    pub phi_halt_triggered: bool,
}

pub fn compute_delta_window(
    v_current: FixedPoint,
    window: &ConvergenceWindow,
) -> Result<FixedPoint, LyapunovError> {
    let min_w = window.min_value();
    Ok(v_current.checked_sub(min_w)?)
}

pub fn evaluate(
    validators: &[ValidatorMetrics],
    window: &ConvergenceWindow,
) -> Result<LyapunovEval, LyapunovError> {
    let mut v_sum = FixedPoint::ZERO;
    let mut sum_slash = FixedPoint::ZERO;

    for v in validators {
        if !v.metrics_bounded() {
            return Err(LyapunovError::UnboundedMetric);
        }
        let term_d = WEIGHT_D.checked_mul(v.divergence)?;
        let term_c = WEIGHT_C.checked_mul(v.conflict)?;
        let term = term_d.checked_add(term_c)?;
        v_sum = v_sum.checked_add(term)?;
        sum_slash = sum_slash.checked_add(v.slash_accum)?;
    }

    let phi = WEIGHT_S.checked_mul(sum_slash)?;
    let v_total = v_sum.checked_add(phi)?;

    // δ_window is checked against V_convergence, NOT V_total.
    let (delta_window, halt_triggered) = if window.is_full() {
        let delta = compute_delta_window(v_sum, window)?;
        (delta, delta.raw() > EPSILON.raw())
    } else {
        (FixedPoint::ZERO, false)
    };

    let phi_halt_triggered = phi.raw() >= PHI_MAX_SAFE.raw();

    Ok(LyapunovEval {
        v_convergence: v_sum,
        phi_safety: phi,
        v_total,
        delta_window,
        halt_triggered,
        phi_halt_triggered,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_shifts_correctly() {
        let mut w = ConvergenceWindow::new();

        w.push(FixedPoint::from_raw(100));
        assert_eq!(w.values[0].raw(), 100);
        assert_eq!(w.filled, 1);

        w.push(FixedPoint::from_raw(200));
        assert_eq!(w.values[0].raw(), 200);
        assert_eq!(w.values[1].raw(), 100);
        assert_eq!(w.filled, 2);

        w.push(FixedPoint::from_raw(300));
        assert_eq!(w.values[0].raw(), 300);
        assert_eq!(w.values[1].raw(), 200);
        assert_eq!(w.values[2].raw(), 100);
        assert_eq!(w.filled, 3);
        assert!(w.is_full());

        w.push(FixedPoint::from_raw(400));
        assert_eq!(w.values[0].raw(), 400);
        assert_eq!(w.values[1].raw(), 300);
        assert_eq!(w.values[2].raw(), 200);
        assert_eq!(w.filled, 3);

        assert_eq!(w.min_value().raw(), 200);
    }

    // ADR-001/002: phi_safety uses sum aggregation, not max.
    // This test would pass under max but fails unless sum is used.
    #[test]
    fn phi_safety_sums_across_validators() {
        let slash_a = FixedPoint::from_raw(400_000_000); // 400 in fp
        let slash_b = FixedPoint::from_raw(400_000_000); // 400 in fp
        let validators = [
            ValidatorMetrics { divergence: FixedPoint::ZERO, conflict: FixedPoint::ZERO, slash_accum: slash_a },
            ValidatorMetrics { divergence: FixedPoint::ZERO, conflict: FixedPoint::ZERO, slash_accum: slash_b },
        ];
        let window = ConvergenceWindow::new();
        let result = evaluate(&validators, &window).unwrap();
        // sum = 800_000_000; phi = floor(250_000 * 800_000_000 / 1_000_000) = 200_000_000
        // max would give: floor(250_000 * 400_000_000 / 1_000_000) = 100_000_000
        assert_eq!(result.phi_safety.raw(), 200_000_000, "phi must use sum, not max");
        assert!(!result.phi_halt_triggered, "200_000_000 < PHI_MAX_SAFE=500_000_000");
    }

    #[test]
    fn phi_halt_triggers_at_threshold() {
        // Two validators whose combined slash pushes phi >= PHI_MAX_SAFE (500_000_000).
        // Need sum_slash such that floor(250_000 * sum_slash / 1_000_000) >= 500_000_000
        // i.e. sum_slash >= 2_000_000_000
        let slash_each = FixedPoint::from_raw(1_000_000_000); // sum = 2_000_000_000
        let validators = [
            ValidatorMetrics { divergence: FixedPoint::ZERO, conflict: FixedPoint::ZERO, slash_accum: slash_each },
            ValidatorMetrics { divergence: FixedPoint::ZERO, conflict: FixedPoint::ZERO, slash_accum: slash_each },
        ];
        let window = ConvergenceWindow::new();
        let result = evaluate(&validators, &window).unwrap();
        // phi = floor(250_000 * 2_000_000_000 / 1_000_000) = 500_000_000 == PHI_MAX_SAFE
        assert_eq!(result.phi_safety.raw(), 500_000_000);
        assert!(result.phi_halt_triggered, "phi == PHI_MAX_SAFE must trigger halt");
    }
}
