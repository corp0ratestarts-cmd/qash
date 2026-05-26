//! Unlocked Domain A demo profile adapter for MVP receipt commitments.
//!
//! This module is a non-genesis demonstrator adapter. It accepts public
//! `TxMvpReceiptCommitPublicExport` records only and folds them into a
//! deterministic demo root. It does not admit `TX-MVP-ReceiptCommit` into locked
//! Domain A consensus.

use crate::mvp::{
    TxMvpReceiptCommitError, TxMvpReceiptCommitPublicExport, TX_MVP_PUBLIC_EXPORT_BYTES,
};
use sha3::{Digest, Sha3_256};

pub const MVP_DEMO_PROFILE_NAME: &str = "qash-mvp-unlocked-domain-a-demo-profile";
pub const MVP_DEMO_PROFILE_VERSION: u32 = 1;
pub const MVP_DEMO_PROFILE_ROOT_DOMAIN: &[u8] = b"QASH-MVP-DEMO-PROFILE-ROOT\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MvpDemoProfileError {
    InvalidPublicExport(TxMvpReceiptCommitError),
    EmptyInput,
}

impl From<TxMvpReceiptCommitError> for MvpDemoProfileError {
    fn from(err: TxMvpReceiptCommitError) -> Self {
        Self::InvalidPublicExport(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvpDemoProfileReplayReport {
    pub profile_name: &'static str,
    pub profile_version: u32,
    pub records: usize,
    pub commitment_root: [u8; 32],
    pub public_transcript_only: bool,
    pub private_payloads_seen: bool,
}

pub fn decode_public_exports(
    bytes: &[u8],
) -> Result<Vec<TxMvpReceiptCommitPublicExport>, MvpDemoProfileError> {
    if bytes.is_empty() {
        return Err(MvpDemoProfileError::EmptyInput);
    }
    if !bytes.len().is_multiple_of(TX_MVP_PUBLIC_EXPORT_BYTES) {
        return Err(TxMvpReceiptCommitError::InvalidLength.into());
    }

    let mut out = Vec::with_capacity(bytes.len() / TX_MVP_PUBLIC_EXPORT_BYTES);
    for chunk in bytes.chunks_exact(TX_MVP_PUBLIC_EXPORT_BYTES) {
        out.push(TxMvpReceiptCommitPublicExport::decode(chunk)?);
    }
    Ok(out)
}

pub fn replay_public_exports(records: &[TxMvpReceiptCommitPublicExport]) -> [u8; 32] {
    let mut root = [0u8; 32];
    for record in records {
        root = replay_step(root, &record.encode());
    }
    root
}

pub fn replay_public_export_bytes(bytes: &[u8]) -> Result<MvpDemoProfileReplayReport, MvpDemoProfileError> {
    let records = decode_public_exports(bytes)?;
    let commitment_root = replay_public_exports(&records);
    Ok(MvpDemoProfileReplayReport {
        profile_name: MVP_DEMO_PROFILE_NAME,
        profile_version: MVP_DEMO_PROFILE_VERSION,
        records: records.len(),
        commitment_root,
        public_transcript_only: true,
        private_payloads_seen: false,
    })
}

fn replay_step(previous: [u8; 32], public_record: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(MVP_DEMO_PROFILE_ROOT_DOMAIN);
    hasher.update(previous);
    hasher.update((public_record.len() as u64).to_le_bytes());
    hasher.update(public_record);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{decode_public_exports, replay_public_exports, replay_public_export_bytes, MvpDemoProfileError};
    use crate::mvp::{
        TxMvpReceiptCommit, TxMvpReceiptCommitError, TxMvpReceiptCommitPublicExport,
        TX_MVP_PUBLIC_EXPORT_BYTES, TX_MVP_RECEIPT_COMMIT_VERSION,
    };

    #[derive(Clone, Copy)]
    enum FixtureKind {
        FirstNonce,
        SecondNonce,
        FirstPayloadCommitment,
        SecondPayloadCommitment,
        DisclosureCommitment,
    }

    fn fixture_bytes(kind: FixtureKind) -> [u8; 32] {
        core::array::from_fn(|idx| {
            let base = idx as u8;
            match kind {
                FixtureKind::FirstNonce => base.rotate_left(1),
                FixtureKind::SecondNonce => base.rotate_right(1),
                FixtureKind::FirstPayloadCommitment => !base,
                FixtureKind::SecondPayloadCommitment => base.reverse_bits(),
                FixtureKind::DisclosureCommitment => (!base).rotate_left(1),
            }
        })
    }

    fn public_export_sequence() -> [TxMvpReceiptCommitPublicExport; 2] {
        let first = TxMvpReceiptCommit::new(
            10,
            fixture_bytes(FixtureKind::FirstNonce),
            fixture_bytes(FixtureKind::FirstPayloadCommitment),
            fixture_bytes(FixtureKind::DisclosureCommitment),
        )
        .public_export()
        .expect("valid public export fixture");

        let second = TxMvpReceiptCommit::new(
            11,
            fixture_bytes(FixtureKind::SecondNonce),
            fixture_bytes(FixtureKind::SecondPayloadCommitment),
            fixture_bytes(FixtureKind::DisclosureCommitment),
        )
        .public_export()
        .expect("valid public export fixture");

        [first, second]
    }

    #[test]
    fn public_export_bytes_decode_and_replay() {
        let records = public_export_sequence();
        let mut bytes = Vec::new();
        for record in records {
            bytes.extend_from_slice(&record.encode());
        }

        let decoded = decode_public_exports(&bytes).unwrap();
        assert_eq!(decoded, records);
        let report = replay_public_export_bytes(&bytes).unwrap();
        assert_eq!(report.records, 2);
        assert!(report.public_transcript_only);
        assert!(!report.private_payloads_seen);
        assert_eq!(report.commitment_root, replay_public_exports(&records));
    }

    #[test]
    fn malformed_public_export_bytes_fail_closed() {
        assert_eq!(replay_public_export_bytes(&[]), Err(MvpDemoProfileError::EmptyInput));

        let records = public_export_sequence();
        let valid = records[0].encode();
        assert_eq!(
            decode_public_exports(&valid[..TX_MVP_PUBLIC_EXPORT_BYTES - 1]),
            Err(MvpDemoProfileError::InvalidPublicExport(TxMvpReceiptCommitError::InvalidLength))
        );

        let mut invalid_version = valid;
        invalid_version[0..4].copy_from_slice(&(TX_MVP_RECEIPT_COMMIT_VERSION + 1).to_le_bytes());
        assert_eq!(
            decode_public_exports(&invalid_version),
            Err(MvpDemoProfileError::InvalidPublicExport(TxMvpReceiptCommitError::InvalidVersion))
        );
    }

    #[test]
    fn replay_root_is_order_sensitive_transcript_order_is_the_rule() {
        let records = public_export_sequence();
        let forward = replay_public_exports(&records);
        let reversed = replay_public_exports(&[records[1], records[0]]);
        assert_ne!(forward, reversed);
    }
}
