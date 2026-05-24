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

/// Policy for handling a truncated tail in a WAL file.
///
/// A truncated tail means the file ends mid-record. This can happen after a
/// crash during `append_synced`. The policy determines whether to:
/// - `IgnoreTruncatedTail`: discard the incomplete record and return what was
///   successfully decoded. Safe when the caller will re-sync from peers.
/// - `RejectTruncatedTail`: treat any incomplete tail as corruption and return
///   `RecoveryWalError::InvalidLength`. Required when the WAL is the sole
///   source of truth and truncation must be manually investigated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncationPolicy {
    IgnoreTruncatedTail,
    RejectTruncatedTail,
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

    /// Replay all valid WAL records. Applies `policy` to handle a truncated tail.
    ///
    /// Errors immediately on:
    /// - Invalid magic header (`RecoveryWalError::InvalidMagic`)
    /// - Invalid record tag mid-stream (`RecoveryWalError::InvalidTag`)
    /// - I/O failure that is not a clean EOF (`RecoveryWalError::Io`)
    ///
    /// A truncated tail (partial record at the end) is handled per `policy`.
    pub fn replay_with_policy(
        &self,
        policy: TruncationPolicy,
    ) -> Result<Vec<ZeroPersistenceWalRecord>, RecoveryWalError> {
        use std::io::Read;
        let mut file = std::fs::File::open(&self.path).map_err(|_| RecoveryWalError::Io)?;

        // Validate magic header.
        let mut magic = [0u8; 8];
        match file.read_exact(&mut magic) {
            Ok(()) if magic == RECOVERY_WAL_MAGIC => {}
            Ok(()) => return Err(RecoveryWalError::InvalidMagic),
            Err(_) => return Err(RecoveryWalError::InvalidMagic),
        }

        let mut out = Vec::new();
        loop {
            // Probe for the first byte to distinguish clean EOF from a truncated tail.
            // read_exact on an empty position returns UnexpectedEof for both cases, so
            // we use a single-byte read to tell them apart.
            let mut first = [0u8; 1];
            match file.read(&mut first) {
                Ok(0) => break, // clean EOF at a record boundary
                Ok(_) => {}
                Err(_) => return Err(RecoveryWalError::Io),
            }
            // We have at least one byte; read the rest of the record.
            let mut record = [0u8; RECOVERY_RECORD_BYTES];
            record[0] = first[0];
            match file.read_exact(&mut record[1..]) {
                Ok(()) => out.push(decode_record(&record)?),
                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    match policy {
                        TruncationPolicy::IgnoreTruncatedTail => break,
                        TruncationPolicy::RejectTruncatedTail => return Err(RecoveryWalError::InvalidLength),
                    }
                }
                Err(_) => return Err(RecoveryWalError::Io),
            }
        }
        Ok(out)
    }

    /// Replay all valid WAL records, ignoring a truncated tail.
    /// This is the standard recovery path; use `replay_with_policy` for stricter control.
    pub fn replay(&self) -> Result<Vec<ZeroPersistenceWalRecord>, RecoveryWalError> {
        self.replay_with_policy(TruncationPolicy::IgnoreTruncatedTail)
    }
}

/// Summary of public evidence state reconstructed from a WAL replay.
///
/// Contains only commitment roots — no raw transactions, peer identities,
/// graph edges, routes, or receipt body material.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecoveryEvidence {
    /// The highest epoch seen in any StateRoot WAL record.
    pub latest_epoch: u64,
    /// The state root from the highest-epoch StateRoot record, if any.
    pub latest_state_root: Option<[u8; 32]>,
    /// All effect commitment roots, in WAL order (no raw payloads).
    pub effect_roots: Vec<[u8; 32]>,
    /// All shred commitment event roots (key material is NOT stored here).
    pub shred_event_roots: Vec<[u8; 32]>,
}

impl RecoveryEvidence {
    fn ingest(&mut self, record: &ZeroPersistenceWalRecord) {
        match record {
            ZeroPersistenceWalRecord::StateRoot { epoch, state_root } => {
                if *epoch >= self.latest_epoch {
                    self.latest_epoch = *epoch;
                    self.latest_state_root = Some(*state_root);
                }
            }
            ZeroPersistenceWalRecord::EffectCommitment { effect_root, .. } => {
                self.effect_roots.push(*effect_root);
            }
            ZeroPersistenceWalRecord::ShredCommitment { event_root, .. } => {
                self.shred_event_roots.push(*event_root);
            }
            ZeroPersistenceWalRecord::BlindAudit { .. } => {
                // Blind audit roots are not included in public evidence state.
            }
        }
    }
}

/// Reconstruct public evidence state from a WAL without touching raw tx or peer data.
///
/// This is the zero-persistence recovery path: rebuild only roots and counters,
/// then re-sync the remainder from the network. Raw transactions, peer identities,
/// graph topology, and receipt body material are never stored in the WAL and
/// therefore never appear here.
#[cfg(feature = "std")]
pub fn replay_into_evidence(
    wal: &FileRecoveryWal,
    policy: TruncationPolicy,
) -> Result<RecoveryEvidence, RecoveryWalError> {
    let records = wal.replay_with_policy(policy)?;
    let mut evidence = RecoveryEvidence::default();
    for record in &records {
        evidence.ingest(record);
    }
    Ok(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_round_trips() {
        let record = ZeroPersistenceWalRecord::ShredCommitment { epoch: 3, key_id_commitment: [4u8; 32], event_root: [5u8; 32] };
        assert_eq!(decode_record(&encode_record(record)).unwrap(), record);
    }

    #[test]
    fn all_record_variants_round_trip() {
        let variants = [
            ZeroPersistenceWalRecord::EffectCommitment { epoch: 1, effect_root: [1u8; 32], receipt_root: [2u8; 32] },
            ZeroPersistenceWalRecord::StateRoot { epoch: 2, state_root: [3u8; 32] },
            ZeroPersistenceWalRecord::BlindAudit { epoch: 3, event_root: [4u8; 32] },
            ZeroPersistenceWalRecord::ShredCommitment { epoch: 4, key_id_commitment: [5u8; 32], event_root: [6u8; 32] },
        ];
        for v in &variants {
            assert_eq!(&decode_record(&encode_record(*v)).unwrap(), v);
        }
    }

    #[test]
    fn decode_invalid_length_returns_error() {
        assert_eq!(decode_record(&[0u8; 10]), Err(RecoveryWalError::InvalidLength));
        assert_eq!(decode_record(&[0u8; RECOVERY_RECORD_BYTES + 1]), Err(RecoveryWalError::InvalidLength));
    }

    #[test]
    fn decode_invalid_tag_returns_error() {
        let mut buf = [0u8; RECOVERY_RECORD_BYTES];
        buf[0] = 99;
        assert_eq!(decode_record(&buf), Err(RecoveryWalError::InvalidTag));
    }

    #[cfg(feature = "std")]
    #[test]
    fn file_wal_open_creates_magic_header() {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-wal-test-magic-{}.wal", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let _wal = FileRecoveryWal::open(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..8], &RECOVERY_WAL_MAGIC);

        let _ = std::fs::remove_file(&path);
    }

    #[cfg(feature = "std")]
    #[test]
    fn replay_returns_empty_on_fresh_wal() {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-wal-test-empty-{}.wal", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let wal = FileRecoveryWal::open(&path).unwrap();
        let records = wal.replay().unwrap();
        assert!(records.is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[cfg(feature = "std")]
    #[test]
    fn replay_round_trips_multiple_records() {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-wal-test-multi-{}.wal", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let wal = FileRecoveryWal::open(&path).unwrap();
        let r1 = ZeroPersistenceWalRecord::StateRoot { epoch: 10, state_root: [0xAA; 32] };
        let r2 = ZeroPersistenceWalRecord::EffectCommitment { epoch: 11, effect_root: [0xBB; 32], receipt_root: [0xCC; 32] };
        wal.append_synced(r1).unwrap();
        wal.append_synced(r2).unwrap();

        let reopened = FileRecoveryWal::open(&path).unwrap();
        let records = reopened.replay().unwrap();
        assert_eq!(records, vec![r1, r2]);

        let _ = std::fs::remove_file(&path);
    }

    #[cfg(feature = "std")]
    #[test]
    fn replay_ignore_truncated_tail_succeeds() {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-wal-test-trunc-ignore-{}.wal", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let wal = FileRecoveryWal::open(&path).unwrap();
        let r1 = ZeroPersistenceWalRecord::StateRoot { epoch: 5, state_root: [0x11; 32] };
        wal.append_synced(r1).unwrap();

        // Append a partial record (corruption: only half of a record)
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&[0u8; RECOVERY_RECORD_BYTES / 2]).unwrap();
        drop(file);

        let records = wal.replay_with_policy(TruncationPolicy::IgnoreTruncatedTail).unwrap();
        assert_eq!(records, vec![r1]);

        let _ = std::fs::remove_file(&path);
    }

    #[cfg(feature = "std")]
    #[test]
    fn replay_reject_truncated_tail_returns_error() {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-wal-test-trunc-reject-{}.wal", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let wal = FileRecoveryWal::open(&path).unwrap();
        let r1 = ZeroPersistenceWalRecord::StateRoot { epoch: 5, state_root: [0x22; 32] };
        wal.append_synced(r1).unwrap();

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&[0u8; RECOVERY_RECORD_BYTES / 2]).unwrap();
        drop(file);

        let result = wal.replay_with_policy(TruncationPolicy::RejectTruncatedTail);
        assert_eq!(result, Err(RecoveryWalError::InvalidLength));

        let _ = std::fs::remove_file(&path);
    }

    #[cfg(feature = "std")]
    #[test]
    fn replay_invalid_magic_returns_error() {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-wal-test-badmagic-{}.wal", std::process::id()));

        use std::io::Write;
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"BADMAGIC").unwrap();
        drop(file);

        let wal = FileRecoveryWal { path: path.clone() };
        assert_eq!(wal.replay(), Err(RecoveryWalError::InvalidMagic));

        let _ = std::fs::remove_file(&path);
    }

    #[cfg(feature = "std")]
    #[test]
    fn replay_into_evidence_builds_correct_state() {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-wal-test-evidence-{}.wal", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let wal = FileRecoveryWal::open(&path).unwrap();
        wal.append_synced(ZeroPersistenceWalRecord::StateRoot { epoch: 1, state_root: [0x01; 32] }).unwrap();
        wal.append_synced(ZeroPersistenceWalRecord::StateRoot { epoch: 5, state_root: [0x05; 32] }).unwrap();
        wal.append_synced(ZeroPersistenceWalRecord::StateRoot { epoch: 3, state_root: [0x03; 32] }).unwrap();
        wal.append_synced(ZeroPersistenceWalRecord::EffectCommitment { epoch: 2, effect_root: [0xEE; 32], receipt_root: [0xFF; 32] }).unwrap();
        wal.append_synced(ZeroPersistenceWalRecord::BlindAudit { epoch: 4, event_root: [0xAA; 32] }).unwrap();
        wal.append_synced(ZeroPersistenceWalRecord::ShredCommitment { epoch: 5, key_id_commitment: [0xBB; 32], event_root: [0xCC; 32] }).unwrap();

        let evidence = replay_into_evidence(&wal, TruncationPolicy::IgnoreTruncatedTail).unwrap();

        // latest_epoch is the highest StateRoot epoch seen (5), even though epoch=3 came after
        assert_eq!(evidence.latest_epoch, 5);
        assert_eq!(evidence.latest_state_root, Some([0x05; 32]));
        // effect_roots: only from EffectCommitment records
        assert_eq!(evidence.effect_roots, vec![[0xEE; 32]]);
        // shred_event_roots: only from ShredCommitment records (BlindAudit is excluded)
        assert_eq!(evidence.shred_event_roots, vec![[0xCC; 32]]);

        let _ = std::fs::remove_file(&path);
    }
}
