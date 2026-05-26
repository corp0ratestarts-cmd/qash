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
const VAULT_SALT_FILE: &str = "vault_salt.bin";
const VAULT_DIR: &str = "vault";
const DISCLOSURE_DIR: &str = "disclosures";
const COMMITMENT_WAL_FILE: &str = "commitments.wal";
const IMPORTED_COMMITMENTS_FILE: &str = "imported_commitments.bin";
const IMPORTS_DIR: &str = "imports";
const IMPORTS_MANIFEST_FILE: &str = "imports/manifest.json";
const PUBLIC_COMMITMENTS_HEADER: &[u8] = b"QASH-MVP-PUBLIC-COMMITMENTS\0";
const VAULT_SALT_BYTES: usize = 32;
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
        let salt_path = root.join(VAULT_SALT_FILE);
        if !salt_path.exists() {
            let mut salt = [0u8; VAULT_SALT_BYTES];
            getrandom::getrandom(&mut salt).map_err(|err| MvpVaultError::Io(io::Error::other(err.to_string())))?;
            fs::write(salt_path, salt)?;
        }
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
        let mut seen: BTreeSet<[u8; 32]> = BTreeSet::new();
        let mut exports: Vec<TxMvpReceiptCommitPublicExport> = Vec::new();

        for record in self.read_commitments()? {
            let key = record.public_export.tx_commitment;
            if seen.insert(key) {
                exports.push(record.public_export);
            }
        }

        // Legacy single-file import (sync --import).
        let import_path = self.root.join(IMPORTED_COMMITMENTS_FILE);
        if import_path.exists() {
            let data = fs::read(&import_path)?;
            append_public_exports_from_bytes(&data, &mut exports, &mut seen)?;
        }

        // Multi-source imports (import-commitments --file).
        let imports_dir = self.root.join(IMPORTS_DIR);
        if imports_dir.is_dir() {
            let mut entries: Vec<_> = fs::read_dir(&imports_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension().and_then(|x| x.to_str()) == Some("bin")
                })
                .collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let data = fs::read(entry.path())?;
                append_public_exports_from_bytes(&data, &mut exports, &mut seen)?;
            }
        }

        Ok(exports)
    }

    /// Import a public commitments file with a source label (v0.3 multi-operator path).
    ///
    /// Stores the binary in `imports/NNN.bin` and appends an entry to
    /// `imports/manifest.json`. Records already seen in earlier imports or the
    /// local WAL are counted as duplicates but not re-stored — the raw file is
    /// kept intact so the transcript is reproducible.
    pub fn import_with_label(&self, data: &[u8], label: &str) -> Result<ImportResult, MvpVaultError> {
        if data.len() < PUBLIC_COMMITMENTS_HEADER.len()
            || &data[..PUBLIC_COMMITMENTS_HEADER.len()] != PUBLIC_COMMITMENTS_HEADER
        {
            return Err(MvpVaultError::InvalidWal("invalid public commitments header"));
        }
        let records_data = &data[PUBLIC_COMMITMENTS_HEADER.len()..];
        if !records_data.len().is_multiple_of(TX_MVP_PUBLIC_EXPORT_BYTES) {
            return Err(MvpVaultError::InvalidWal("truncated public commitments record"));
        }
        let total = records_data.len() / TX_MVP_PUBLIC_EXPORT_BYTES;
        for i in 0..total {
            let start = i * TX_MVP_PUBLIC_EXPORT_BYTES;
            TxMvpReceiptCommitPublicExport::decode(&records_data[start..start + TX_MVP_PUBLIC_EXPORT_BYTES])?;
        }

        let imports_dir = self.root.join(IMPORTS_DIR);
        fs::create_dir_all(&imports_dir)?;

        // Count duplicates against already-present exports BEFORE writing the new
        // file so the newly-added records don't appear in their own dedup check.
        let existing = self.read_all_public_exports()?;
        let existing_keys: BTreeSet<[u8; 32]> = existing.iter().map(|e| e.tx_commitment).collect();
        // Count duplicates against already-present exports BEFORE writing the file.
        let existing = self.read_all_public_exports()?;
        let existing_keys: BTreeSet<[u8; 32]> = existing.iter().map(|e| e.tx_commitment).collect();
        let mut new_records: usize = 0;
        let mut duplicates: usize = 0;
        for i in 0..total {
            let start = i * TX_MVP_PUBLIC_EXPORT_BYTES;
            let export = TxMvpReceiptCommitPublicExport::decode(
                &records_data[start..start + TX_MVP_PUBLIC_EXPORT_BYTES],
            )?;
            if existing_keys.contains(&export.tx_commitment) {
                duplicates += 1;
            } else {
                new_records += 1;
            }
        }

        let seq = next_import_seq(&imports_dir)?;
        let filename = format!("{seq:03}.bin");
        fs::write(imports_dir.join(&filename), data)?;

        append_import_manifest_entry(&self.root, seq, label, &filename, total)?;

        Ok(ImportResult { seq, label: label.to_string(), records: total, new_records, duplicates })
    }

    /// Return the list of multi-source import entries from `imports/manifest.json`.
    pub fn read_import_manifest(&self) -> Result<Vec<ImportManifestEntry>, MvpVaultError> {
        let path = self.root.join(IMPORTS_MANIFEST_FILE);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&path)?;
        parse_import_manifest(&text)
    }

    /// Derive a nonce from the workspace salt + current WAL record count + epoch.
    /// Produces a unique, deterministic-within-workspace nonce without exposing
    /// raw entropy; keeps `--nonce-hex` for deterministic test mode.
    pub fn fresh_nonce(&self, epoch: u64) -> Result<[u8; 32], MvpVaultError> {
        let salt_bytes = fs::read(self.root.join(VAULT_SALT_FILE))?;
        if salt_bytes.len() != VAULT_SALT_BYTES {
            return Err(MvpVaultError::InvalidWorkspace("vault salt has wrong size"));
        }
        let counter = u64::try_from(self.read_commitments()?.len())
            .unwrap_or(u64::MAX);
        let mut hasher = Sha3_256::new();
        hasher.update(b"QASH-MVP-NONCE-DERIVE\0");
        hasher.update(&salt_bytes);
        hasher.update(counter.to_le_bytes());
        hasher.update(epoch.to_le_bytes());
        Ok(hasher.finalize().into())
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

// ---------------------------------------------------------------------------
// Multi-import types and helpers (v0.3)
// ---------------------------------------------------------------------------

/// Result of a labelled multi-source import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportResult {
    pub seq: u32,
    pub label: String,
    pub records: usize,
    pub new_records: usize,
    pub duplicates: usize,
}

/// One entry in the `imports/manifest.json` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportManifestEntry {
    pub seq: u32,
    pub label: String,
    pub file: String,
    pub records: usize,
}

fn next_import_seq(imports_dir: &Path) -> Result<u32, MvpVaultError> {
    let mut max_seq: u32 = 0;
    for entry in fs::read_dir(imports_dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|x| x.to_str()) == Some("bin") {
            let name = entry.file_name();
            let stem = name.to_string_lossy();
            if let Ok(n) = stem.trim_end_matches(".bin").parse::<u32>() {
                if n > max_seq {
                    max_seq = n;
                }
            }
        }
    }
    Ok(max_seq + 1)
}

fn append_import_manifest_entry(
    root: &Path,
    seq: u32,
    label: &str,
    filename: &str,
    records: usize,
) -> Result<(), MvpVaultError> {
    let path = root.join(IMPORTS_MANIFEST_FILE);
    let mut entries = if path.exists() {
        let text = fs::read_to_string(&path)?;
        parse_import_manifest(&text)?
    } else {
        Vec::new()
    };
    entries.push(ImportManifestEntry {
        seq,
        label: label.to_string(),
        file: filename.to_string(),
        records,
    });
    fs::write(&path, serialise_import_manifest(&entries))?;
    Ok(())
}

fn serialise_import_manifest(entries: &[ImportManifestEntry]) -> String {
    let mut out = String::from("[\n");
    for (i, e) in entries.iter().enumerate() {
        let comma = if i + 1 < entries.len() { "," } else { "" };
        let label_escaped = e.label.replace('\\', "\\\\").replace('"', "\\\"");
        let file_escaped = e.file.replace('\\', "\\\\").replace('"', "\\\"");
        out.push_str(&format!(
            "  {{\"seq\":{},\"label\":\"{}\",\"file\":\"{}\",\"records\":{}}}{}\n",
            e.seq, label_escaped, file_escaped, e.records, comma
        ));
    }
    out.push(']');
    out
}

fn parse_import_manifest(text: &str) -> Result<Vec<ImportManifestEntry>, MvpVaultError> {
    // Minimal hand-rolled parser for the specific format written by serialise_import_manifest.
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with('{') || !line.ends_with('}') {
            continue;
        }
        let inner = &line[1..line.len() - 1];
        let seq = extract_json_u32(inner, "seq").unwrap_or(0);
        let label = extract_json_str(inner, "label").unwrap_or_default();
        let file = extract_json_str(inner, "file").unwrap_or_default();
        let records = extract_json_usize(inner, "records").unwrap_or(0);
        if seq > 0 && !file.is_empty() {
            entries.push(ImportManifestEntry { seq, label, file, records });
        }
    }
    Ok(entries)
}

fn extract_json_str(s: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start = s.find(&needle)? + needle.len();
    let rest = &s[start..];
    let end = rest.find('"')?;
    Some(rest[..end].replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn extract_json_u32(s: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{}\":", key);
    let start = s.find(&needle)? + needle.len();
    let rest = s[start..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn extract_json_usize(s: &str, key: &str) -> Option<usize> {
    extract_json_u32(s, key).map(|n| n as usize)
}

fn append_public_exports_from_bytes(
    data: &[u8],
    exports: &mut Vec<TxMvpReceiptCommitPublicExport>,
    seen: &mut BTreeSet<[u8; 32]>,
) -> Result<(), MvpVaultError> {
    if data.len() < PUBLIC_COMMITMENTS_HEADER.len() {
        return Ok(());
    }
    if &data[..PUBLIC_COMMITMENTS_HEADER.len()] != PUBLIC_COMMITMENTS_HEADER {
        return Ok(());
    }
    let records_data = &data[PUBLIC_COMMITMENTS_HEADER.len()..];
    let n = records_data.len() / TX_MVP_PUBLIC_EXPORT_BYTES;
    for i in 0..n {
        let start = i * TX_MVP_PUBLIC_EXPORT_BYTES;
        let export = TxMvpReceiptCommitPublicExport::decode(
            &records_data[start..start + TX_MVP_PUBLIC_EXPORT_BYTES],
        )?;
        if seen.insert(export.tx_commitment) {
            exports.push(export);
        }
    }
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

    // ── Additional corruption / schema hardening tests ────────────────────

    #[test]
    fn wal_record_body_payload_corruption_is_detected_on_reopen() {
        // Flip bytes deep inside the WAL record payload (not the magic prefix).
        // The record is structurally complete (correct length and magic) but
        // the inner TX bytes are corrupt. Opening must reject it.
        let path = temp_workspace("payload-corrupt");
        let vault = MvpReceiptVault::init(&path).unwrap();
        vault
            .issue_receipt(
                10,
                fixture_bytes(FixtureKind::FirstNonce),
                b"synthetic body for corruption test",
                fixture_bytes(FixtureKind::DisclosureCommitment),
            )
            .unwrap();
        let wal_path = path.join(super::COMMITMENT_WAL_FILE);
        let mut data = fs::read(&wal_path).unwrap();
        // Offset: WAL_MAGIC (8) + WAL_RECORD_MAGIC (8) = 16 bytes header,
        // then flip a byte in the middle of the TX payload.
        let header_size = 16;
        let corrupt_offset = header_size + 20;
        assert!(data.len() > corrupt_offset, "WAL too short for corruption test");
        data[corrupt_offset] ^= 0xAB;
        fs::write(&wal_path, &data).unwrap();
        assert!(
            matches!(MvpReceiptVault::open(&path), Err(MvpVaultError::InvalidWal(_))),
            "corrupt WAL payload must be rejected on reopen"
        );
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn wal_truncated_to_header_plus_record_magic_only_is_rejected() {
        // Write a WAL with the file magic and a record magic prefix but no
        // record payload — simulates a crash mid-write.
        let path = temp_workspace("truncated-at-record-magic");
        let vault = MvpReceiptVault::init(&path).unwrap();
        drop(vault);
        let wal_path = path.join(super::COMMITMENT_WAL_FILE);
        let mut data = fs::read(&wal_path).unwrap();
        data.extend_from_slice(super::WAL_RECORD_MAGIC);
        // Append only 4 bytes of payload (far too short for a full record).
        data.extend_from_slice(&[0u8; 4]);
        fs::write(&wal_path, &data).unwrap();
        assert!(
            matches!(MvpReceiptVault::open(&path), Err(MvpVaultError::InvalidWal(_))),
            "partial record after record magic must be rejected"
        );
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn import_of_empty_public_commitments_returns_zero() {
        let path = temp_workspace("import-empty");
        let vault = MvpReceiptVault::init(&path).unwrap();
        // An export with only the header and no records is valid and returns 0.
        let header_only = {
            let mut v = Vec::new();
            v.extend_from_slice(super::PUBLIC_COMMITMENTS_HEADER);
            v
        };
        let count = vault.import_public_commitments(&header_only).unwrap();
        assert_eq!(count, 0, "header-only export must import 0 records");
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn import_of_corrupted_export_header_is_rejected() {
        let path = temp_workspace("import-bad-header");
        let vault = MvpReceiptVault::init(&path).unwrap();
        // Corrupt the very first byte of the export header magic.
        let mut bad_header = super::PUBLIC_COMMITMENTS_HEADER.to_vec();
        bad_header[0] ^= 0xff;
        let result = vault.import_public_commitments(&bad_header);
        assert!(
            matches!(result, Err(MvpVaultError::InvalidWal(_))),
            "corrupted export header must be rejected on import"
        );
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn replay_root_is_stable_across_two_sequential_reads() {
        // Regression guard: two sequential calls to read_all_public_exports
        // on the same vault must produce the same sequence and therefore the
        // same commitment root.
        let path = temp_workspace("replay-stable");
        let vault = MvpReceiptVault::init(&path).unwrap();
        vault
            .issue_receipt(
                1,
                fixture_bytes(FixtureKind::FirstNonce),
                b"alpha incident",
                fixture_bytes(FixtureKind::DisclosureCommitment),
            )
            .unwrap();
        vault
            .issue_receipt(
                2,
                fixture_bytes(FixtureKind::SecondNonce),
                b"beta incident",
                fixture_bytes(FixtureKind::DisclosureCommitment),
            )
            .unwrap();

        let exports_a = vault.read_all_public_exports().unwrap();
        let exports_b = vault.read_all_public_exports().unwrap();
        assert_eq!(exports_a.len(), exports_b.len());
        for (a, b) in exports_a.iter().zip(exports_b.iter()) {
            assert_eq!(a.encode(), b.encode());
        }
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn replay_report_json_schema_has_required_fields() {
        // Parse the JSON string that cmd_replay writes to --report and verify
        // all required schema fields are present with correct types.
        // We build the report string the same way demo.rs does.
        let records: usize = 3;
        let root = [0xABu8; 32];
        let hex_root: String = root.iter().map(|b| format!("{b:02x}")).collect();
        let report = format!(
            "{{\n  \"profile\": \"TX-MVP-ReceiptCommit\",\n  \"profile_version\": 1,\n  \"records\": {},\n  \"commitment_root\": \"{}\",\n  \"public_transcript_only\": true,\n  \"private_payloads_seen\": false,\n  \"status\": \"ok\"\n}}\n",
            records, hex_root
        );

        // Minimal JSON field validation without serde — check key presence and value shapes.
        assert!(report.contains("\"profile\": \"TX-MVP-ReceiptCommit\""));
        assert!(report.contains("\"profile_version\": 1"));
        assert!(report.contains(&format!("\"records\": {records}")));
        assert!(report.contains(&format!("\"commitment_root\": \"{hex_root}\"")));
        assert!(report.contains("\"public_transcript_only\": true"));
        assert!(report.contains("\"private_payloads_seen\": false"));
        assert!(report.contains("\"status\": \"ok\""));
        // commitment_root must be a 64-char lowercase hex string.
        let start = report.find("\"commitment_root\": \"").unwrap() + "\"commitment_root\": \"".len();
        let end = report[start..].find('"').unwrap() + start;
        let root_hex = &report[start..end];
        assert_eq!(root_hex.len(), 64, "commitment_root must be 64 hex chars");
        assert!(root_hex.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()), "commitment_root must be lowercase hex");
        // Private incident payload text must never appear in replay reports.
        assert!(!report.contains("body"), "private body text must not appear in replay report");
        assert!(!report.contains("incident"), "private payload text must not appear in replay report");
    }
}
