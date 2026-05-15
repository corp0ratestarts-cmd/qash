// Cascade algorithm agility schedule (spec §3.5 / GENESIS_CONSTANTS.toml
// [crypto.cascade.rotation_schedule]).
//
// This is a pure, deterministic function of epoch — Domain A safe.
// Domain separators and hash composition remain fixed; only the
// outer signature algorithm rotates.

/// Returns the primary signature algorithm name for the given epoch.
///
/// Epoch schedule (from GENESIS_CONSTANTS.toml rotation_schedule):
///   epoch 0     → Dilithium5
///   epoch 10000 → ML-DSA-87
///   epoch 20000 → SLH-DSA-SHA3-256
///   epoch 30000 → Falcon-512
///   epoch 40000 → TERMINAL_HALT
pub fn select_cascade_config(epoch: u64) -> &'static str {
    if epoch >= 40_000 {
        "TERMINAL_HALT"
    } else if epoch >= 30_000 {
        "Falcon-512"
    } else if epoch >= 20_000 {
        "SLH-DSA-SHA3-256"
    } else if epoch >= 10_000 {
        "ML-DSA-87"
    } else {
        "Dilithium5"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_boundaries() {
        assert_eq!(select_cascade_config(0),       "Dilithium5");
        assert_eq!(select_cascade_config(9_999),   "Dilithium5");
        assert_eq!(select_cascade_config(10_000),  "ML-DSA-87");
        assert_eq!(select_cascade_config(19_999),  "ML-DSA-87");
        assert_eq!(select_cascade_config(20_000),  "SLH-DSA-SHA3-256");
        assert_eq!(select_cascade_config(29_999),  "SLH-DSA-SHA3-256");
        assert_eq!(select_cascade_config(30_000),  "Falcon-512");
        assert_eq!(select_cascade_config(39_999),  "Falcon-512");
        assert_eq!(select_cascade_config(40_000),  "TERMINAL_HALT");
        assert_eq!(select_cascade_config(u64::MAX),"TERMINAL_HALT");
    }
}
