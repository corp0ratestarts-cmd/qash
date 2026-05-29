/// Hancke-Kuhn distance-bounding protocol (2005).
///
/// Prevents relay attacks on proximity-based admission channels (NFC/BLE).
/// The prover demonstrates physical proximity by responding to 64 one-bit
/// challenges within a timing bound that limits the relay distance.
///
/// Security: Pr[relay succeeds] ≤ (3/4)^64 ≈ 2^{-26.6} with 64 rounds.
use sha3::{Digest, Sha3_256};

/// Number of challenge-response rounds.
pub const HK_ROUNDS: usize = 64;

/// Maximum allowable round-trip time in nanoseconds (500 ns → ~15 cm relay limit).
pub const HK_TIMING_BOUND_NS: u64 = 500;

/// Error type for distance-bounding failures.
#[derive(Debug, PartialEq, Eq)]
pub enum DistanceBoundingError {
    TimingViolation { round: usize, elapsed_ns: u64 },
    ResponseMismatch { round: usize },
}

impl core::fmt::Display for DistanceBoundingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TimingViolation { round, elapsed_ns } =>
                write!(f, "timing violation at round {round}: {elapsed_ns}ns > {HK_TIMING_BOUND_NS}ns"),
            Self::ResponseMismatch { round } =>
                write!(f, "response mismatch at round {round}"),
        }
    }
}

/// Hancke-Kuhn verifier.
///
/// The verifier holds pre-committed R0/R1 bit arrays derived from the
/// shared key and nonce. It sends challenge bits and checks that each
/// response arrives within `HK_TIMING_BOUND_NS` nanoseconds.
pub struct HanckeKuhnVerifier {
    challenges: [u8; HK_ROUNDS / 8],
    expected_r0: [u8; HK_ROUNDS / 8],
    expected_r1: [u8; HK_ROUNDS / 8],
}

impl HanckeKuhnVerifier {
    /// Initialise from a shared key and nonce.
    ///
    /// R0 and R1 are derived deterministically via SHA3-256.
    pub fn new(shared_key: &[u8; 32], nonce: &[u8; 16], challenges: [u8; HK_ROUNDS / 8]) -> Self {
        let r0 = prf_hk(shared_key, nonce, b"R0");
        let r1 = prf_hk(shared_key, nonce, b"R1");
        Self { challenges, expected_r0: r0, expected_r1: r1 }
    }

    /// Verify a single round response and timing.
    pub fn verify_round(
        &self,
        round: usize,
        response_bit: u8,
        elapsed_ns: u64,
    ) -> Result<(), DistanceBoundingError> {
        if elapsed_ns > HK_TIMING_BOUND_NS {
            return Err(DistanceBoundingError::TimingViolation { round, elapsed_ns });
        }
        let challenge_bit = (self.challenges[round / 8] >> (round % 8)) & 1;
        let expected = if challenge_bit == 0 {
            (self.expected_r0[round / 8] >> (round % 8)) & 1
        } else {
            (self.expected_r1[round / 8] >> (round % 8)) & 1
        };
        if response_bit != expected {
            return Err(DistanceBoundingError::ResponseMismatch { round });
        }
        Ok(())
    }
}

fn prf_hk(key: &[u8; 32], nonce: &[u8; 16], label: &[u8]) -> [u8; HK_ROUNDS / 8] {
    let mut h = Sha3_256::new();
    h.update(key);
    h.update(nonce);
    h.update(label);
    let digest = h.finalize();
    let mut out = [0u8; HK_ROUNDS / 8];
    out.copy_from_slice(&digest[..HK_ROUNDS / 8]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_verifier() -> HanckeKuhnVerifier {
        let key = [0x42u8; 32];
        let nonce = [0x99u8; 16];
        let challenges = [0xAAu8; HK_ROUNDS / 8]; // alternating bits
        HanckeKuhnVerifier::new(&key, &nonce, challenges)
    }

    #[test]
    fn timing_violation_is_detected() {
        let v = make_verifier();
        let result = v.verify_round(0, 0, HK_TIMING_BOUND_NS + 1);
        assert!(matches!(result, Err(DistanceBoundingError::TimingViolation { .. })));
    }

    #[test]
    fn response_mismatch_is_detected() {
        let key = [0x42u8; 32];
        let nonce = [0x99u8; 16];
        let challenges = [0u8; HK_ROUNDS / 8]; // all zeros → must use R0
        let v2 = HanckeKuhnVerifier::new(&key, &nonce, challenges);
        // The correct response for round 0 is (r0[0] >> 0) & 1
        // Pass the inverted bit
        let correct_r0_bit = prf_hk(&key, &nonce, b"R0")[0] & 1;
        let wrong_bit = correct_r0_bit ^ 1;
        let result = v2.verify_round(0, wrong_bit, 0);
        assert!(matches!(result, Err(DistanceBoundingError::ResponseMismatch { round: 0 })));
    }

    #[test]
    fn correct_response_within_timing_bound_succeeds() {
        let key = [0x42u8; 32];
        let nonce = [0x99u8; 16];
        let challenges = [0u8; HK_ROUNDS / 8]; // all zeros → use R0
        let v = HanckeKuhnVerifier::new(&key, &nonce, challenges);
        let correct_bit = prf_hk(&key, &nonce, b"R0")[0] & 1;
        assert!(v.verify_round(0, correct_bit, 100).is_ok());
    }

    #[test]
    fn prf_hk_is_deterministic() {
        let key = [0x11u8; 32];
        let nonce = [0x22u8; 16];
        assert_eq!(prf_hk(&key, &nonce, b"R0"), prf_hk(&key, &nonce, b"R0"));
    }

    #[test]
    fn prf_hk_r0_differs_from_r1() {
        let key = [0x11u8; 32];
        let nonce = [0x22u8; 16];
        assert_ne!(prf_hk(&key, &nonce, b"R0"), prf_hk(&key, &nonce, b"R1"));
    }
}
