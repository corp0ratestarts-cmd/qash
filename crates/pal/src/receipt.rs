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

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305,
};
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

// ── 4-C: Encrypted receipt body (ChaCha20-Poly1305 AEAD) ──────────────────────

/// An encrypted receipt body alongside its Domain A–visible commitment.
///
/// The `ciphertext` is encrypted with ChaCha20-Poly1305 keyed to `viewing_key`.
/// The AEAD tag is appended to the ciphertext (last 16 bytes). Nonce and
/// associated data are deterministically derived from public inputs only —
/// no secret material appears in the nonce.
///
/// The `commitment` is SHA3-256(ciphertext || nonce || associated_data) and is
/// the only value visible to Class I observers (via `receipt_root`).
#[derive(Debug, Clone)]
pub struct EncryptedReceiptBody {
    /// Ciphertext with AEAD tag appended (len = plaintext_len + 16).
    pub ciphertext: Vec<u8>,
    /// Epoch at which this receipt was created.
    pub epoch: u64,
    /// 12-byte AEAD nonce, deterministically derived from public inputs.
    pub nonce: [u8; 12],
    /// SHA3-256(ciphertext || nonce || associated_data).
    pub commitment: [u8; 32],
}

/// Encrypt a receipt payload under `viewing_key` using ChaCha20-Poly1305 AEAD.
///
/// The nonce is derived deterministically from public inputs only:
///   SHA3-256("qash-receipt-aead-nonce-v1" || receipt_id || epoch_le ||
///            disclosure_domain_byte)[0..12]
///
/// Associated data (authenticated, not encrypted):
///   "qash-receipt-aead-ad-v1" || receipt_id || epoch_le ||
///   disclosure_domain_byte || ciphertext_len_le8
///
/// Nonce uniqueness is guaranteed when (receipt_id, epoch, disclosure_domain)
/// is unique per plaintext, which the protocol enforces by construction.
///
/// Panics only if the underlying AEAD implementation is broken (never in
/// normal operation with valid key material).
pub fn encrypt_receipt_body(
    payload: &[u8],
    receipt_id: &[u8; 32],
    epoch: u64,
    disclosure_domain: DisclosureDomain,
    viewing_key: &ViewingKey,
) -> EncryptedReceiptBody {
    let nonce_bytes = derive_receipt_nonce(receipt_id, epoch, disclosure_domain);
    let ciphertext_len = (payload.len() + 16) as u64;
    let ad = receipt_aead_associated_data(receipt_id, epoch, disclosure_domain, ciphertext_len);

    let cipher = ChaCha20Poly1305::new_from_slice(&viewing_key.0)
        .expect("viewing key is always 32 bytes");
    let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, Payload { msg: payload, aad: &ad })
        .expect("ChaCha20-Poly1305 encrypt must not fail with valid key");

    let commitment = compute_receipt_commitment(&ciphertext, &nonce_bytes, &ad);
    EncryptedReceiptBody {
        ciphertext,
        epoch,
        nonce: nonce_bytes,
        commitment,
    }
}

/// Decrypt an `EncryptedReceiptBody` under `viewing_key`.
///
/// Returns `None` on any AEAD authentication failure (tampered ciphertext,
/// wrong key, wrong receipt_id/epoch/disclosure_domain) or commitment mismatch.
pub fn decrypt_receipt_body(
    body: &EncryptedReceiptBody,
    receipt_id: &[u8; 32],
    disclosure_domain: DisclosureDomain,
    viewing_key: &ViewingKey,
) -> Option<Vec<u8>> {
    let ciphertext_len = body.ciphertext.len() as u64;
    let ad = receipt_aead_associated_data(receipt_id, body.epoch, disclosure_domain, ciphertext_len);

    let computed_commitment = compute_receipt_commitment(&body.ciphertext, &body.nonce, &ad);
    if computed_commitment != body.commitment {
        return None;
    }

    let cipher = ChaCha20Poly1305::new_from_slice(&viewing_key.0).ok()?;
    let nonce = chacha20poly1305::Nonce::from_slice(&body.nonce);
    cipher
        .decrypt(nonce, Payload { msg: &body.ciphertext, aad: &ad })
        .ok()
}

fn derive_receipt_nonce(
    receipt_id: &[u8; 32],
    epoch: u64,
    disclosure_domain: DisclosureDomain,
) -> [u8; 12] {
    let mut h = Sha3_256::new();
    h.update(b"qash-receipt-aead-nonce-v1");
    h.update(receipt_id);
    h.update(epoch.to_le_bytes());
    h.update([disclosure_domain_byte(disclosure_domain)]);
    let out: [u8; 32] = h.finalize().into();
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&out[..12]);
    nonce
}

fn receipt_aead_associated_data(
    receipt_id: &[u8; 32],
    epoch: u64,
    disclosure_domain: DisclosureDomain,
    ciphertext_len: u64,
) -> Vec<u8> {
    let mut ad = Vec::with_capacity(24 + 32 + 8 + 1 + 8);
    ad.extend_from_slice(b"qash-receipt-aead-ad-v1\0");
    ad.extend_from_slice(receipt_id);
    ad.extend_from_slice(&epoch.to_le_bytes());
    ad.push(disclosure_domain_byte(disclosure_domain));
    ad.extend_from_slice(&ciphertext_len.to_le_bytes());
    ad
}

fn disclosure_domain_byte(domain: DisclosureDomain) -> u8 {
    match domain {
        DisclosureDomain::HolderOnly => 0x01,
        DisclosureDomain::HolderAndAuditor => 0x02,
        DisclosureDomain::LocalOperatorPolicy => 0x03,
    }
}

fn compute_receipt_commitment(ciphertext: &[u8], nonce: &[u8; 12], ad: &[u8]) -> [u8; 32] {
    let mut h = Sha3_256::new();
    h.update(ciphertext);
    h.update(nonce);
    h.update(ad);
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

/// Compute an independent dual-root all-of binding over receipt evidence metadata.
///
/// Binds `receipt_id`, `ciphertext_root`, `key_commitment`, and `ciphertext_len`.
/// Does not include raw ciphertext or key material.
/// Domain B only. Not FIPS/CAVP/ACVP evidence.
pub fn compute_receipt_evidence_root_pair(
    commitment: &EncryptedReceiptCommitment,
) -> AllOfHashPair32 {
    let mut data = [0u8; 32 + 32 + 8];
    data[..32].copy_from_slice(&commitment.ciphertext_root);
    data[32..64].copy_from_slice(&commitment.key_commitment);
    data[64..].copy_from_slice(&commitment.ciphertext_len.to_le_bytes());
    allof_hash_pair_32(
        b"qash-receipt-evidence-v1",
        &commitment.receipt_id,
        &data,
    )
}

/// Verify an `AllOfHashPair32` against the receipt evidence metadata.
///
/// Returns `true` only when both arms independently match.
pub fn verify_receipt_evidence_root_pair(
    commitment: &EncryptedReceiptCommitment,
    pair: &AllOfHashPair32,
) -> bool {
    let mut data = [0u8; 32 + 32 + 8];
    data[..32].copy_from_slice(&commitment.ciphertext_root);
    data[32..64].copy_from_slice(&commitment.key_commitment);
    data[64..].copy_from_slice(&commitment.ciphertext_len.to_le_bytes());
    verify_allof_hash_pair_32(
        pair,
        b"qash-receipt-evidence-v1",
        &commitment.receipt_id,
        &data,
    )
}

fn fold_roots(a: [u8; 32], b: [u8; 32], c: [u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for idx in 0..32 {
        out[idx] = a[idx] ^ b[idx] ^ c[idx];
    }
    out
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
        let receipt_id = [0xAAu8; 32];
        let payload = b"test receipt payload";
        let encrypted = encrypt_receipt_body(payload, &receipt_id, 5, DisclosureDomain::HolderOnly, &key);
        assert_ne!(&encrypted.ciphertext[..payload.len()], payload.as_slice());
        let decrypted = decrypt_receipt_body(&encrypted, &receipt_id, DisclosureDomain::HolderOnly, &key)
            .expect("decrypt ok");
        assert_eq!(decrypted, payload);
    }

    #[test]
    fn receipt_decrypt_rejects_tampered_ciphertext() {
        let mk = [0x42u8; 32];
        let seed = [0x99u8; 32];
        let key = derive_viewing_key(&mk, &seed, 5);
        let receipt_id = [0xBBu8; 32];
        let mut encrypted = encrypt_receipt_body(b"receipt data", &receipt_id, 5, DisclosureDomain::HolderOnly, &key);
        encrypted.ciphertext[0] ^= 0xFF;
        assert!(decrypt_receipt_body(&encrypted, &receipt_id, DisclosureDomain::HolderOnly, &key).is_none());
    }

    #[test]
    fn receipt_decrypt_rejects_wrong_key() {
        let mk1 = [0x42u8; 32];
        let mk2 = [0x43u8; 32];
        let seed = [0x99u8; 32];
        let key1 = derive_viewing_key(&mk1, &seed, 5);
        let key2 = derive_viewing_key(&mk2, &seed, 5);
        let receipt_id = [0xCCu8; 32];
        let encrypted = encrypt_receipt_body(b"secret payload", &receipt_id, 5, DisclosureDomain::HolderOnly, &key1);
        assert!(decrypt_receipt_body(&encrypted, &receipt_id, DisclosureDomain::HolderOnly, &key2).is_none());
    }

    #[test]
    fn receipt_nonce_changes_with_receipt_id() {
        let receipt_a = [0x01u8; 32];
        let receipt_b = [0x02u8; 32];
        let nonce_a = derive_receipt_nonce(&receipt_a, 1, DisclosureDomain::HolderOnly);
        let nonce_b = derive_receipt_nonce(&receipt_b, 1, DisclosureDomain::HolderOnly);
        assert_ne!(nonce_a, nonce_b);
    }

    #[test]
    fn receipt_nonce_changes_with_epoch() {
        let receipt_id = [0x01u8; 32];
        let nonce_e1 = derive_receipt_nonce(&receipt_id, 1, DisclosureDomain::HolderOnly);
        let nonce_e2 = derive_receipt_nonce(&receipt_id, 2, DisclosureDomain::HolderOnly);
        assert_ne!(nonce_e1, nonce_e2);
    }

    #[test]
    fn receipt_nonce_changes_with_disclosure_domain() {
        let receipt_id = [0x01u8; 32];
        let nonce_h = derive_receipt_nonce(&receipt_id, 1, DisclosureDomain::HolderOnly);
        let nonce_a = derive_receipt_nonce(&receipt_id, 1, DisclosureDomain::HolderAndAuditor);
        let nonce_l = derive_receipt_nonce(&receipt_id, 1, DisclosureDomain::LocalOperatorPolicy);
        assert_ne!(nonce_h, nonce_a);
        assert_ne!(nonce_h, nonce_l);
        assert_ne!(nonce_a, nonce_l);
    }

    #[test]
    fn receipt_decrypt_rejects_wrong_disclosure_domain() {
        let mk = [0x42u8; 32];
        let seed = [0x99u8; 32];
        let key = derive_viewing_key(&mk, &seed, 5);
        let receipt_id = [0xDDu8; 32];
        let encrypted = encrypt_receipt_body(b"private data", &receipt_id, 5, DisclosureDomain::HolderOnly, &key);
        assert!(decrypt_receipt_body(&encrypted, &receipt_id, DisclosureDomain::HolderAndAuditor, &key).is_none());
    }

    #[test]
    fn receipt_decrypt_rejects_wrong_receipt_id() {
        let mk = [0x42u8; 32];
        let seed = [0x99u8; 32];
        let key = derive_viewing_key(&mk, &seed, 5);
        let receipt_id_a = [0xEEu8; 32];
        let receipt_id_b = [0xFFu8; 32];
        let encrypted = encrypt_receipt_body(b"private data", &receipt_id_a, 5, DisclosureDomain::HolderOnly, &key);
        assert!(decrypt_receipt_body(&encrypted, &receipt_id_b, DisclosureDomain::HolderOnly, &key).is_none());
    }

    fn test_hash(seed: u8) -> [u8; 32] {
        core::array::from_fn(|i| seed.wrapping_add(i as u8))
    }

    fn sample_commitment() -> EncryptedReceiptCommitment {
        EncryptedReceiptCommitment {
            receipt_id: test_hash(0x01),
            ciphertext_root: test_hash(0x02),
            key_commitment: test_hash(0x03),
            disclosure_domain: DisclosureDomain::HolderOnly,
            ciphertext_len: 256,
        }
    }

    #[test]
    fn receipt_evidence_root_pair_accepts_valid() {
        let c = sample_commitment();
        let pair = compute_receipt_evidence_root_pair(&c);
        assert!(verify_receipt_evidence_root_pair(&c, &pair));
    }

    #[test]
    fn receipt_evidence_root_pair_rejects_tampered_sha3() {
        let c = sample_commitment();
        let mut pair = compute_receipt_evidence_root_pair(&c);
        pair.sha3_512_32 = [0u8; 32];
        assert!(!verify_receipt_evidence_root_pair(&c, &pair));
    }

    #[test]
    fn receipt_evidence_root_pair_rejects_tampered_blake3() {
        let c = sample_commitment();
        let mut pair = compute_receipt_evidence_root_pair(&c);
        pair.blake3_32 = [0u8; 32];
        assert!(!verify_receipt_evidence_root_pair(&c, &pair));
    }

    #[test]
    fn receipt_evidence_root_pair_changes_when_metadata_changes() {
        let c1 = sample_commitment();
        let mut c2 = sample_commitment();
        c2.ciphertext_len = 512;
        assert_ne!(
            compute_receipt_evidence_root_pair(&c1),
            compute_receipt_evidence_root_pair(&c2)
        );
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
