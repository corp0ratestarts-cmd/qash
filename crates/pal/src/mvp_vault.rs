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
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

const MANIFEST_FILE: &str = "manifest.txt";
const VAULT_DIR: &str = "vault";
const DISCLOSURE_DIR: &str = "disclosures";
const COMMITMENT_WAL_FILE: &str = "commitments.wal";
const IMPORTED_COMMITMENTS_FILE: &str = "imported_commitments.bin";
const PUBLIC_COMMITMENTS_HEADER: &[u8] = b"QASH-MVP-PUBLIC-COMMITMENTS\0";
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EpochNonceKey {
    epoch: u64,
    nonce: [u8; 32],
}

impl EpochNonceKey {
    fn from_tx(tx: &TxMvpReceiptCommit) -> Self {
        Self {
            epoch: tx.epoch,
            nonce: tx.nonce,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MvpReceiptVault {
    root: PathBuf,
    epoch_nonce_index: RefCell<BTreeSet<EpochNonceKey>>,
}

impl MvpReceiptVault {
    pub fn init(root: impl Into<PathBuf>) -> Result<Self, MvpVaultError> {
        let root = root.into();
        fs::create_dir_all(root.join(VAULT_DIR))?;
        fs::create_dir_all(root.join(DISCLOSURE_DIR))?;
        fs::write(root.join(MANIFEST_FILE), MANIFEST_MAGIC.as_bytes())?;
        ensure_wal_header(&root.join(COMMITMENT_WAL_FILE))?;
        Ok(Self {
            root,
            epoch_nonce_index: RefCell::new(BTreeSet::new()),
        })
    }

    pub fn open(root: impl Into<PathBuf>) -> Result<Self, MvpVaultError> {
        let root = root.into();
        let manifest = fs::read_to_string(root.join(MANIFEST_FILE))?;
        if manifest != MANIFEST_MAGIC {
            return Err(MvpVaultError::InvalidWorkspace("invalid MVP manifest"));
        }
        let wal_path = root.join(COMMITMENT_WAL_FILE);
        ensure_wal_header(&wal_path)?;
        let records = read_wal_records(&wal_path)?;
        let epoch_nonce_index = records
            .iter()
            .map(|record| EpochNonceKey::from_tx(&record.tx))
            .collect();
        Ok(Self {
            root,
            epoch_nonce_index: RefCell::new(epoch_nonce_index),
        })
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
        tx.validate()?;
        let epoch_nonce_key = EpochNonceKey::from_tx(&tx);
        if self.epoch_nonce_index.borrow().contains(&epoch_nonce_key) {
            return Err(MvpVaultError::Tx(TxMvpReceiptCommitError::DuplicateEpochNonce));
        }

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

        let append_result = append_wal_record(
            &self.wal_path(),
            &CommitmentWalRecord {
                tx,
                public_export: tx.public_export()?,
            },
        );

        if let Err(err) = append_result {
            let _ = fs::remove_file(&receipt_path);
            return Err(err);
        }

        self.epoch_nonce_index.borrow_mut().insert(epoch_nonce_key);
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
        out.extend_from_slice(PUBLIC_COMMITMENTS_HEADER);
        for record in self.read_commitments()? {
            out.extend_from_slice(&record.public_export.encode());
        }
        Ok(out)
    }

    pub fn import_public_commitments(&self, data: &[u8]) -> Result<usize, MvpVaultError> {
        if data.len() < PUBLIC_COMMITMENTS_HEADER.len()
            || &data[..PUBLIC_COMMITMENTS_HEADER.len()] != PUBLIC_COMMITMENTS_HEADER
        {
            return Err(MvpVaultError::InvalidWal("invalid public commitments header"));
        }
        let records_data = &data[PUBLIC_COMMITMENTS_HEADER.len()..];
        if !records_data.len().is_multiple_of(TX_MVP_PUBLIC_EXPORT_BYTES) {
            return Err(MvpVaultError::InvalidWal("truncated public commitments record"));
        }
        let count = records_data.len() / TX_MVP_PUBLIC_EXPORT_BYTES;
        for i in 0..count {
            let start = i * TX_MVP_PUBLIC_EXPORT_BYTES;
            TxMvpReceiptCommitPublicExport::decode(&records_data[start..start + TX_MVP_PUBLIC_EXPORT_BYTES])?;
        }
        fs::write(self.root.join(IMPORTED_COMMITMENTS_FILE), data)?;
        Ok(count)
    }

    pub fn read_all_public_exports(&self) -> Result<Vec<TxMvpReceiptCommitPublicExport>, MvpVaultError> {
        let mut exports: Vec<TxMvpReceiptCommitPublicExport> = self
            .read_commitments()?
            .into_iter()
            .map(|r| r.public_export)
            .collect();
        let import_path = self.root.join(IMPORTED_COMMITMENTS_FILE);
        if import_path.exists() {
            let data = fs::read(&import_path)?;
            if data.len() >= PUBLIC_COMMITMENTS_HEADER.len()
                && &data[..PUBLIC_COMMITMENTS_HEADER.len()] == PUBLIC_COMMITMENTS_HEADER
            {
                let records_data = &data[PUBLIC_COMMITMENTS_HEADER.len()..];
                let n = records_data.len() / TX_MVP_PUBLIC_EXPORT_BYTES;
                for i in 0..n {
                    let start = i * TX_MVP_PUBLIC_EXPORT_BYTES;
                    let export = TxMvpReceiptCommitPublicExport::decode(
                        &records_data[start..start + TX_MVP_PUBLIC_EXPORT_BYTES],
                    )?;
                    exports.push(export);
                }
            }
        }
        Ok(exports)
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
        match file.read_exact(&mut payload) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                return Err(MvpVaultError::InvalidWal("truncated WAL record"));
            }
            Err(err) => return Err(MvpVaultError::Io(err)),
        }
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
    fn reopened_vault_rebuilds_epoch_nonce_index() {
        let path = temp_workspace("reopen-index");
        let nonce = fixture_bytes(FixtureKind::FirstNonce);
        let disclosure = fixture_bytes(FixtureKind::DisclosureCommitment);
        let vault = MvpReceiptVault::init(&path).unwrap();
        vault.issue_receipt(10, nonce, b"first synthetic body", disclosure).unwrap();
        drop(vault);

        let reopened = MvpReceiptVault::open(&path).unwrap();
        let duplicate = reopened.issue_receipt(10, nonce, b"second synthetic body", disclosure);
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

    #[test]
    fn disclosure_of_unknown_receipt_is_rejected() {
        let path = temp_workspace("unknown-receipt");
        let vault = MvpReceiptVault::init(&path).unwrap();
        let unknown_id = fixture_bytes(FixtureKind::FirstNonce);
        assert!(matches!(vault.disclose_receipt(unknown_id), Err(MvpVaultError::ReceiptNotFound)));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn truncated_wal_record_is_rejected() {
        let path = temp_workspace("truncated-wal");
        let vault = MvpReceiptVault::init(&path).unwrap();
        vault
            .issue_receipt(
                10,
                fixture_bytes(FixtureKind::FirstNonce),
                b"synthetic body",
                fixture_bytes(FixtureKind::DisclosureCommitment),
            )
            .unwrap();
        let wal_path = path.join(super::COMMITMENT_WAL_FILE);
        let mut data = fs::read(&wal_path).unwrap();
        data.truncate(data.len() - 10);
        fs::write(&wal_path, &data).unwrap();
        assert!(matches!(MvpReceiptVault::open(&path), Err(MvpVaultError::InvalidWal(_))));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn wrong_wal_header_magic_is_rejected() {
        let path = temp_workspace("wrong-magic");
        let vault = MvpReceiptVault::init(&path).unwrap();
        drop(vault);
        let wal_path = path.join(super::COMMITMENT_WAL_FILE);
        let mut data = fs::read(&wal_path).unwrap();
        data[0] ^= 0xff;
        fs::write(&wal_path, &data).unwrap();
        assert!(matches!(MvpReceiptVault::open(&path), Err(MvpVaultError::InvalidWal(_))));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn wrong_wal_record_magic_is_rejected() {
        let path = temp_workspace("wrong-record-magic");
        let vault = MvpReceiptVault::init(&path).unwrap();
        vault
            .issue_receipt(
                10,
                fixture_bytes(FixtureKind::FirstNonce),
                b"synthetic body",
                fixture_bytes(FixtureKind::DisclosureCommitment),
            )
            .unwrap();
        let wal_path = path.join(super::COMMITMENT_WAL_FILE);
        let mut data = fs::read(&wal_path).unwrap();
        let header_len = super::WAL_MAGIC.len();
        data[header_len] ^= 0xff;
        fs::write(&wal_path, &data).unwrap();
        assert!(matches!(
            MvpReceiptVault::open(&path),
            Err(MvpVaultError::InvalidWal(_))
        ));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn import_allows_replay_but_not_disclosure() {
        let node_a = temp_workspace("import-node-a");
        let node_b = temp_workspace("import-node-b");

        let vault_a = MvpReceiptVault::init(&node_a).unwrap();
        let receipt = vault_a
            .issue_receipt(
                10,
                fixture_bytes(FixtureKind::FirstNonce),
                b"synthetic import body",
                fixture_bytes(FixtureKind::DisclosureCommitment),
            )
            .unwrap();
        let public = vault_a.export_public_commitments().unwrap();

        let vault_b = MvpReceiptVault::init(&node_b).unwrap();
        let count = vault_b.import_public_commitments(&public).unwrap();
        assert_eq!(count, 1);

        let exports_a = vault_a.read_all_public_exports().unwrap();
        let exports_b = vault_b.read_all_public_exports().unwrap();
        assert_eq!(exports_a.len(), exports_b.len());
        assert_eq!(exports_a[0].encode(), exports_b[0].encode());

        assert!(matches!(
            vault_b.disclose_receipt(receipt.receipt_id),
            Err(MvpVaultError::ReceiptNotFound)
        ));

        let _ = fs::remove_dir_all(&node_a);
        let _ = fs::remove_dir_all(&node_b);
    }
}
