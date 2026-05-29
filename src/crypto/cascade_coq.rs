// Coq extraction interface for H_cascade (TH-9, TH-10, TH-11).
//
// This module defines the types and invariants that the Coq proofs in
// proofs/cascade/ reason about. It is NOT the executable cascade —
// see cascade.rs for that.
//
// Coq correspondence (proofs/cascade/cascade_health_bounded.v):
//   Lemma ch_t_upper_bound     → CascadeHealthFactor::compute() post-condition
//   Lemma cascade_health_term_no_overflow → CascadeHealthFactor::weighted_term()
//   Lemma ch_term_admissible   → CascadeHealthFactor::chi_term()
//
// The Coq proof uses: p = 1_000_000, chi = 150_000, max_queries = 1_000_000.
// These constants are reproduced below and exercised in the correspondence tests.

/// Fixed-point scale p = 1_000_000 (GENESIS_CONSTANTS.toml fixed_point_scale).
/// Coq: `Definition p : Z := 1_000_000.`
pub const P: i64 = 1_000_000;

/// Lyapunov weight χ for the cascade health term (GENESIS_CONSTANTS.toml cascade_health_factor).
/// Coq: `Definition chi : Z := 150_000.`
pub const CHI: i128 = 150_000;

/// Cascade health factor: CH_t ∈ [0, p].
/// Invariant: CascadeHealthFactor.value <= 1_000_000
///
/// Proved in proofs/cascade/cascade_health_bounded.v:
///   ch_t_upper_bound: ∀ fail_count ∈ [0, max_queries], CH_t ∈ [0, p]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CascadeHealthFactor {
    pub value: i64,
}

impl CascadeHealthFactor {
    pub const ZERO: Self = Self { value: 0 };
    pub const MAX: Self = Self { value: P };

    /// Compute CH_t = fail_count × p / max_queries.
    ///
    /// Pre:  0 ≤ cascade_fail_count ≤ max_queries_per_epoch, max_queries_per_epoch > 0
    /// Post: value ∈ [0, P]  (proved: ch_t_upper_bound in cascade_health_bounded.v)
    pub fn compute(cascade_fail_count: i64, max_queries_per_epoch: i64) -> Self {
        debug_assert!(cascade_fail_count >= 0);
        debug_assert!(cascade_fail_count <= max_queries_per_epoch);
        debug_assert!(max_queries_per_epoch > 0);
        let value = cascade_fail_count * P / max_queries_per_epoch;
        debug_assert!((0..=P).contains(&value));
        Self { value }
    }

    /// χ · CH_t term for V_convergence, computed in i128 to avoid overflow.
    ///
    /// For arbitrary χ — use `chi_term()` for the genesis-canonical CHI weight.
    pub fn weighted_term(self, chi: i128) -> i128 {
        chi * (self.value as i128)
    }

    /// Canonical Lyapunov term: CHI · CH_t.
    ///
    /// Proved non-overflowing in cascade_health_bounded.v:
    ///   cascade_health_term_no_overflow: ∀ ch_t ∈ [0, p], CHI·ch_t ∈ [0, CHI·p]
    ///   CHI·p = 150_000 × 1_000_000 = 1.5×10¹¹ << i128::MAX
    pub fn chi_term(self) -> i128 {
        self.weighted_term(CHI)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ch_t_upper_bound correspondence (cascade_health_bounded.v) ---

    #[test]
    fn ch_zero_when_no_failures() {
        // Coq: ch_t_upper_bound with fail_count = 0 → CH_t = 0
        let ch = CascadeHealthFactor::compute(0, 1_000_000);
        assert_eq!(ch.value, 0);
    }

    #[test]
    fn ch_p_when_all_fail() {
        // Coq: ch_t_upper_bound with fail_count = max_queries → CH_t = p
        let ch = CascadeHealthFactor::compute(1_000_000, 1_000_000);
        assert_eq!(ch, CascadeHealthFactor::MAX);
        assert_eq!(ch.value, P);
    }

    #[test]
    fn ch_t_stays_in_range_partial_failure() {
        // Coq: ch_t_upper_bound for all fail_count ∈ [0, max_queries]
        let ch = CascadeHealthFactor::compute(500_000, 1_000_000);
        assert!(ch.value >= 0);
        assert!(ch.value <= P);
    }

    // --- cascade_health_term_no_overflow correspondence ---

    #[test]
    fn weighted_term_no_overflow_at_max() {
        // Coq: cascade_health_term_no_overflow with ch_t = p, chi = 150_000
        // CHI * P = 150_000 * 1_000_000 = 1.5×10¹¹, well within i128::MAX
        let ch = CascadeHealthFactor::MAX;
        assert_eq!(ch.weighted_term(CHI), 150_000_000_000i128);
    }

    #[test]
    fn weighted_term_zero_at_zero() {
        // Coq: cascade_health_term_no_overflow with ch_t = 0 → chi * 0 = 0
        assert_eq!(CascadeHealthFactor::ZERO.weighted_term(CHI), 0);
    }

    // --- ch_term_admissible correspondence (combined lemma) ---

    #[test]
    fn chi_term_at_max_matches_proof_constant() {
        // Coq: ch_term_admissible: CHI * (max_queries * p / max_queries) = CHI * p
        // = 150_000 * 1_000_000 = 150_000_000_000
        let ch = CascadeHealthFactor::compute(1_000_000, 1_000_000);
        assert_eq!(ch.chi_term(), CHI * (P as i128));
        assert_eq!(ch.chi_term(), 150_000_000_000i128);
    }

    #[test]
    fn chi_term_in_range() {
        for fail in [0i64, 250_000, 500_000, 750_000, 1_000_000] {
            let ch = CascadeHealthFactor::compute(fail, 1_000_000);
            let term = ch.chi_term();
            assert!(term >= 0, "chi_term negative at fail={fail}");
            assert!(
                term <= CHI * (P as i128),
                "chi_term overflow at fail={fail}"
            );
        }
    }
}
