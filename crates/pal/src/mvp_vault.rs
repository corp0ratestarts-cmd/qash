//! Local Domain B vault and commitment WAL for the MVP demonstrator.
//!
//! Private receipt bodies stay in the local vault. Public exports contain only
//! commitment records derived from `TxMvpReceiptCommit`.

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
const DISCLOSURE_DIR: &str = "disclosures";
const COMMITMENT_WAL_FILE: &str = "commitments.wal";
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
        let existing: Vec<TxMvpReceiptCommit> = self
            .read_commitments()?
            .into_iter()
            .map(|record| record.tx)
            .collect();
        tx.validate_epoch_nonce_unused(existing.iter())?;

        let receipt_id = tx.tx_commitment()?;
        let receipt_path = self.receipt_path(receipt_id);
        let mut receipt_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&receipt_path)
            .map_err(|err| match err.kind() {
                ErrorKind::AlreadyExists => MvpVaultError::DuplicateReceipt,
                _ => MvpVaultError::Io(err),
            })?;
        receipt_file.write_all(body)?;
        receipt_file.sync_all()?;

        append_wal_record(
            &self.wal_path(),
            &CommitmentWalRecord {
                tx,
                public_export: tx.public_export()?,
            },
        )?;

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

        let disclosure_path = self.root.join(DISCLOSURE_DIR).join(hex32(receipt_id));
        let mut disclosure_file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(disclosure_path)?;
        disclosure_file.write_all(&out)?;
        disclosure_file.sync_all()?;
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
    ensure_wal_header(path)?;
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
        let expected = public_export.encode();
        if expected.as_slice() != &payload[TX_MVP_RECEIPT_COMMIT_BYTES..] {
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
    use std::path::PathBuf;

    #[derive(Clone, Copy)]
    enum FixtureKind {
        FirstNonce,
        SecondNonce,
        DisclosureCommitment,
    }

    fn fixture_bytes(kind: FixtureKind) -> [u8; 32] {
        core::array::from_fn(|idx| {
            let base = idx as u8;
            match kind {
                FixtureKind::FirstNonce => base.rotate_left(1),
                FixtureKind::SecondNonce => base.rotate_right(1),
                FixtureKind::DisclosureCommitment => !base,
            }
        })
    }

    fn temp_workspace(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-mvp-vault-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn issue_receipt_keeps_private_body_out_of_public_export() {
        let path = temp_workspace("public-export");
        let vault = MvpReceiptVault::init(&path).unwrap();
        let body = b"synthetic offline incident body";
        let receipt = vault
            .issue_receipt(
                10,
                fixture_bytes(FixtureKind::FirstNonce),
                body,
                fixture_bytes(FixtureKind::DisclosureCommitment),
            )
            .unwrap();
        let public = vault.export_public_commitments().unwrap();

        assert_eq!(vault.read_commitments().unwrap().len(), 1);
        assert!(!public.windows(body.len()).any(|window| window == body));
        assert!(!public.windows(32).any(|window| window == receipt.tx.nonce));
        assert!(public.windows(32).any(|window| window == receipt.tx.payload_commitment));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn duplicate_epoch_nonce_is_rejected() {
        let path = temp_workspace("duplicate");
        let vault = MvpReceiptVault::init(&path).unwrap();
        let nonce = fixture_bytes(FixtureKind::FirstNonce);
        let disclosure = fixture_bytes(FixtureKind::DisclosureCommitment);
        vault.issue_receipt(10, nonce, b"first synthetic body", disclosure).unwrap();
        let duplicate = vault.issue_receipt(10, nonce, b"second synthetic body", disclosure);
        assert!(matches!(duplicate, Err(MvpVaultError::Tx(_))));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn disclosure_exports_only_selected_receipt() {
        let path = temp_workspace("disclose");
        let vault = MvpReceiptVault::init(&path).unwrap();
        let first = vault
            .issue_receipt(
                10,
                fixture_bytes(FixtureKind::FirstNonce),
                b"first synthetic incident",
                fixture_bytes(FixtureKind::DisclosureCommitment),
            )
            .unwrap();
        vault
            .issue_receipt(
                10,
                fixture_bytes(FixtureKind::SecondNonce),
                b"second synthetic incident",
                fixture_bytes(FixtureKind::DisclosureCommitment),
            )
            .unwrap();

        let disclosure = vault.disclose_receipt(first.receipt_id).unwrap();
        assert!(disclosure.windows(b"first synthetic incident".len()).any(|w| w == b"first synthetic incident"));
        assert!(!disclosure.windows(b"second synthetic incident".len()).any(|w| w == b"second synthetic incident"));
        let _ = fs::remove_dir_all(&path);
    }
}
