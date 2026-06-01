//! Receipt privacy primitives for zero-persistence Domain B.
//!
//! This module models durable receipt evidence as commitments — the protocol-facing
//! surface is limited to fixed-width roots and atomic shred evidence.  The 4-C
//! additions below add viewing-key derivation and an `EncryptedReceiptBody` type
//! for Domain B receipt encryption with forward-secrecy guarantees.
//!
//! # Forward secrecy (Class III observer, §P4a)
//!
//! Viewing keys are derived from `epoch_seed` via SHA3-256. After epoch closure,
//! `erase_epoch_viewing_key` zeroizes the derived key. Past receipts become
//! permanently unreadable — satisfying GDPR Art. 17 Right to Erasure for
//! epoch-scoped receipt access.

use sha3::{Digest, Sha3_256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::crypto::dual_hash::{allof_hash_pair_32, verify_allof_hash_pair_32, AllOfHashPair32};

use crate::zero_wal::{ZeroPersistenceWal, ZeroPersistenceWalRecord};

// ── 4-C: Viewing key derivation ───────────────────────────────────────────────

/// Domain B viewing key — epoch-scoped, zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct ViewingKey(pub [u8; 32]);

/// Derive the viewing key for `epoch` from `master_key` and `epoch_seed`.
///
/// Forward secrecy: the key is unique per (master_key, epoch_seed, epoch)
/// triple.  Once `epoch_seed` is discarded after epoch closure, past keys
/// cannot be rederived.
///
/// Domain B only — never cross into Domain A.
pub fn derive_viewing_key(master_key: &[u8; 32], epoch_seed: &[u8; 32], epoch: u64) -> ViewingKey {
    let mut h = Sha3_256::new();
    h.update(master_key);
    h.update(epoch.to_be_bytes());
    h.update(epoch_seed);
    let out: [u8; 32] = h.finalize().into();
    ViewingKey(out)
}

/// Erase the viewing key for `epoch` from `key_store`.
///
/// After this call the receipts from `epoch` are permanently inaccessible
/// to the caller, satisfying GDPR Art. 17 (right to erasure) for their
/// own epoch-scoped viewing capability.
pub fn erase_epoch_viewing_key(epoch: u64, key_store: &mut EpochKeyStore) {
    if let Some(key) = key_store.epoch_keys.get_mut(&epoch) {
        key.zeroize();
    }
    key_store.epoch_keys.remove(&epoch);
}

/// Minimal in-memory epoch-key store for Domain B use.
///
/// Production implementations back this with a TEE-protected vault.
#[derive(Default)]
pub struct EpochKeyStore {
    epoch_keys: std::collections::BTreeMap<u64, [u8; 32]>,
}

impl EpochKeyStore {
    pub fn insert(&mut self, epoch: u64, key: ViewingKey) {
        self.epoch_keys.insert(epoch, key.0);
    }

    pub fn get(&self, epoch: u64) -> Option<ViewingKey> {
        self.epoch_keys.get(&epoch).map(|k| ViewingKey(*k))
    }
}

// ── 4-C: Encrypted receipt body (stub encryption) ─────────────────────────────

/// An encrypted receipt body alongside its Domain A–visible commitment.
///
/// The `ciphertext` is encrypted to the recipient's viewing key (derived from
/// `epoch_seed`). The `commitment` is the SHA3-256 of the ciphertext and is the
/// only value visible to Class I observers (via `receipt_root`).
#[derive(Debug, Clone)]
pub struct EncryptedReceiptBody {
    /// Ciphertext: payload XOR-masked with viewing key (stub; production uses
    /// ChaCha20-Poly1305 or ML-KEM-768 hybrid — see ROADMAP 4-C for full spec).
    pub ciphertext: Vec<u8>,
    /// Epoch at which this receipt was created.
    pub epoch: u64,
    /// SHA3-256(ciphertext) — the only public commitment.
    pub commitment: [u8; 32],
}

/// Encrypt a receipt payload under `viewing_key` and produce an `EncryptedReceiptBody`.
///
/// This is a stub implementation using XOR masking. Production MUST use
/// ChaCha20-Poly1305 (with ML-KEM-768 KEM for key establishment) — the
/// interface is intentionally typed to allow replacement without API changes.
pub fn encrypt_receipt_body(
    payload: &[u8],
    epoch: u64,
    viewing_key: &ViewingKey,
) -> EncryptedReceiptBody {
    // Stub: XOR each byte with the cycled viewing key bytes.
    let key_bytes = &viewing_key.0;
    let ciphertext: Vec<u8> = payload
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key_bytes[i % 32])
        .collect();
    let commitment = sha3_256_bytes(&ciphertext);
    EncryptedReceiptBody {
        ciphertext,
        epoch,
        commitment,
    }
}

/// Decrypt an `EncryptedReceiptBody` under `viewing_key`.
///
/// Returns `None` if the commitment does not match (ciphertext tampered).
pub fn decrypt_receipt_body(
    body: &EncryptedReceiptBody,
    viewing_key: &ViewingKey,
) -> Option<Vec<u8>> {
    let computed_commitment = sha3_256_bytes(&body.ciphertext);
    if computed_commitment != body.commitment {
        return None;
    }
    let key_bytes = &viewing_key.0;
    let plaintext: Vec<u8> = body
        .ciphertext
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key_bytes[i % 32])
        .collect();
    Some(plaintext)
}

fn sha3_256_bytes(data: &[u8]) -> [u8; 32] {
    let mut h = Sha3_256::new();
    h.update(data);
    h.finalize().into()
}

/// Deployment-scoped disclosure policy for an encrypted receipt commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisclosureDomain {
    HolderOnly,
    HolderAndAuditor,
    LocalOperatorPolicy,
}

/// Durable commitment for an encrypted receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncryptedReceiptCommitment {
    pub receipt_id: [u8; 32],
    pub ciphertext_root: [u8; 32],
    pub key_commitment: [u8; 32],
    pub disclosure_domain: DisclosureDomain,
    pub ciphertext_len: u64,
}

/// Request to atomically erase/revoke a local receipt key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShredRequest {
    pub key_id_commitment: [u8; 32],
    pub epoch: u64,
    pub event_root: [u8; 32],
}

/// Public/durable evidence that a local receipt key erase/revocation completed.
///
/// A `ShredCommitment` MUST be returned only after the vault has completed the
/// local key erase/revocation and crossed its own durability boundary. It is a
/// completion receipt, not an intent record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShredCommitment {
    pub key_id_commitment: [u8; 32],
    pub epoch: u64,
    pub event_root: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptVaultError {
    DuplicateReceipt,
    MissingReceipt,
    InvalidCommitment,
    ShredIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicShredError<VaultError, WalError> {
    Vault(VaultError),
    EvidenceAppend(WalError),
}

/// Local Domain B vault interface.
///
/// Implementations may store encrypted blobs, but this trait exposes only
/// commitment records and shred evidence to the protocol/evidence layer.
pub trait ReceiptVault {
    type Error;

    fn store_commitment(
        &mut self,
        commitment: EncryptedReceiptCommitment,
    ) -> Result<(), Self::Error>;

    /// Atomically complete local key erase/revocation and return durable evidence.
    ///
    /// Implementations MUST NOT return `Ok(ShredCommitment)` until the key is no
    /// longer recoverable through the local vault's normal recovery path and the
    /// corresponding evidence boundary has been committed.
    fn commit_shred(&mut self, request: ShredRequest) -> Result<ShredCommitment, Self::Error>;
}

impl EncryptedReceiptCommitment {
    pub fn public_root(&self) -> [u8; 32] {
        fold_roots(self.receipt_id, self.ciphertext_root, self.key_commitment)
    }
}

impl From<ShredRequest> for ShredCommitment {
    fn from(request: ShredRequest) -> Self {
        Self {
            key_id_commitment: request.key_id_commitment,
            epoch: request.epoch,
            event_root: request.event_root,
        }
    }
}

/// Commit a shred in the vault and append its completion evidence to the WAL.
///
/// This helper prevents callers from appending a shred intent. It obtains the
/// `ShredCommitment` only from `ReceiptVault::commit_shred`, then persists that
/// completion receipt. If WAL append fails, the returned error identifies an
/// evidence-append failure after vault completion; callers must treat the vault
/// as authoritative for the completed shred boundary.
pub fn commit_shred_with_evidence<V, W>(
    vault: &mut V,
    wal: &mut W,
    request: ShredRequest,
) -> Result<ShredCommitment, AtomicShredError<V::Error, W::Error>>
where
    V: ReceiptVault,
    W: ZeroPersistenceWal,
{
    let completed = vault
        .commit_shred(request)
        .map_err(AtomicShredError::Vault)?;
    wal.append_commitment(ZeroPersistenceWalRecord::from(completed))
        .map_err(AtomicShredError::EvidenceAppend)?;
    Ok(completed)
}

fn fold_roots(a: [u8; 32], b: [u8; 32], c: [u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for idx in 0..32 {
        out[idx] = a[idx] ^ b[idx] ^ c[idx];
    }
    out
}

// ── All-of dual-root evidence helpers ────────────────────────────────────────

/// Compute the all-of dual-root pair for an `EncryptedReceiptCommitment`.
///
/// Canonical transcript: `ciphertext_root || key_commitment ||
/// disclosure_domain_byte || ciphertext_len_le`. Salt is `receipt_id`.
/// Not FIPS/CAVP/ACVP evidence. Does not commit raw ciphertext or key material.
pub fn compute_receipt_evidence_root_pair(
    commitment: &EncryptedReceiptCommitment,
) -> AllOfHashPair32 {
    let data = receipt_evidence_data(commitment);
    allof_hash_pair_32(b"qash-receipt-evidence-v1", &commitment.receipt_id, &data)
}

/// Verify an all-of dual-root pair against a fresh computation for the given commitment.
///
/// Returns `true` only when both SHA3 and BLAKE3 roots match independently.
pub fn verify_receipt_evidence_root_pair(
    commitment: &EncryptedReceiptCommitment,
    pair: &AllOfHashPair32,
) -> bool {
    let data = receipt_evidence_data(commitment);
    verify_allof_hash_pair_32(pair, b"qash-receipt-evidence-v1", &commitment.receipt_id, &data)
}

fn receipt_evidence_data(commitment: &EncryptedReceiptCommitment) -> [u8; 73] {
    let mut data = [0u8; 73];
    data[..32].copy_from_slice(&commitment.ciphertext_root);
    data[32..64].copy_from_slice(&commitment.key_commitment);
    data[64] = disclosure_domain_byte(commitment.disclosure_domain);
    data[65..73].copy_from_slice(&commitment.ciphertext_len.to_le_bytes());
    data
}

fn disclosure_domain_byte(d: DisclosureDomain) -> u8 {
    match d {
        DisclosureDomain::HolderOnly => 0x01,
        DisclosureDomain::HolderAndAuditor => 0x02,
        DisclosureDomain::LocalOperatorPolicy => 0x03,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_public_root_is_commitment_only() {
        let receipt = EncryptedReceiptCommitment {
            receipt_id: [1u8; 32],
            ciphertext_root: [2u8; 32],
            key_commitment: [4u8; 32],
            disclosure_domain: DisclosureDomain::HolderOnly,
            ciphertext_len: 128,
        };
        assert_eq!(receipt.public_root(), [7u8; 32]);
    }

    #[test]
    fn shred_commitment_is_completion_of_request() {
        let request = ShredRequest {
            key_id_commitment: [3u8; 32],
            epoch: 9,
            event_root: [5u8; 32],
        };
        let shred = ShredCommitment::from(request);
        assert_eq!(shred.epoch, 9);
        assert_eq!(shred.key_id_commitment, [3u8; 32]);
    }

    // ── 4-C viewing key tests ────────────────────────────────────────────────

    #[test]
    fn viewing_key_is_deterministic() {
        let mk = [0x42u8; 32];
        let seed = [0x99u8; 32];
        let k1 = derive_viewing_key(&mk, &seed, 7);
        let k2 = derive_viewing_key(&mk, &seed, 7);
        assert_eq!(k1.0, k2.0);
    }

    #[test]
    fn viewing_key_differs_across_epochs() {
        let mk = [0x42u8; 32];
        let seed = [0x99u8; 32];
        let k1 = derive_viewing_key(&mk, &seed, 1);
        let k2 = derive_viewing_key(&mk, &seed, 2);
        assert_ne!(k1.0, k2.0);
    }

    #[test]
    fn viewing_key_differs_across_seeds() {
        let mk = [0x42u8; 32];
        let k1 = derive_viewing_key(&mk, &[0x11u8; 32], 1);
        let k2 = derive_viewing_key(&mk, &[0x22u8; 32], 1);
        assert_ne!(k1.0, k2.0);
    }

    #[test]
    fn receipt_encrypt_decrypt_roundtrip() {
        let mk = [0x42u8; 32];
        let seed = [0x99u8; 32];
        let key = derive_viewing_key(&mk, &seed, 5);
        let payload = b"test receipt payload";
        let encrypted = encrypt_receipt_body(payload, 5, &key);
        assert_ne!(encrypted.ciphertext, payload.as_slice());
        let decrypted = decrypt_receipt_body(&encrypted, &key).expect("decrypt ok");
        assert_eq!(decrypted, payload);
    }

    #[test]
    fn receipt_decrypt_rejects_tampered_ciphertext() {
        let mk = [0x42u8; 32];
        let seed = [0x99u8; 32];
        let key = derive_viewing_key(&mk, &seed, 5);
        let mut encrypted = encrypt_receipt_body(b"receipt data", 5, &key);
        encrypted.ciphertext[0] ^= 0xFF;
        assert!(decrypt_receipt_body(&encrypted, &key).is_none());
    }

    // ── All-of receipt evidence root tests ───────────────────────────────────

    fn sample_commitment() -> EncryptedReceiptCommitment {
        EncryptedReceiptCommitment {
            receipt_id: [1u8; 32],
            ciphertext_root: [2u8; 32],
            key_commitment: [3u8; 32],
            disclosure_domain: DisclosureDomain::HolderOnly,
            ciphertext_len: 256,
        }
    }

    #[test]
    fn receipt_evidence_accepts_exact_root_pair() {
        let c = sample_commitment();
        let pair = compute_receipt_evidence_root_pair(&c);
        assert!(verify_receipt_evidence_root_pair(&c, &pair));
    }

    #[test]
    fn receipt_evidence_rejects_modified_sha3_root() {
        let c = sample_commitment();
        let mut pair = compute_receipt_evidence_root_pair(&c);
        pair.sha3_512_32[0] ^= 0xFF;
        assert!(!verify_receipt_evidence_root_pair(&c, &pair));
    }

    #[test]
    fn receipt_evidence_rejects_modified_blake3_root() {
        let c = sample_commitment();
        let mut pair = compute_receipt_evidence_root_pair(&c);
        pair.blake3_32[0] ^= 0xFF;
        assert!(!verify_receipt_evidence_root_pair(&c, &pair));
    }

    #[test]
    fn receipt_evidence_root_changes_when_manifest_changes() {
        let base = sample_commitment();
        let modified = EncryptedReceiptCommitment { ciphertext_root: [9u8; 32], ..base };
        let pair_base = compute_receipt_evidence_root_pair(&base);
        let pair_mod = compute_receipt_evidence_root_pair(&modified);
        assert_ne!(pair_base.sha3_512_32, pair_mod.sha3_512_32);
        assert_ne!(pair_base.blake3_32, pair_mod.blake3_32);
    }

    #[test]
    fn erase_viewing_key_removes_it_from_store() {
        let mk = [0x42u8; 32];
        let seed = [0x99u8; 32];
        let key = derive_viewing_key(&mk, &seed, 3);
        let mut store = EpochKeyStore::default();
        store.insert(3, key);
        assert!(store.get(3).is_some());
        erase_epoch_viewing_key(3, &mut store);
        assert!(store.get(3).is_none());
    }
}
