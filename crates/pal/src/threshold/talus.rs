/// TALUS-style threshold signing stub.
///
/// In production, this implements t-of-n threshold ML-DSA where no single
/// key holder ever sees the full signing key. This stub provides the type
/// scaffolding and interface. Full MPC implementation requires a secure
/// channel between signers.
///
/// Only active with `--features threshold-signing`.

/// Error type for threshold signing operations.
#[cfg(feature = "threshold-signing")]
#[derive(Debug, PartialEq, Eq)]
pub enum ThresholdError {
    InsufficientShares { got: usize, need: usize },
    InvalidShare,
    CombinedSignatureInvalid,
    Timeout,
}

#[cfg(feature = "threshold-signing")]
impl core::fmt::Display for ThresholdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InsufficientShares { got, need } =>
                write!(f, "insufficient shares: got {got}, need {need}"),
            Self::InvalidShare => write!(f, "invalid signature share"),
            Self::CombinedSignatureInvalid => write!(f, "combined signature failed verification"),
            Self::Timeout => write!(f, "threshold signing timed out"),
        }
    }
}

/// A partial signature share contributed by one of n key holders.
#[cfg(feature = "threshold-signing")]
#[derive(Clone)]
pub struct SignatureShare {
    pub holder_index: usize,
    pub share_bytes: Vec<u8>,
}

/// Threshold signer holding one share of a t-of-n key.
#[cfg(feature = "threshold-signing")]
pub struct ThresholdSigner {
    pub threshold: usize,
    pub total_holders: usize,
    pub holder_index: usize,
}

#[cfg(feature = "threshold-signing")]
impl ThresholdSigner {
    pub fn new(threshold: usize, total_holders: usize, holder_index: usize) -> Self {
        Self { threshold, total_holders, holder_index }
    }

    /// Generate a stub partial signature over msg.
    pub fn sign_share(&self, msg: &[u8]) -> SignatureShare {
        use sha3::{Digest, Sha3_256};
        let mut h = Sha3_256::new();
        h.update(&[self.holder_index as u8]);
        h.update(msg);
        SignatureShare {
            holder_index: self.holder_index,
            share_bytes: h.finalize().to_vec(),
        }
    }

    /// Combine t-of-n shares into a full signature.
    /// Returns Err if fewer than threshold shares provided.
    pub fn combine_shares(
        &self,
        shares: &[SignatureShare],
        _msg: &[u8],
    ) -> Result<Vec<u8>, ThresholdError> {
        if shares.len() < self.threshold {
            return Err(ThresholdError::InsufficientShares {
                got: shares.len(),
                need: self.threshold,
            });
        }
        // Stub: XOR all shares together as a placeholder combiner
        let mut combined = vec![0u8; 32];
        for share in shares {
            for (i, b) in share.share_bytes.iter().enumerate().take(32) {
                combined[i] ^= b;
            }
        }
        Ok(combined)
    }
}

#[cfg(all(test, feature = "threshold-signing"))]
mod tests {
    use super::*;

    #[test]
    fn insufficient_shares_returns_error() {
        let signer = ThresholdSigner::new(3, 5, 0);
        let shares = vec![signer.sign_share(b"msg")];
        assert_eq!(
            signer.combine_shares(&shares, b"msg"),
            Err(ThresholdError::InsufficientShares { got: 1, need: 3 })
        );
    }

    #[test]
    fn sufficient_shares_returns_ok() {
        let signer = ThresholdSigner::new(2, 3, 0);
        let s1 = signer.sign_share(b"hello");
        let s2 = ThresholdSigner::new(2, 3, 1).sign_share(b"hello");
        assert!(signer.combine_shares(&[s1, s2], b"hello").is_ok());
    }

    #[test]
    fn threshold_error_displays() {
        assert!(!ThresholdError::Timeout.to_string().is_empty());
        assert!(!ThresholdError::InvalidShare.to_string().is_empty());
    }
}
