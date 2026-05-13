//! Lyapunov evaluation primitives (deterministic, no heap).

use crate::fixed_point::{FixedPoint, OverflowError, SCALE};

/// Protocol-facing window size (u32). Used in wire encoding.
pub const WINDOW_SIZE_WIRE: u32 = 3;
/// Array-sizing alias (usize required by Rust array syntax; not stored in state).
pub const WINDOW_SIZE: usize = WINDOW_SIZE_WIRE as usize;

pub const WEIGHT_D:  FixedPoint = FixedPoint::from_raw(350_000);
pub const WEIGHT_C:  FixedPoint = FixedPoint::from_raw(300_000);
pub const WEIGHT_S:  FixedPoint = FixedPoint::from_raw(200_000);
pub const WEIGHT_CH: FixedPoint = FixedPoint::from_raw(150_000);
pub const EPSILON:   FixedPoint = FixedPoint::from_raw(20_000);

/// Genesis parameter: max transactions admitted per epoch.
pub const MAX_QUERIES_PER_EPOCH: i128 = 1_000_000;

/// Φ_safety halt threshold (as FixedPoint.raw()).
/// = N_max × floor(γ_raw × i64::MAX / p) / 2
/// = 1024 × floor(200_000 × 9_223_372_036_854_775_807 / 1_000_000) / 2
/// ≈ 9.44 × 10^20  (fits comfortably in i128)
///
/// If phi_safety.raw() >= PHI_MAX_SAFE the network enters absorbing halt.
/// This is the §5 Condition 2 gate (ADR-001 accepted).
pub const PHI_MAX_SAFE: i128 =
    1024_i128 * (200_000_i128 * (i64::MAX as i128) / 1_000_000_i128) / 2;

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
    pub slash_accum: FixedPoint,  // Σ >= 0 (monotone; bounded by i64::MAX via admissibility)
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

    /// Access raw window internals. Used by golden replay tests and commitment encoding.
    pub fn raw_parts(&self) -> (u8, &[FixedPoint; WINDOW_SIZE]) {
        (self.filled, &self.values)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LyapunovEval {
    /// V_convergence = Σ(α·D + β·C) + χ·CH. Used for δ_window check.
    pub v_convergence: FixedPoint,
    /// Φ_safety = Σ_i(γ·Σ_i) (sum over all validators, ADR-001 accepted).
    /// Gate: if phi_safety.raw() >= PHI_MAX_SAFE → absorbing halt.
    pub phi_safety: FixedPoint,
    /// V_total = V_convergence + Φ_safety. Informational.
    pub v_total: FixedPoint,
    /// δ_window = V_convergence - min(preceding window).
    pub delta_window: FixedPoint,
    /// Whether δ_window > ε (triggers H1 halt).
    pub halt_triggered: bool,
}

pub fn compute_delta_window(
    v_current: FixedPoint,
    window: &ConvergenceWindow,
) -> Result<FixedPoint, LyapunovError> {
    let min_w = window.min_value();
    Ok(v_current.checked_sub(min_w)?)
}

/// Standalone evaluate() for unit tests. Main path uses evaluate_projected() in transition.rs.
/// cascade_fail_count: number of cascade proof failures in this epoch's input set.
pub fn evaluate(
    validators: &[ValidatorMetrics],
    window: &ConvergenceWindow,
    cascade_fail_count: u32,
) -> Result<LyapunovEval, LyapunovError> {
    let mut v_sum = FixedPoint::ZERO;
    let mut phi_acc = FixedPoint::ZERO;

    for v in validators {
        if !v.metrics_bounded() {
            return Err(LyapunovError::UnboundedMetric);
        }
        let term_d = WEIGHT_D.checked_mul(v.divergence)?;
        let term_c = WEIGHT_C.checked_mul(v.conflict)?;
        let term = term_d.checked_add(term_c)?;
        v_sum = v_sum.checked_add(term)?;

        // Φ_safety: sum over validators (ADR-001 accepted — sum, not max)
        let phi_term = WEIGHT_S.checked_mul(v.slash_accum)?;
        phi_acc = phi_acc.checked_add(phi_term)?;
    }

    // CH term: cascade health factor (v1.1, §4c)
    let ch_raw = (cascade_fail_count as i128) * SCALE / MAX_QUERIES_PER_EPOCH;
    let ch_term = WEIGHT_CH.checked_mul(FixedPoint::from_raw(ch_raw))?;
    v_sum = v_sum.checked_add(ch_term)?;

    let v_total = v_sum.checked_add(phi_acc)?;

    let (delta_window, halt_triggered) = if window.is_full() {
        let delta = compute_delta_window(v_sum, window)?;
        (delta, delta.raw() > EPSILON.raw())
    } else {
        (FixedPoint::ZERO, false)
    };

    Ok(LyapunovEval {
        v_convergence: v_sum,
        phi_safety: phi_acc,
        v_total,
        delta_window,
        halt_triggered,
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

    #[test]
    fn phi_max_safe_is_positive() {
        assert!(PHI_MAX_SAFE > 0);
    }
}
