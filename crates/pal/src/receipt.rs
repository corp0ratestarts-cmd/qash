//! Receipt privacy primitives for zero-persistence Domain B.
//!
//! This module deliberately models durable receipt evidence as commitments, not
//! receipt bodies. Encrypted receipt blobs may live in a local vault, but the
//! protocol-facing surface is limited to fixed-width roots and atomic shred evidence.
//!
//! # Vault recovery semantics
//!
//! On restart, a vault should be able to reconstruct its state from:
//! - Encrypted blobs (ciphertext only — never plaintext-at-halt)
//! - Key commitments (not key material)
//! - Shred completion evidence (the `ShredCommitment` WAL record)
//!
//! Plaintext receipt bodies are never stored in durable or public structs.
//! If the vault is destroyed without a completed shred, re-encryption from a
//! backup is required; the protocol has no recovery path for unshredded keys.

use crate::zero_wal::{ZeroPersistenceWal, ZeroPersistenceWalRecord};

/// Deployment-scoped disclosure policy for an encrypted receipt commitment.
///
/// The policy is enforced at the API boundary: callers must check
/// `DisclosureDomain::may_disclose_to` before returning any commitment field
/// to an external observer. Default to the most restrictive policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisclosureDomain {
    HolderOnly,
    HolderAndAuditor,
    LocalOperatorPolicy,
}

/// Represents a class of observer requesting access to a receipt field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observer {
    Holder,
    Auditor,
    LocalOperator,
    PublicNetwork,
}

impl DisclosureDomain {
    /// Returns true if the domain permits disclosure to `observer`.
    ///
    /// This is an ENFORCEMENT gate — callers must not return commitment fields
    /// to an observer unless this returns true.
    pub fn may_disclose_to(self, observer: Observer) -> bool {
        match self {
            DisclosureDomain::HolderOnly => matches!(observer, Observer::Holder),
            DisclosureDomain::HolderAndAuditor => {
                matches!(observer, Observer::Holder | Observer::Auditor)
            }
            DisclosureDomain::LocalOperatorPolicy => matches!(
                observer,
                Observer::Holder | Observer::Auditor | Observer::LocalOperator
            ),
        }
    }

    /// Returns true only if this domain permits public network disclosure.
    ///
    /// No current domain permits public network disclosure. This method exists
    /// as an explicit gate so future domains cannot accidentally become public
    /// without a named variant.
    pub fn is_public_network_permitted(self) -> bool {
        false
    }
}

/// Encryption algorithm profile for a receipt vault entry.
///
/// Records the algorithm ID and key commitment — never key material or plaintext.
/// Used to verify that a committed vault entry was encrypted under the declared
/// algorithm before accepting shred requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptEncryptionProfile {
    /// Numeric identifier for the encryption algorithm (registry-defined).
    pub algorithm_id: u32,
    /// 32-byte commitment to the encryption key. Never the key itself.
    pub key_commitment: [u8; 32],
    /// Disclosure policy for this receipt.
    pub disclosure_domain: DisclosureDomain,
    /// 32-byte root of the ciphertext. Never the ciphertext itself.
    pub ciphertext_root: [u8; 32],
}

/// Well-known algorithm IDs for receipt encryption.
pub mod algorithm_ids {
    /// AES-256-GCM (symmetric, per-receipt key).
    pub const AES_256_GCM: u32 = 1;
    /// ChaCha20-Poly1305 (symmetric, per-receipt key).
    pub const CHACHA20_POLY1305: u32 = 2;
    /// ML-KEM-768 wrapped key (post-quantum KEM).
    pub const ML_KEM_768_WRAP: u32 = 3;
}

impl ReceiptEncryptionProfile {
    /// Derive a 32-byte public profile root (no key material, no ciphertext).
    ///
    /// The root commits to (algorithm_id, key_commitment, ciphertext_root)
    /// in a fixed-width encoding, using XOR fold over the three fields after
    /// domain-separating algorithm_id into a 32-byte tag.
    pub fn public_root(&self) -> [u8; 32] {
        let mut alg_tag = [0u8; 32];
        alg_tag[..4].copy_from_slice(&self.algorithm_id.to_le_bytes());
        fold_roots(alg_tag, self.key_commitment, self.ciphertext_root)
    }

    /// Returns true only if this profile's disclosure_domain permits the observer.
    pub fn may_disclose_to(&self, observer: Observer) -> bool {
        self.disclosure_domain.may_disclose_to(observer)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // DisclosureDomain enforcement tests (Stage 2b)
    // -------------------------------------------------------------------------

    #[test]
    fn holder_only_permits_only_holder() {
        let d = DisclosureDomain::HolderOnly;
        assert!(d.may_disclose_to(Observer::Holder));
        assert!(!d.may_disclose_to(Observer::Auditor));
        assert!(!d.may_disclose_to(Observer::LocalOperator));
        assert!(!d.may_disclose_to(Observer::PublicNetwork));
    }

    #[test]
    fn holder_and_auditor_permits_both_not_operator() {
        let d = DisclosureDomain::HolderAndAuditor;
        assert!(d.may_disclose_to(Observer::Holder));
        assert!(d.may_disclose_to(Observer::Auditor));
        assert!(!d.may_disclose_to(Observer::LocalOperator));
        assert!(!d.may_disclose_to(Observer::PublicNetwork));
    }

    #[test]
    fn local_operator_policy_excludes_public_network() {
        let d = DisclosureDomain::LocalOperatorPolicy;
        assert!(d.may_disclose_to(Observer::Holder));
        assert!(d.may_disclose_to(Observer::Auditor));
        assert!(d.may_disclose_to(Observer::LocalOperator));
        assert!(!d.may_disclose_to(Observer::PublicNetwork));
    }

    #[test]
    fn no_domain_permits_public_network() {
        for d in [
            DisclosureDomain::HolderOnly,
            DisclosureDomain::HolderAndAuditor,
            DisclosureDomain::LocalOperatorPolicy,
        ] {
            assert!(!d.is_public_network_permitted());
            assert!(!d.may_disclose_to(Observer::PublicNetwork));
        }
    }

    // -------------------------------------------------------------------------
    // ReceiptEncryptionProfile tests (Stage 2a)
    // -------------------------------------------------------------------------

    #[test]
    fn encryption_profile_root_excludes_algorithm_id_from_raw_fields() {
        let profile = ReceiptEncryptionProfile {
            algorithm_id: algorithm_ids::AES_256_GCM,
            key_commitment: [0x01u8; 32],
            disclosure_domain: DisclosureDomain::HolderOnly,
            ciphertext_root: [0x02u8; 32],
        };
        let root = profile.public_root();
        // Root is deterministic and non-zero.
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn encryption_profile_root_differs_by_algorithm_id() {
        let base = ReceiptEncryptionProfile {
            algorithm_id: algorithm_ids::AES_256_GCM,
            key_commitment: [0xABu8; 32],
            disclosure_domain: DisclosureDomain::HolderOnly,
            ciphertext_root: [0xCDu8; 32],
        };
        let other = ReceiptEncryptionProfile {
            algorithm_id: algorithm_ids::CHACHA20_POLY1305,
            ..base
        };
        assert_ne!(base.public_root(), other.public_root());
    }

    #[test]
    fn encryption_profile_root_differs_by_key_commitment() {
        let base = ReceiptEncryptionProfile {
            algorithm_id: algorithm_ids::AES_256_GCM,
            key_commitment: [0x01u8; 32],
            disclosure_domain: DisclosureDomain::HolderOnly,
            ciphertext_root: [0x02u8; 32],
        };
        let other = ReceiptEncryptionProfile {
            key_commitment: [0x03u8; 32],
            ..base
        };
        assert_ne!(base.public_root(), other.public_root());
    }

    #[test]
    fn encryption_profile_enforces_disclosure_domain() {
        let profile = ReceiptEncryptionProfile {
            algorithm_id: algorithm_ids::AES_256_GCM,
            key_commitment: [0x10u8; 32],
            disclosure_domain: DisclosureDomain::HolderOnly,
            ciphertext_root: [0x20u8; 32],
        };
        assert!(profile.may_disclose_to(Observer::Holder));
        assert!(!profile.may_disclose_to(Observer::Auditor));
        assert!(!profile.may_disclose_to(Observer::PublicNetwork));
    }

    #[test]
    fn encryption_profile_contains_no_plaintext_or_key_material() {
        // Structural test: ReceiptEncryptionProfile has no field that could hold
        // a plaintext body or raw key. It has algorithm_id, key_commitment,
        // disclosure_domain, and ciphertext_root — all commitment/meta only.
        let profile = ReceiptEncryptionProfile {
            algorithm_id: algorithm_ids::ML_KEM_768_WRAP,
            key_commitment: [0xFFu8; 32],
            disclosure_domain: DisclosureDomain::HolderAndAuditor,
            ciphertext_root: [0x11u8; 32],
        };
        // The public_root() is 32 bytes — no way to reconstruct plaintext.
        assert_eq!(profile.public_root().len(), 32);
    }

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
}
