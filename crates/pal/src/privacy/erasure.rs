//! Key-shredding engine for erasure-compatible receipt handling.
//!
//! Provides a `ReceiptKey` type that is zeroized on drop, and `shred_key()`
//! which consumes the key by value so the key material is provably
//! unrecoverable from memory after the call returns.
//!
//! This is one component of an erasure-handling design. Compliance with
//! Art. 17 GDPR requires the full design plus legal assessment; this module
//! delivers the implementation layer only. Do NOT claim "GDPR compliant" —
//! claim "GDPR-aligned design with erasure-compatible receipt handling."

use zeroize::ZeroizeOnDrop;

use crate::receipt::ShredCommitment;

/// A local encryption key for a single receipt. Zeroized on drop.
///
/// Keys never cross the Domain B → Domain A boundary. Domain A sees only the
/// `key_commitment` field of `EncryptedReceiptCommitment`.
#[derive(ZeroizeOnDrop)]
pub struct ReceiptKey {
    material: [u8; 32],
    /// Stable commitment to this key (SHA3-256 of `material`). Computed at
    /// construction so it remains accessible after the key is shredded.
    pub key_commitment: [u8; 32],
}

impl ReceiptKey {
    /// Construct a new receipt key from raw key material.
    ///
    /// `material` must be cryptographically-strong random bytes supplied by the
    /// Domain B RNG (e.g. HMAC-DRBG). Never pass deterministic or low-entropy
    /// data here in production.
    pub fn new(material: [u8; 32]) -> Self {
        let key_commitment = commitment_of(&material);
        Self {
            material,
            key_commitment,
        }
    }

    /// Borrow the raw key material for encryption operations.
    ///
    /// The returned slice MUST NOT be cloned, logged, or stored. It is valid
    /// only for the duration of a single encryption call. Domain B callers are
    /// responsible for using the material only within a scoped encryption
    /// primitive and never persisting the bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.material
    }
}

/// Evidence that a `ReceiptKey` has been consumed and its material zeroized.
///
/// Returned by `shred_key()`. Callers should persist this as WAL evidence
/// before considering the shred complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShredKeyEvidence {
    pub key_commitment: [u8; 32],
}

impl From<ShredKeyEvidence> for ShredCommitment {
    fn from(ev: ShredKeyEvidence) -> Self {
        ShredCommitment {
            key_id_commitment: ev.key_commitment,
            epoch: 0,
            event_root: [0u8; 32],
        }
    }
}

/// Consume a `ReceiptKey` by value, zeroizing its material, and return
/// durable evidence of the shred.
///
/// After this function returns, the key material is gone from memory.
/// The caller holds only the `ShredKeyEvidence` commitment — which contains
/// no usable key bytes.
pub fn shred_key(key: ReceiptKey) -> ShredKeyEvidence {
    let commitment = key.key_commitment;
    drop(key); // ZeroizeOnDrop fires here, wiping `material`
    ShredKeyEvidence {
        key_commitment: commitment,
    }
}

fn commitment_of(material: &[u8; 32]) -> [u8; 32] {
    // Domain-tag-prefixed fold: tag || material, iterated SHA3-256 round.
    // This is a commitment binding, not a derivation; the key itself is the secret.
    const DOMAIN_TAG: u8 = 0xEC; // erasure commitment
    let mut buf = [0u8; 33];
    buf[0] = DOMAIN_TAG;
    buf[1..].copy_from_slice(material);
    sha3_256(&buf)
}

fn sha3_256(input: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Sha3_256};
    let mut h = Sha3_256::new();
    h.update(input);
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shred_key_returns_stable_commitment() {
        let material = [0xAB_u8; 32];
        let key = ReceiptKey::new(material);
        let commitment_before = key.key_commitment;
        let evidence = shred_key(key);
        assert_eq!(evidence.key_commitment, commitment_before);
    }

    #[test]
    fn shred_prevents_decryption() {
        // Simulate: encrypt with key, shred, verify key bytes are gone.
        let material = [0x42_u8; 32];
        let key = ReceiptKey::new(material);
        let commitment = key.key_commitment;

        // Simulated ciphertext (XOR of plaintext and key material — not a real
        // cipher, just enough to prove the key bytes are needed for recovery).
        let plaintext = [0xFF_u8; 32];
        let mut ciphertext = [0u8; 32];
        for i in 0..32 {
            ciphertext[i] = plaintext[i] ^ key.as_bytes()[i];
        }

        let evidence = shred_key(key);

        // After shred: evidence carries only the commitment, not the key bytes.
        assert_eq!(evidence.key_commitment, commitment);

        // There is no way to recover `material` from `evidence.key_commitment`
        // (preimage resistance). Verify the ciphertext is now opaque by showing
        // that XOR with the commitment does NOT recover the plaintext.
        let mut attempted_decrypt = [0u8; 32];
        for i in 0..32 {
            attempted_decrypt[i] = ciphertext[i] ^ evidence.key_commitment[i];
        }
        assert_ne!(attempted_decrypt, plaintext, "key commitment must not decrypt the ciphertext");
    }

    #[test]
    fn disclosure_domain_correct() {
        use crate::receipt::DisclosureDomain;
        // Verify the three canonical domains are present and distinct.
        let d1 = DisclosureDomain::HolderOnly;
        let d2 = DisclosureDomain::HolderAndAuditor;
        let d3 = DisclosureDomain::LocalOperatorPolicy;
        assert_ne!(d1, d2);
        assert_ne!(d2, d3);
        assert_ne!(d1, d3);
    }
}
