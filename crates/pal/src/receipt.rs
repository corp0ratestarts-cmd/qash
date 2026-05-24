//! Receipt privacy primitives for zero-persistence Domain B.
//!
//! This module deliberately models durable receipt evidence as commitments, not
//! receipt bodies. Encrypted receipt blobs may live in a local vault, but the
//! protocol-facing surface is limited to fixed-width roots and atomic shred evidence.

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
}
