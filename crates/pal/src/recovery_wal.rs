//! Crash-safe fixed-record recovery WAL for zero-persistence Domain B.

use crate::zero_wal::ZeroPersistenceWalRecord;

pub const RECOVERY_WAL_MAGIC: [u8; 8] = *b"QPZWAL1\0";
pub const RECOVERY_RECORD_BYTES: usize = 1 + 7 + 8 + 32 + 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryWalError {
    InvalidMagic,
    InvalidLength,
    InvalidTag,
    Io,
}

pub fn encode_record(record: ZeroPersistenceWalRecord) -> [u8; RECOVERY_RECORD_BYTES] {
    let mut out = [0u8; RECOVERY_RECORD_BYTES];
    match record {
        ZeroPersistenceWalRecord::EffectCommitment { epoch, effect_root, receipt_root } => {
            out[0] = 1;
            write_u64(&mut out, 8, epoch);
            write_root(&mut out, 16, effect_root);
            write_root(&mut out, 48, receipt_root);
        }
        ZeroPersistenceWalRecord::StateRoot { epoch, state_root } => {
            out[0] = 2;
            write_u64(&mut out, 8, epoch);
            write_root(&mut out, 16, state_root);
        }
        ZeroPersistenceWalRecord::BlindAudit { epoch, event_root } => {
            out[0] = 3;
            write_u64(&mut out, 8, epoch);
            write_root(&mut out, 16, event_root);
        }
        ZeroPersistenceWalRecord::ShredCommitment { epoch, key_id_commitment, event_root } => {
            out[0] = 4;
            write_u64(&mut out, 8, epoch);
            write_root(&mut out, 16, key_id_commitment);
            write_root(&mut out, 48, event_root);
        }
    }
    out
}

pub fn decode_record(input: &[u8]) -> Result<ZeroPersistenceWalRecord, RecoveryWalError> {
    if input.len() != RECOVERY_RECORD_BYTES {
        return Err(RecoveryWalError::InvalidLength);
    }
    let tag = input[0];
    let mut pos = 8;
    let epoch = read_u64(input, &mut pos);
    let a = read_root(input, &mut pos);
    let b = read_root(input, &mut pos);
    match tag {
        1 => Ok(ZeroPersistenceWalRecord::EffectCommitment { epoch, effect_root: a, receipt_root: b }),
        2 => Ok(ZeroPersistenceWalRecord::StateRoot { epoch, state_root: a }),
        3 => Ok(ZeroPersistenceWalRecord::BlindAudit { epoch, event_root: a }),
        4 => Ok(ZeroPersistenceWalRecord::ShredCommitment { epoch, key_id_commitment: a, event_root: b }),
        _ => Err(RecoveryWalError::InvalidTag),
    }
}

fn write_u64(out: &mut [u8; RECOVERY_RECORD_BYTES], pos: usize, value: u64) {
    out[pos..pos + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_root(out: &mut [u8; RECOVERY_RECORD_BYTES], pos: usize, value: [u8; 32]) {
    out[pos..pos + 32].copy_from_slice(&value);
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

#[cfg(feature = "std")]
pub struct FileRecoveryWal {
    path: std::path::PathBuf,
}

#[cfg(feature = "std")]
impl FileRecoveryWal {
    pub fn open(path: impl Into<std::path::PathBuf>) -> Result<Self, RecoveryWalError> {
        let path = path.into();
        if !path.exists() {
            let mut file = std::fs::OpenOptions::new().create_new(true).write(true).open(&path).map_err(|_| RecoveryWalError::Io)?;
            use std::io::Write;
            file.write_all(&RECOVERY_WAL_MAGIC).map_err(|_| RecoveryWalError::Io)?;
            file.sync_all().map_err(|_| RecoveryWalError::Io)?;
        }
        Ok(Self { path })
    }

    pub fn append_synced(&self, record: ZeroPersistenceWalRecord) -> Result<(), RecoveryWalError> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(&self.path).map_err(|_| RecoveryWalError::Io)?;
        file.write_all(&encode_record(record)).map_err(|_| RecoveryWalError::Io)?;
        file.sync_all().map_err(|_| RecoveryWalError::Io)?;
        Ok(())
    }

    pub fn replay(&self) -> Result<Vec<ZeroPersistenceWalRecord>, RecoveryWalError> {
        use std::io::Read;
        let mut file = std::fs::File::open(&self.path).map_err(|_| RecoveryWalError::Io)?;
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic).map_err(|_| RecoveryWalError::Io)?;
        if magic != RECOVERY_WAL_MAGIC {
            return Err(RecoveryWalError::InvalidMagic);
        }
        let mut out = Vec::new();
        loop {
            let mut record = [0u8; RECOVERY_RECORD_BYTES];
            match file.read_exact(&mut record) {
                Ok(()) => out.push(decode_record(&record)?),
                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(_) => return Err(RecoveryWalError::Io),
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_round_trips() {
        let record = ZeroPersistenceWalRecord::ShredCommitment { epoch: 3, key_id_commitment: [4u8; 32], event_root: [5u8; 32] };
        assert_eq!(decode_record(&encode_record(record)).unwrap(), record);
    }
}
