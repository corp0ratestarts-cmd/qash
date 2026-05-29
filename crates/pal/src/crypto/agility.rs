// PQC crypto-agility epoch gate — GENESIS_CONSTANTS.toml [crypto.cascade].
//
// At epoch >= PQC_AGILITY_EPOCH (10000) the cascade primary signature
// algorithm migrates from Dilithium5 to SLH-DSA-SHA3-256 (anchor), with
// Falcon-512 as the fallback.  Below the threshold the primary remains active.
//
// This module handles suite-selection only.  Actual signing/verification is
// done by the PAL backend (stub until the Dilithium5/SLH-DSA drivers are wired).
// Domain B only — do not import from qash-consensus.

/// Epoch at which the PQC algorithm migration activates.
///
/// Matches `pqc_agility_epoch = 10000` in GENESIS_CONSTANTS.toml.
pub const PQC_AGILITY_EPOCH: u64 = 10000;

/// Active post-quantum signature suite for a given epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigSuite {
    /// Dilithium5 (NIST ML-DSA level 5) — active below the agility epoch.
    Dilithium5,
    /// SLH-DSA-SHA3-256 (NIST SLH-DSA, stateless hash-based) — anchor suite,
    /// active at and above the agility epoch.
    SlhDsaSha3_256,
    /// Falcon-512 — fallback suite; selected when the primary is unavailable.
    Falcon512,
}

impl SigSuite {
    /// Return the primary suite active at `epoch`.
    ///
    /// Below `PQC_AGILITY_EPOCH`: Dilithium5.
    /// At or above `PQC_AGILITY_EPOCH`: SLH-DSA-SHA3-256 (anchor).
    pub fn primary_for_epoch(epoch: u64) -> Self {
        if epoch < PQC_AGILITY_EPOCH {
            Self::Dilithium5
        } else {
            Self::SlhDsaSha3_256
        }
    }

    /// Return the fallback suite (epoch-invariant).
    pub fn fallback() -> Self {
        Self::Falcon512
    }

    /// Human-readable algorithm identifier (matches GENESIS_CONSTANTS.toml value).
    pub fn name(self) -> &'static str {
        match self {
            Self::Dilithium5 => "Dilithium5",
            Self::SlhDsaSha3_256 => "SLH-DSA-SHA3-256",
            Self::Falcon512 => "Falcon-512",
        }
    }

    /// Whether the agility migration has activated for this epoch.
    pub fn migration_active(epoch: u64) -> bool {
        epoch >= PQC_AGILITY_EPOCH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_threshold_uses_dilithium5() {
        assert_eq!(SigSuite::primary_for_epoch(0), SigSuite::Dilithium5);
        assert_eq!(SigSuite::primary_for_epoch(9999), SigSuite::Dilithium5);
    }

    #[test]
    fn at_threshold_migrates_to_slh_dsa() {
        assert_eq!(SigSuite::primary_for_epoch(10000), SigSuite::SlhDsaSha3_256);
        assert_eq!(SigSuite::primary_for_epoch(99999), SigSuite::SlhDsaSha3_256);
    }

    #[test]
    fn fallback_is_epoch_invariant() {
        assert_eq!(SigSuite::fallback(), SigSuite::Falcon512);
    }

    #[test]
    fn migration_active_flag() {
        assert!(!SigSuite::migration_active(9999));
        assert!(SigSuite::migration_active(10000));
        assert!(SigSuite::migration_active(10001));
    }

    #[test]
    fn names_match_genesis_constants() {
        assert_eq!(SigSuite::Dilithium5.name(), "Dilithium5");
        assert_eq!(SigSuite::SlhDsaSha3_256.name(), "SLH-DSA-SHA3-256");
        assert_eq!(SigSuite::Falcon512.name(), "Falcon-512");
    }

    #[test]
    fn agility_epoch_constant_matches_spec() {
        assert_eq!(PQC_AGILITY_EPOCH, 10000);
    }
}
