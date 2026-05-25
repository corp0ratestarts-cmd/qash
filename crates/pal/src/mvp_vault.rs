//! Local Domain B vault and commitment WAL for the MVP demonstrator.
//!
//! This module persists private incident receipt bodies separately from the
//! commitment-only WAL. Export paths use public commitment records only.

#![cfg(feature = "std")]

use crate::mvp::{
    TxMvpReceiptCommit, TxMvpReceiptCommitError, TxMvpReceiptCommitPublicExport,
    TX_MVP_PUBLIC_EXPORT_BYTES, TX_MVP_RECEIPT_COMMIT_BYTES,
};
use sha3::{Digest, Sha3_256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

const MANIFEST_FILE: &str = "manifest.txt";
const VAULT_DIR: &str = "vault";
const COMMITMENT_WAL_FILE: &str = "commitments.wal";
const DISCLOSURE_DIR: &str = "disclosures";
const MANIFEST_MAGIC: &str = "QASH-MVP-INCIDENT-RECEIPT-DEMO\n";
const WAL_MAGIC: &[u8; 8] = b"QMVPWAL\0";
const WAL_RECORD_MAGIC: &[u8; 8] = b"QMVPREC\0";
const WAL_RECORD_BYTES: usize = TX_MVP_RECEIPT_COMMIT_BYTES + TX_MVP_PUBLIC_EXPORT_BYTES;

#[derive(Debug)]
pub enum MvpVaultError {
    Io(io::Error),
    Tx(TxMvpReceiptCommitError),
    InvalidWorkspace(&'static str),
    InvalidWal(&'static str),
    ReceiptNotFound,
    DuplicateReceipt,
}

impl From<io::Error> for MvpVaultError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<TxMvpReceiptCommitError> for MvpVaultError {
    fn from(err: TxMvpReceiptCommitError) -> Self {
        Self::Tx(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateIncidentReceipt {
    pub receipt_id: [u8; 32],
    pub body: Vec<u8>,
    pub tx: TxMvpReceiptCommit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitmentWalRecord {
    pub tx: TxMvpReceiptCommit,
    pub public_export: TxMvpReceiptCommitPublicExport,
}

#[derive(Debug, Clone)]
pub struct MvpReceiptVault {
    root: PathBuf,
}

impl MvpReceiptVault {
    pub fn init(root: impl Into<PathBuf>) -> Result<Self, MvpVaultError> {
        let root = root.into();
        fs::create_dir_all(root.join(VAULT_DIR))?;
        fs::create_dir_all(root.join(DISCLOSURE_DIR))?;
        fs::write(root.join(MANIFEST_FILE), MANIFEST_MAGIC.as_bytes())?;
        ensure_wal_header(&root.join(COMMITMENT_WAL_FILE))?;
        Ok(Self { root })
    }

    pub fn open(root: impl Into<PathBuf>) -> Result<Self, MvpVaultError> {
        let root = root.into();
        let manifest = fs::read_to_string(root.join(MANIFEST_FILE))?;
        if manifest != MANIFEST_MAGIC {
            return Err(MvpVaultError::InvalidWorkspace("invalid MVP manifest"));
        }
        ensure_wal_header(&root.join(COMMITMENT_WAL_FILE))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn issue_receipt(
        &self,
        epoch: u64,
        nonce: [u8; 32],
        body: &[u8],
        disclosure_key_commitment: [u8; 32],
    ) -> Result<PrivateIncidentReceipt, MvpVaultError> {
        let payload_commitment = payload_commitment(body);
        let tx = TxMvpReceiptCommit::new(epoch, nonce, payload_commitment, disclosure_key_commitment);
        let existing: Vec<TxMvpReceiptCommit> = self.read_commitments()?.into_iter().map(|r| r.tx).collect();
        tx.validate_epoch_nonce_unused(existing.iter())?;
        let receipt_id = tx.tx_commitment()?;
        let receipt_path = self.receipt_path(receipt_id);
        if receipt_path.exists() {
            return Err(MvpVaultError::DuplicateReceipt);
        }
        fs::write(&receipt_path, body)?;
        append_wal_record(&self.wal_path(), &CommitmentWalRecord {
            tx,
            public_export: tx.public_export()?,
        })?;
        Ok(PrivateIncidentReceipt {
            receipt_id,
            body: body.to_vec(),
            tx,
        })
    }

    pub fn read_commitments(&self) -> Result<Vec<CommitmentWalRecord>, MvpVaultError> {
        read_wal_records(&self.wal_path())
    }

    pub fn export_public_commitments(&self) -> Result<Vec<u8>, MvpVaultError> {
        let mut out = Vec::new();
        out.extend_from_slice(b"QASH-MVP-PUBLIC-COMMITMENTS\0");
        for record in self.read_commitments()? {
            out.extend_from_slice(&record.public_export.encode());
        }
        Ok(out)
    }

    pub fn disclose_receipt(&self, receipt_id: [u8; 32]) -> Result<Vec<u8>, MvpVaultError> {
        let body = match fs::read(self.receipt_path(receipt_id)) {
            Ok(body) => body,
            Err(err) if err.kind() == ErrorKind::NotFound => return Err(MvpVaultError::ReceiptNotFound),
            Err(err) => return Err(MvpVaultError::Io(err)),
        };
        let mut out = Vec::new();
        out.extend_from_slice(b"QASH-MVP-DISCLOSURE\0");
        out.extend_from_slice(&receipt_id);
        out.extend_from_slice(&(body.len() as u64).to_le_bytes());
        out.extend_from_slice(&body);
        fs::write(self.root.join(DISCLOSURE_DIR).join(hex32(receipt_id)), &out)?;
        Ok(out)
    }

    fn wal_path(&self) -> PathBuf {
        self.root.join(COMMITMENT_WAL_FILE)
    }

    fn receipt_path(&self, receipt_id: [u8; 32]) -> PathBuf {
        self.root.join(VAULT_DIR).join(hex32(receipt_id))
    }
}

pub fn payload_commitment(body: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(b"QASH-MVP-PAYLOAD-COMMITMENT\0");
    hasher.update((body.len() as u64).to_le_bytes());
    hasher.update(body);
    hasher.finalize().into()
}

fn ensure_wal_header(path: &Path) -> Result<(), MvpVaultError> {
    match File::open(path) {
        Ok(mut file) => {
            let mut magic = [0u8; 8];
            file.read_exact(&mut magic)?;
            if &magic != WAL_MAGIC {
                return Err(MvpVaultError::InvalidWal("invalid MVP WAL magic"));
            }
            Ok(())
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
            file.write_all(WAL_MAGIC)?;
            file.sync_all()?;
            Ok(())
        }
        Err(err) => Err(MvpVaultError::Io(err)),
    }
}

fn append_wal_record(path: &Path, record: &CommitmentWalRecord) -> Result<(), MvpVaultError> {
    let mut file = OpenOptions::new().append(true).open(path)?;
    file.write_all(WAL_RECORD_MAGIC)?;
    file.write_all(&record.tx.encode()?)?;
    file.write_all(&record.public_export.encode())?;
    file.sync_all()?;
    Ok(())
}

fn read_wal_records(path: &Path) -> Result<Vec<CommitmentWalRecord>, MvpVaultError> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != WAL_MAGIC {
        return Err(MvpVaultError::InvalidWal("invalid MVP WAL magic"));
    }
    let mut records = Vec::new();
    loop {
        let mut record_magic = [0u8; 8];
        match file.read_exact(&mut record_magic) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => break,
            Err(err) => return Err(MvpVaultError::Io(err)),
        }
        if &record_magic != WAL_RECORD_MAGIC {
            return Err(MvpVaultError::InvalidWal("invalid MVP WAL record magic"));
        }
        let mut payload = [0u8; WAL_RECORD_BYTES];
        file.read_exact(&mut payload)?;
        let tx = TxMvpReceiptCommit::decode(&payload[..TX_MVP_RECEIPT_COMMIT_BYTES])?;
        let public_export = tx.public_export()?;
        if public_export.encode() != payload[TX_MVP_RECEIPT_COMMIT_BYTES..] {
            return Err(MvpVaultError::InvalidWal("MVP WAL public export mismatch"));
        }
        records.push(CommitmentWalRecord { tx, public_export });
    }
    Ok(records)
}

fn hex32(bytes: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{MvpReceiptVault, MvpVaultError};
    use std::fs;

    fn temp_workspace(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn issue_receipt_keeps_private_body_out_of_public_export() {
        let path = temp_workspace("mvp-vault-public-export");
        let vault = MvpReceiptVault::init(&path).unwrap();
        let body = b"pump-station incident: offline door alarm";
        let receipt = vault.issue_receipt(10, [4u8; 32], body, [5u8; 32]).unwrap();
        let public = vault.export_public_commitments().unwrap();

        assert_eq!(vault.read_commitments().unwrap().len(), 1);
        assert!(!public.windows(body.len()).any(|window| window == body));
        assert!(!public.windows(32).any(|window| window == receipt.tx.nonce));
        assert!(public.windows(32).any(|window| window == receipt.tx.payload_commitment));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn duplicate_epoch_nonce_is_rejected() {
        let path = temp_workspace("mvp-vault-duplicate");
        let vault = MvpReceiptVault::init(&path).unwrap();
        vault.issue_receipt(10, [4u8; 32], b"first", [5u8; 32]).unwrap();
        let duplicate = vault.issue_receipt(10, [4u8; 32], b"second", [5u8; 32]);
        assert!(matches!(duplicate, Err(MvpVaultError::Tx(_))));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn disclosure_exports_only_selected_receipt() {
        let path = temp_workspace("mvp-vault-disclose");
        let vault = MvpReceiptVault::init(&path).unwrap();
        let first = vault.issue_receipt(10, [1u8; 32], b"first incident", [5u8; 32]).unwrap();
        vault.issue_receipt(10, [2u8; 32], b"second incident", [5u8; 32]).unwrap();

        let disclosure = vault.disclose_receipt(first.receipt_id).unwrap();
        assert!(disclosure.windows(b"first incident".len()).any(|w| w == b"first incident"));
        assert!(!disclosure.windows(b"second incident".len()).any(|w| w == b"second incident"));
        let _ = fs::remove_dir_all(&path);
    }
}
