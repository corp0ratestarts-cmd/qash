//! Public observation transcript — the only Domain A artifacts visible to
//! external passive observers.
//!
//! Raw transactions, validator metrics, and routing metadata are intentionally
//! absent. This struct is a read-only view of the epoch's public commitment
//! surface.

/// Canonical wire length of a serialised PublicTranscript:
/// 32 (state_root) + 32 (receipt_root) + 32 (efb_root) + 8 (epoch) + 1 (halt_flag) = 105 bytes.
pub const PUBLIC_TRANSCRIPT_WIRE_LEN: usize = 105;

/// The public commitment surface for one epoch.
///
/// This is the ONLY artifact that Domain B may broadcast to public channels.
/// Callers MUST NOT fabricate PublicTranscript values outside the authorized
/// construction path (after Lyapunov validation and cascade or EFB verification).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicTranscript {
    pub state_root:   [u8; 32],
    pub receipt_root: [u8; 32],
    pub efb_root:     [u8; 32],
    pub epoch:        u64,
    pub halt_flag:    bool,
}

impl PublicTranscript {
    /// Deterministic canonical encoding for Domain B broadcast.
    ///
    /// Format (105 bytes, fields big-endian):
    ///   state_root[32] | receipt_root[32] | efb_root[32] | epoch[8 BE] | halt_flag[1]
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

    /// Decode from canonical wire format. Returns None if length != 105.
    #[inline]
    pub fn decode_canonical(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != PUBLIC_TRANSCRIPT_WIRE_LEN {
            return None;
        }
        let mut state_root   = [0u8; 32];
        let mut receipt_root = [0u8; 32];
        let mut efb_root     = [0u8; 32];
        state_root.copy_from_slice(&bytes[0..32]);
        receipt_root.copy_from_slice(&bytes[32..64]);
        efb_root.copy_from_slice(&bytes[64..96]);
        let epoch     = u64::from_be_bytes(bytes[96..104].try_into().ok()?);
        let halt_flag = bytes[104] != 0;
        Some(Self { state_root, receipt_root, efb_root, epoch, halt_flag })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PublicTranscript {
        PublicTranscript {
            state_root:   [0x11u8; 32],
            receipt_root: [0x22u8; 32],
            efb_root:     [0x33u8; 32],
            epoch:        0x0102_0304_0506_0708u64,
            halt_flag:    false,
        }
    }

    #[test]
    fn public_transcript_encode_decode_roundtrip() {
        let t = sample();
        let enc = t.encode_canonical();
        assert_eq!(enc.len(), PUBLIC_TRANSCRIPT_WIRE_LEN);
        let dec = PublicTranscript::decode_canonical(&enc).unwrap();
        assert_eq!(dec, t);
    }

    #[test]
    fn public_transcript_wrong_length_returns_none() {
        assert!(PublicTranscript::decode_canonical(&[0u8; 104]).is_none());
        assert!(PublicTranscript::decode_canonical(&[0u8; 106]).is_none());
    }

    #[test]
    fn public_transcript_halt_flag_encodes_correctly() {
        let mut t = sample();
        t.halt_flag = true;
        let enc = t.encode_canonical();
        assert_eq!(enc[104], 1);
        let dec = PublicTranscript::decode_canonical(&enc).unwrap();
        assert!(dec.halt_flag);
    }

    #[test]
    fn public_transcript_epoch_big_endian() {
        let t = sample();
        let enc = t.encode_canonical();
        // epoch 0x0102_0304_0506_0708 in big-endian at bytes 96-104
        assert_eq!(&enc[96..104], &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    }

    #[test]
    fn public_transcript_contains_no_graph_fields() {
        // Structural test: PublicTranscript has no Vec, String, or SocketAddr.
        // This is enforced by the type definition; this test documents the intent.
        let t = sample();
        let enc = t.encode_canonical();
        assert_eq!(enc.len(), PUBLIC_TRANSCRIPT_WIRE_LEN);
    }
}
