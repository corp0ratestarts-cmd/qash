// Coq extraction interface for H_cascade (TH-9, TH-10, TH-11).
//
// This module defines the types and invariants that the Coq proofs in
// proofs/cascade/ reason about. It is NOT the executable cascade —
// see cascade.rs for that.
//
// When the Coq proofs are extracted to Rust, the extracted code should
// be structurally equivalent to cascade.rs. Any divergence is a proof gap.
//
// STATUS: STUB — extraction not yet wired.

/// Cascade health factor: CH_t ∈ [0, p].
/// Invariant: CascadeHealthFactor.value <= 1_000_000
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CascadeHealthFactor {
    pub value: i64,
}

impl CascadeHealthFactor {
    pub const P: i64 = 1_000_000;
    pub const ZERO: Self = Self { value: 0 };

    /// Compute CH_t from fail count and max queries.
    /// Panics in debug if invariant is violated (should never happen in Domain A).
    pub fn compute(cascade_fail_count: i64, max_queries_per_epoch: i64) -> Self {
        debug_assert!(cascade_fail_count >= 0);
        debug_assert!(cascade_fail_count <= max_queries_per_epoch);
        debug_assert!(max_queries_per_epoch > 0);
        let value = cascade_fail_count * Self::P / max_queries_per_epoch;
        debug_assert!((0..=Self::P).contains(&value));
        Self { value }
    }

    /// χ · CH_t term for V_convergence, computed in i128 to avoid overflow.
    pub fn weighted_term(self, chi: i128) -> i128 {
        chi * (self.value as i128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ch_zero_when_no_failures() {
        let ch = CascadeHealthFactor::compute(0, 1_000_000);
        assert_eq!(ch.value, 0);
    }

    #[test]
    fn ch_p_when_all_fail() {
        let ch = CascadeHealthFactor::compute(1_000_000, 1_000_000);
        assert_eq!(ch.value, CascadeHealthFactor::P);
    }

    #[test]
    fn weighted_term_no_overflow() {
        let ch = CascadeHealthFactor {
            value: CascadeHealthFactor::P,
        };
        let chi: i128 = 150_000;
        // 150_000 * 1_000_000 = 1.5e11, well within i128
        assert_eq!(ch.weighted_term(chi), 150_000_000_000i128);
    }
}
