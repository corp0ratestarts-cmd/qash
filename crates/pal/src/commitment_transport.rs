//! Fixed-size commitment frame encoding for zero-persistence Domain B.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitmentFrame {
    pub epoch: u64,
    pub state_root: [u8; 32],
    pub receipt_root: [u8; 32],
    pub efb_root: [u8; 32],
    pub evidence_root: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitmentFrameError {
    InvalidLength,
    InvalidMagic,
}

pub const COMMITMENT_FRAME_MAGIC: [u8; 8] = *b"QPCOMM1\0";
pub const COMMITMENT_FRAME_BYTES: usize = 8 + 8 + 32 + 32 + 32 + 32;

impl CommitmentFrame {
    pub fn encode(&self) -> [u8; COMMITMENT_FRAME_BYTES] {
        let mut out = [0u8; COMMITMENT_FRAME_BYTES];
        let mut pos = 0;
        out[pos..pos + 8].copy_from_slice(&COMMITMENT_FRAME_MAGIC);
        pos += 8;
        out[pos..pos + 8].copy_from_slice(&self.epoch.to_le_bytes());
        pos += 8;
        out[pos..pos + 32].copy_from_slice(&self.state_root);
        pos += 32;
        out[pos..pos + 32].copy_from_slice(&self.receipt_root);
        pos += 32;
        out[pos..pos + 32].copy_from_slice(&self.efb_root);
        pos += 32;
        out[pos..pos + 32].copy_from_slice(&self.evidence_root);
        out
    }

    pub fn decode(input: &[u8]) -> Result<Self, CommitmentFrameError> {
        if input.len() != COMMITMENT_FRAME_BYTES {
            return Err(CommitmentFrameError::InvalidLength);
        }
        if input[..8] != COMMITMENT_FRAME_MAGIC {
            return Err(CommitmentFrameError::InvalidMagic);
        }
        let mut pos = 8;
        let epoch = read_u64(input, &mut pos);
        let state_root = read_root(input, &mut pos);
        let receipt_root = read_root(input, &mut pos);
        let efb_root = read_root(input, &mut pos);
        let evidence_root = read_root(input, &mut pos);
        Ok(Self { epoch, state_root, receipt_root, efb_root, evidence_root })
    }
}

fn read_u64(input: &[u8], pos: &mut usize) -> u64 {
    let mut out = [0u8; 8];
    out.copy_from_slice(&input[*pos..*pos + 8]);
    *pos += 8;
    u64::from_le_bytes(out)
}

fn read_root(input: &[u8], pos: &mut usize) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&input[*pos..*pos + 32]);
    *pos += 32;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commitment_frame_round_trips() {
        let frame = CommitmentFrame {
            epoch: 99,
            state_root: [1u8; 32],
            receipt_root: [2u8; 32],
            efb_root: [3u8; 32],
            evidence_root: [4u8; 32],
        };
        assert_eq!(CommitmentFrame::decode(&frame.encode()).unwrap(), frame);
    }
}
