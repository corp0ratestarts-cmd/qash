//! Public observation transcript — the only Domain A artifacts
//! visible to external passive observers.
//!
//! `RawTx`, `Tx` envelopes, `ValidatorMetrics`, and `ConvergenceWindow`
//! are intentionally absent: they are Domain B admission artifacts and
//! MUST NOT appear in `PublicTranscript`.
//!
//! Constructed ONLY by epoch advancement after Lyapunov validation and cascade
//! or EFB verification.  Callers in Domain A MUST NOT fabricate or modify
//! `PublicTranscript` instances outside the authorized construction path.
//! This struct is a read-only view of the epoch's public commitment surface.

/// Canonical wire length of a serialised `PublicTranscript`:
/// 32 (state_root) + 32 (receipt_root) + 32 (efb_root) + 8 (epoch) + 1 (halt_flag).
pub const PUBLIC_TRANSCRIPT_WIRE_LEN: usize = 105;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicTranscript {
    pub state_root: [u8; 32],
    pub receipt_root: [u8; 32],
    pub efb_root: [u8; 32],
    pub epoch: u64,
    pub halt_flag: bool,
}

impl PublicTranscript {
    /// Deterministic canonical encoding for Domain B broadcast.
    ///
    /// Format (105 bytes, all fields big-endian):
    ///   state_root[32] | receipt_root[32] | efb_root[32] | epoch[8] | halt_flag[1]
    ///
    /// This is the ONLY encoding that Domain B may broadcast to public channels.
    /// Do not serialize `EpochState` directly — use this method.
    #[inline]
    pub fn encode_canonical(&self) -> [u8; PUBLIC_TRANSCRIPT_WIRE_LEN] {
        let mut out = [0u8; PUBLIC_TRANSCRIPT_WIRE_LEN];
        out[0..32].copy_from_slice(&self.state_root);
        out[32..64].copy_from_slice(&self.receipt_root);
        out[64..96].copy_from_slice(&self.efb_root);
        out[96..104].copy_from_slice(&self.epoch.to_be_bytes());
        out[104] = self.halt_flag as u8;
        out
    }

    /// Decode from canonical wire format.
    ///
    /// Returns `None` if `bytes` is not exactly `PUBLIC_TRANSCRIPT_WIRE_LEN`.
    #[inline]
    pub fn decode_canonical(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != PUBLIC_TRANSCRIPT_WIRE_LEN {
            return None;
        }
        let mut state_root = [0u8; 32];
        let mut receipt_root = [0u8; 32];
        let mut efb_root = [0u8; 32];
        state_root.copy_from_slice(&bytes[0..32]);
        receipt_root.copy_from_slice(&bytes[32..64]);
        efb_root.copy_from_slice(&bytes[64..96]);
        let epoch = u64::from_be_bytes(bytes[96..104].try_into().ok()?);
        let halt_flag = bytes[104] != 0;
        Some(Self {
            state_root,
            receipt_root,
            efb_root,
            epoch,
            halt_flag,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PublicTranscript {
        PublicTranscript {
            state_root: [0x11u8; 32],
            receipt_root: [0x22u8; 32],
            efb_root: [0x33u8; 32],
            epoch: 0x0102_0304_0506_0708u64,
            halt_flag: true,
        }
    }

    #[test]
    fn encode_canonical_length() {
        assert_eq!(
            sample().encode_canonical().len(),
            PUBLIC_TRANSCRIPT_WIRE_LEN
        );
    }

    #[test]
    fn encode_decode_roundtrip() {
        let pt = sample();
        let encoded = pt.encode_canonical();
        let decoded = PublicTranscript::decode_canonical(&encoded).expect("decode ok");
        assert_eq!(pt, decoded);
    }

    #[test]
    fn decode_canonical_rejects_wrong_length() {
        assert!(PublicTranscript::decode_canonical(&[0u8; 104]).is_none());
        assert!(PublicTranscript::decode_canonical(&[0u8; 106]).is_none());
        assert!(PublicTranscript::decode_canonical(&[]).is_none());
    }

    #[test]
    fn halt_flag_encoding() {
        let mut pt = sample();
        pt.halt_flag = false;
        assert_eq!(pt.encode_canonical()[104], 0x00);
        pt.halt_flag = true;
        assert_eq!(pt.encode_canonical()[104], 0x01);
    }

    #[test]
    fn epoch_encoding_big_endian() {
        let mut pt = sample();
        pt.epoch = 1;
        let enc = pt.encode_canonical();
        assert_eq!(&enc[96..104], &[0, 0, 0, 0, 0, 0, 0, 1]);
    }
}
