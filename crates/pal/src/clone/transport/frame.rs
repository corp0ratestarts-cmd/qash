// Clone chunk wire frame — spec §10.7.
//
// FORMAT (v1.2):
//   VERSION       u8       = 0x12
//   EPOCH         u64 LE
//   CHUNK_IDX     u16 LE
//   CHUNK_TOTAL   u16 LE
//   COMPRESSED_LEN u16 LE
//   PAYLOAD       [u8; COMPRESSED_LEN]   (LZ4-compressed; max 4096 bytes)
//   SIG           [u8; SIG_BYTES]        (Dilithium5 over all preceding bytes)
//
// SIG_BYTES = 2420 (Dilithium5 NIST level 5 signature size).
// Until the Dilithium5 PAL backend is wired, callers supply a 2420-byte
// placeholder.  The frame format is stable.
//
// Domain B only.

/// v1.2 frame version tag.
pub const FRAME_VERSION: u8 = 0x12;

/// Dilithium5 signature size in bytes.
pub const SIG_BYTES: usize = 2420;

/// Maximum LZ4-compressed payload per chunk (per-chunk, not per-session).
pub const MAX_COMPRESSED_PAYLOAD: usize = 4096;

/// Minimum header size (without payload or sig).
const HEADER_BYTES: usize = 1 + 8 + 2 + 2 + 2; // = 15

/// Parse/build error for clone chunk frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    TooShort,
    BadVersion(u8),
    PayloadTooLarge(usize),
    SigAreaMissing,
    LengthMismatch,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::TooShort => write!(f, "frame too short"),
            FrameError::BadVersion(v) => write!(f, "unknown frame version: 0x{v:02x}"),
            FrameError::PayloadTooLarge(n) => write!(f, "compressed payload too large: {n}"),
            FrameError::SigAreaMissing => write!(f, "signature area missing or truncated"),
            FrameError::LengthMismatch => write!(f, "compressed_len field does not match payload"),
        }
    }
}

/// A parsed clone chunk frame (§10.7).
#[derive(Clone, Debug)]
pub struct ChunkFrame {
    pub epoch: u64,
    pub chunk_idx: u16,
    pub chunk_total: u16,
    /// LZ4-compressed payload bytes.
    pub payload: Vec<u8>,
    /// 2420-byte Dilithium5 signature over header+payload.
    pub sig: Box<[u8; SIG_BYTES]>,
}

impl ChunkFrame {
    /// Build a frame. `payload` must be LZ4-compressed and ≤ MAX_COMPRESSED_PAYLOAD.
    /// `sig` is a caller-supplied 2420-byte Dilithium5 signature; pass zeroes
    /// until the PQC PAL backend is available.
    pub fn new(
        epoch: u64,
        chunk_idx: u16,
        chunk_total: u16,
        payload: Vec<u8>,
        sig: Box<[u8; SIG_BYTES]>,
    ) -> Result<Self, FrameError> {
        if payload.len() > MAX_COMPRESSED_PAYLOAD {
            return Err(FrameError::PayloadTooLarge(payload.len()));
        }
        Ok(Self { epoch, chunk_idx, chunk_total, payload, sig })
    }

    /// Serialise to wire bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES + self.payload.len() + SIG_BYTES);
        out.push(FRAME_VERSION);
        out.extend_from_slice(&self.epoch.to_le_bytes());
        out.extend_from_slice(&self.chunk_idx.to_le_bytes());
        out.extend_from_slice(&self.chunk_total.to_le_bytes());
        out.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        out.extend_from_slice(&self.payload);
        out.extend_from_slice(self.sig.as_ref());
        out
    }

    /// The signable prefix: everything in `to_bytes()` before the signature.
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES + self.payload.len());
        out.push(FRAME_VERSION);
        out.extend_from_slice(&self.epoch.to_le_bytes());
        out.extend_from_slice(&self.chunk_idx.to_le_bytes());
        out.extend_from_slice(&self.chunk_total.to_le_bytes());
        out.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        out.extend_from_slice(&self.payload);
        out
    }

    /// Parse from wire bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() < HEADER_BYTES {
            return Err(FrameError::TooShort);
        }
        let version = bytes[0];
        if version != FRAME_VERSION {
            return Err(FrameError::BadVersion(version));
        }
        let epoch = u64::from_le_bytes(bytes[1..9].try_into().unwrap());
        let chunk_idx = u16::from_le_bytes(bytes[9..11].try_into().unwrap());
        let chunk_total = u16::from_le_bytes(bytes[11..13].try_into().unwrap());
        let compressed_len = u16::from_le_bytes(bytes[13..15].try_into().unwrap()) as usize;

        if compressed_len > MAX_COMPRESSED_PAYLOAD {
            return Err(FrameError::PayloadTooLarge(compressed_len));
        }
        let payload_end = HEADER_BYTES + compressed_len;
        if bytes.len() < payload_end {
            return Err(FrameError::LengthMismatch);
        }
        let payload = bytes[HEADER_BYTES..payload_end].to_vec();

        if bytes.len() < payload_end + SIG_BYTES {
            return Err(FrameError::SigAreaMissing);
        }
        let mut sig = Box::new([0u8; SIG_BYTES]);
        sig.copy_from_slice(&bytes[payload_end..payload_end + SIG_BYTES]);

        Ok(Self { epoch, chunk_idx, chunk_total, payload, sig })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_frame() -> ChunkFrame {
        ChunkFrame::new(42, 0, 3, vec![0xAB; 64], Box::new([0u8; SIG_BYTES])).unwrap()
    }

    #[test]
    fn roundtrip_serialisation() {
        let f = test_frame();
        let bytes = f.to_bytes();
        let parsed = ChunkFrame::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.epoch, f.epoch);
        assert_eq!(parsed.chunk_idx, f.chunk_idx);
        assert_eq!(parsed.chunk_total, f.chunk_total);
        assert_eq!(parsed.payload, f.payload);
        assert_eq!(parsed.sig.as_ref(), f.sig.as_ref());
    }

    #[test]
    fn wire_length_is_header_plus_payload_plus_sig() {
        let payload = vec![0u8; 100];
        let f = ChunkFrame::new(0, 0, 1, payload, Box::new([0u8; SIG_BYTES])).unwrap();
        assert_eq!(f.to_bytes().len(), HEADER_BYTES + 100 + SIG_BYTES);
    }

    #[test]
    fn version_tag_is_correct() {
        let f = test_frame();
        assert_eq!(f.to_bytes()[0], FRAME_VERSION);
    }

    #[test]
    fn from_bytes_rejects_bad_version() {
        let mut bytes = test_frame().to_bytes();
        bytes[0] = 0xFF;
        assert_eq!(ChunkFrame::from_bytes(&bytes).unwrap_err(), FrameError::BadVersion(0xFF));
    }

    #[test]
    fn from_bytes_rejects_truncated() {
        assert_eq!(ChunkFrame::from_bytes(&[0u8; 4]).unwrap_err(), FrameError::TooShort);
    }

    #[test]
    fn from_bytes_rejects_oversized_payload_field() {
        let mut bytes = test_frame().to_bytes();
        // Set compressed_len to max+1.
        let bad = (MAX_COMPRESSED_PAYLOAD as u16 + 1).to_le_bytes();
        bytes[13] = bad[0];
        bytes[14] = bad[1];
        assert_eq!(
            ChunkFrame::from_bytes(&bytes).unwrap_err(),
            FrameError::PayloadTooLarge(MAX_COMPRESSED_PAYLOAD + 1)
        );
    }

    #[test]
    fn new_rejects_oversized_payload() {
        let big = vec![0u8; MAX_COMPRESSED_PAYLOAD + 1];
        assert_eq!(
            ChunkFrame::new(0, 0, 1, big, Box::new([0u8; SIG_BYTES])).unwrap_err(),
            FrameError::PayloadTooLarge(MAX_COMPRESSED_PAYLOAD + 1)
        );
    }

    #[test]
    fn signable_bytes_excludes_sig() {
        let f = test_frame();
        let full = f.to_bytes();
        let signable = f.signable_bytes();
        assert_eq!(signable.len(), full.len() - SIG_BYTES);
        assert_eq!(signable, full[..full.len() - SIG_BYTES]);
    }
}
