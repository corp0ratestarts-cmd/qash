//! Corrupt-log robustness tests for the recovery WAL (PR #233 / Wave 4).
//!
//! Verifies that every malformed input to the recovery WAL decode and replay
//! paths fails closed — returning an error or a graceful empty result —
//! rather than panicking or silently producing corrupt state.
//!
//! These tests complement the honggfuzz target in fuzz/fuzz_targets/wal_decode.rs
//! and the existing crash_recovery_parity harness.

#![cfg(feature = "std")]

use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use qash_pal::recovery_wal::{
    decode_record, encode_record, FileRecoveryWal, RecoveryWalError, RECOVERY_RECORD_BYTES,
    RECOVERY_WAL_MAGIC,
};
use qash_pal::zero_wal::ZeroPersistenceWalRecord;

fn unique_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qash-robustness-{label}-{}-{nanos}.wal",
        std::process::id()
    ))
}

fn valid_record_bytes() -> [u8; RECOVERY_RECORD_BYTES] {
    encode_record(ZeroPersistenceWalRecord::StateRoot {
        epoch: 1,
        state_root: [0xab; 32],
    })
}

fn write_magic_file(path: &PathBuf, magic: &[u8]) {
    let mut f = std::fs::File::create(path).expect("create file");
    f.write_all(magic).expect("write magic");
}

/// A WAL file with a corrupt magic header must cause replay() to return
/// Err(InvalidMagic) — not panic, not silently succeed.
#[test]
fn corrupt_log_bad_magic_fails_closed() {
    let path = unique_path("bad-magic");
    write_magic_file(&path, b"BADMAGIC");
    let wal = FileRecoveryWal::open(&path).expect("open does not validate magic");
    let result = wal.replay();
    assert_eq!(
        result,
        Err(RecoveryWalError::InvalidMagic),
        "bad magic must return Err(InvalidMagic)"
    );
    let _ = std::fs::remove_file(&path);
}

/// A WAL file truncated in the middle of a record must replay gracefully:
/// records written before the truncation point are returned, and the partial
/// trailing record is treated as an expected EOF (not an error).
#[test]
fn corrupt_log_truncated_fails_closed() {
    let path = unique_path("truncated");
    {
        let mut f = std::fs::File::create(&path).expect("create file");
        f.write_all(&RECOVERY_WAL_MAGIC).expect("write magic");
        f.write_all(&valid_record_bytes()).expect("write record 1");
        // Write only a partial second record (half of RECOVERY_RECORD_BYTES).
        let partial = &valid_record_bytes()[..RECOVERY_RECORD_BYTES / 2];
        f.write_all(partial).expect("write partial record");
    }
    let wal = FileRecoveryWal::open(&path).expect("open");
    let result = wal.replay().expect("truncated-tail WAL must not return Err");
    assert_eq!(
        result.len(),
        1,
        "only the one complete record before truncation must be returned"
    );
    let _ = std::fs::remove_file(&path);
}

/// A WAL file whose last entry is a partial (fewer-than-record-size) trailing
/// byte sequence — but no complete records at all after the magic — must
/// replay as an empty record list (graceful, not an error).
#[test]
fn corrupt_log_trailing_bytes_fails_closed() {
    let path = unique_path("trailing-bytes");
    {
        let mut f = std::fs::File::create(&path).expect("create file");
        f.write_all(&RECOVERY_WAL_MAGIC).expect("write magic");
        // Write a few stray bytes that do not form a complete record.
        f.write_all(&[0xde, 0xad, 0xbe, 0xef]).expect("write junk");
    }
    let wal = FileRecoveryWal::open(&path).expect("open");
    let result = wal.replay().expect("trailing-bytes WAL must not return Err");
    assert!(
        result.is_empty(),
        "no complete records → replay must return empty vec"
    );
    let _ = std::fs::remove_file(&path);
}

/// A WAL file containing a complete-length record with an invalid tag byte
/// must cause replay() to return Err(InvalidTag) — not silently skip the
/// corrupt record.
#[test]
fn corrupt_log_invalid_tag_fails_closed() {
    let path = unique_path("invalid-tag");
    {
        let mut f = std::fs::File::create(&path).expect("create file");
        f.write_all(&RECOVERY_WAL_MAGIC).expect("write magic");
        // Construct a full-length record but with tag = 0xFF (undefined).
        let mut bad_record = [0u8; RECOVERY_RECORD_BYTES];
        bad_record[0] = 0xFF;
        f.write_all(&bad_record).expect("write invalid-tag record");
    }
    let wal = FileRecoveryWal::open(&path).expect("open");
    let result = wal.replay();
    assert_eq!(
        result,
        Err(RecoveryWalError::InvalidTag),
        "invalid tag byte in a complete record must return Err(InvalidTag)"
    );
    let _ = std::fs::remove_file(&path);
}

/// decode_record must return Err(InvalidLength) for any input that is not
/// exactly RECOVERY_RECORD_BYTES bytes long — including zero-length, short,
/// and over-length inputs.
///
/// This ensures that no caller can silently pass a wrong-size buffer and
/// receive a partially-decoded record.
#[test]
fn corrupt_log_wrong_length_record_fails_closed() {
    // Zero-length input.
    assert_eq!(
        decode_record(&[]),
        Err(RecoveryWalError::InvalidLength),
        "zero-length input must return Err(InvalidLength)"
    );

    // One byte short of a valid record.
    let short = vec![0u8; RECOVERY_RECORD_BYTES - 1];
    assert_eq!(
        decode_record(&short),
        Err(RecoveryWalError::InvalidLength),
        "short input must return Err(InvalidLength)"
    );

    // One byte over a valid record (over-limit).
    let long = vec![0u8; RECOVERY_RECORD_BYTES + 1];
    assert_eq!(
        decode_record(&long),
        Err(RecoveryWalError::InvalidLength),
        "over-limit input must return Err(InvalidLength)"
    );

    // Significantly over-limit (simulates a caller passing an entire WAL
    // file buffer instead of a single record slice).
    let very_long = vec![0u8; RECOVERY_RECORD_BYTES * 64];
    assert_eq!(
        decode_record(&very_long),
        Err(RecoveryWalError::InvalidLength),
        "very large input must return Err(InvalidLength)"
    );
}
