//! Lyapunov evaluation primitives (deterministic, no heap).

use crate::fixed_point::{FixedPoint, OverflowError, SCALE};

/// Protocol-facing window size (u32). Used in wire encoding.
pub const WINDOW_SIZE_WIRE: u32 = 3;
/// Array-sizing alias (usize required by Rust array syntax; not stored in state).
pub const WINDOW_SIZE: usize = WINDOW_SIZE_WIRE as usize;

pub const WEIGHT_D: FixedPoint = FixedPoint::from_raw(400_000);
pub const WEIGHT_C: FixedPoint = FixedPoint::from_raw(350_000);
pub const WEIGHT_S: FixedPoint = FixedPoint::from_raw(250_000);
pub const EPSILON: FixedPoint = FixedPoint::from_raw(20_000);
/// Maximum safe Φ_safety value (raw fixed-point units) before H7 halt.
pub const PHI_MAX_SAFE: FixedPoint = FixedPoint::from_raw(500_000_000);

/// v1.1: cascade health threshold (from GENESIS_CONSTANTS.toml [cascade.health]).
pub const CASCADE_HEALTH_THRESHOLD: u32 = 8;
/// v1.1: weight applied to cascade health deficit in Lyapunov potential.
/// Factor = 50_000 (from GENESIS_CONSTANTS.toml [cascade.health] cascade_health_factor).
pub const CASCADE_HEALTH_FACTOR: i64 = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyapunovError {
    Overflow,
    UnboundedMetric,
}

impl From<OverflowError> for LyapunovError {
    fn from(_: OverflowError) -> Self {
        LyapunovError::Overflow
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatorMetrics {
    pub divergence: FixedPoint,  // D in [0, SCALE]
    pub conflict: FixedPoint,    // C in [0, SCALE]
    pub slash_accum: FixedPoint, // Σ >= 0 (monotone; not bounded by protocol)
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
        self.divergence.raw() >= 0
            && self.divergence.raw() <= s
            && self.conflict.raw() >= 0
            && self.conflict.raw() <= s
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
    /// Φ_safety = γ·Σ(slash_i). Used for H7 halt gate.
    pub phi_safety: FixedPoint,
    /// V_total = V_convergence + Φ_safety. Informational.
    pub v_total: FixedPoint,
    /// δ_window = V_convergence - min(preceding window).
    pub delta_window: FixedPoint,
    /// Whether δ_window > ε (triggers H1 halt).
    pub halt_triggered: bool,
    /// Whether Φ_safety >= PHI_MAX_SAFE (triggers H7 halt).
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
    // Pass full health so the cascade deficit term is zero — equivalent to pre-v1.1 behaviour.
    evaluate_with_cascade_health(validators, window, CASCADE_HEALTH_THRESHOLD)
}

/// v1.1: Lyapunov evaluation with cascade health deficit term.
///
/// Adds `χ · (health_threshold - cascade_health)` to V_total, where:
/// - `χ = CASCADE_HEALTH_FACTOR = 50_000`
/// - health deficit = `CASCADE_HEALTH_THRESHOLD.saturating_sub(cascade_health)`
///
/// When `cascade_health >= CASCADE_HEALTH_THRESHOLD`, the deficit is 0 and this
/// is identical to `evaluate(validators, window)`. When health is below threshold,
/// convergence pressure increases proportionally.
pub fn evaluate_with_cascade_health(
    validators: &[ValidatorMetrics],
    window: &ConvergenceWindow,
    cascade_health: u32,
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

    // v1.1: cascade health deficit term — increases convergence pressure when health < threshold.
    let health_deficit = CASCADE_HEALTH_THRESHOLD.saturating_sub(cascade_health) as i64;
    let cascade_term = FixedPoint::from_raw(
        health_deficit
            .checked_mul(CASCADE_HEALTH_FACTOR)
            .ok_or(LyapunovError::Overflow)? as i128,
    );

    let v_total = v_sum.checked_add(phi)?.checked_add(cascade_term)?;
    let phi_halt_triggered = phi.raw() >= PHI_MAX_SAFE.raw();

    // IMPORTANT: δ_window is checked against V_CONVERGENCE (v_sum), NOT V_total.
    let (delta_window, halt_triggered) = if window.is_full() {
        let delta = compute_delta_window(v_sum, window)?;
        (delta, delta.raw() > EPSILON.raw())
    } else {
        (FixedPoint::ZERO, false)
    };

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
    fn phi_safety_sums_across_validators() {
        let validators = [
            ValidatorMetrics {
                divergence: FixedPoint::ZERO,
                conflict: FixedPoint::ZERO,
                slash_accum: FixedPoint::from_raw(400_000_000),
            },
            ValidatorMetrics {
                divergence: FixedPoint::ZERO,
                conflict: FixedPoint::ZERO,
                slash_accum: FixedPoint::from_raw(400_000_000),
            },
        ];

        let eval = evaluate(&validators, &ConvergenceWindow::new()).unwrap();

        assert_eq!(eval.phi_safety.raw(), 200_000_000);
        assert!(!eval.phi_halt_triggered);
    }

    #[test]
    fn phi_halt_triggers_at_threshold() {
        let validators = [ValidatorMetrics {
            divergence: FixedPoint::ZERO,
            conflict: FixedPoint::ZERO,
            slash_accum: FixedPoint::from_raw(2_000_000_000),
        }];

        let eval = evaluate(&validators, &ConvergenceWindow::new()).unwrap();

        assert_eq!(eval.phi_safety, PHI_MAX_SAFE);
        assert!(eval.phi_halt_triggered);
    }

    #[test]
    fn delta_window_equal_epsilon_does_not_halt() {
        let validators = [ValidatorMetrics {
            divergence: FixedPoint::from_raw(50_000),
            conflict: FixedPoint::ZERO,
            slash_accum: FixedPoint::ZERO,
        }];
        let mut window = ConvergenceWindow::new();
        window.push(FixedPoint::from_raw(0));
        window.push(FixedPoint::from_raw(0));
        window.push(FixedPoint::ZERO);

        let eval = evaluate(&validators, &window).unwrap();

        assert_eq!(eval.v_convergence.raw(), 20_000);
        assert_eq!(eval.delta_window, EPSILON);
        assert!(!eval.halt_triggered);
    }

    #[test]
    fn delta_window_epsilon_plus_one_halts() {
        let validators = [ValidatorMetrics {
            divergence: FixedPoint::from_raw(50_003),
            conflict: FixedPoint::ZERO,
            slash_accum: FixedPoint::ZERO,
        }];
        let mut window = ConvergenceWindow::new();
        window.push(FixedPoint::from_raw(0));
        window.push(FixedPoint::from_raw(0));
        window.push(FixedPoint::ZERO);

        let eval = evaluate(&validators, &window).unwrap();

        assert_eq!(eval.v_convergence.raw(), 20_001);
        assert_eq!(eval.delta_window.raw(), EPSILON.raw() + 1);
        assert!(eval.halt_triggered);
    }

    #[test]
    fn phi_halt_triggers_one_over_threshold() {
        let validators = [ValidatorMetrics {
            divergence: FixedPoint::ZERO,
            conflict: FixedPoint::ZERO,
            slash_accum: FixedPoint::from_raw(2_000_000_004),
        }];

        let eval = evaluate(&validators, &ConvergenceWindow::new()).unwrap();

        assert_eq!(eval.phi_safety.raw(), PHI_MAX_SAFE.raw() + 1);
        assert!(eval.phi_halt_triggered);
    }

    // ------------------------------------------------------------------
    // 2-D: Cascade health deficit term in Lyapunov potential
    // ------------------------------------------------------------------

    #[test]
    fn lyapunov_pressure_higher_at_health_0_than_health_7() {
        // Same validators, same window — the only difference is cascade_health.
        // health=0: deficit = 8, cascade_term = 8 * 50_000 = 400_000
        // health=7: deficit = 1, cascade_term = 1 * 50_000 = 50_000
        // So v_total at health=0 must exceed v_total at health=7.
        let validators = [ValidatorMetrics::ZERO];
        let window = ConvergenceWindow::new();

        let eval_h0 =
            evaluate_with_cascade_health(&validators, &window, 0).expect("health=0 must succeed");
        let eval_h7 =
            evaluate_with_cascade_health(&validators, &window, 7).expect("health=7 must succeed");

        assert!(
            eval_h0.v_total.raw() > eval_h7.v_total.raw(),
            "v_total at health=0 ({}) must exceed v_total at health=7 ({})",
            eval_h0.v_total.raw(),
            eval_h7.v_total.raw(),
        );
        // Verify exact values: deficit=8 → term=400_000; deficit=1 → term=50_000
        assert_eq!(eval_h0.v_total.raw(), 400_000);
        assert_eq!(eval_h7.v_total.raw(), 50_000);
    }

    #[test]
    fn lyapunov_cascade_term_zero_at_full_health() {
        // At health == CASCADE_HEALTH_THRESHOLD (8), deficit = 0, so cascade_term = 0.
        // v_total must equal v_convergence + phi (no additional term).
        let validators = [ValidatorMetrics::ZERO];
        let window = ConvergenceWindow::new();

        let eval_full =
            evaluate_with_cascade_health(&validators, &window, CASCADE_HEALTH_THRESHOLD)
                .expect("full health must succeed");
        let eval_base =
            evaluate(&validators, &window).expect("base evaluate must succeed");

        // cascade_health=8 ≥ threshold=8: term=0; must equal base evaluate.
        assert_eq!(
            eval_full.v_total.raw(),
            eval_base.v_total.raw(),
            "v_total must be identical to base evaluate at full health"
        );
    }

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
}
