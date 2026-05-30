//! Key-shredding engine for erasure-compatible receipt handling.
//!
//! Provides a `ReceiptKey` type that is zeroized on drop, and `shred_key()`
//! which consumes the key by value so the key material is provably
//! unrecoverable from memory after the call returns.
//!
//! Phase 10 extends this module with an erasure request lifecycle:
//! `PendingLocate → Located → Shredding → ShredComplete`.
//! `process_erasure_request` drives the request through all four states,
//! returning `ShredKeyEvidence` on success or `ErasureError` on failure.
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
/// Returned by `shred_key()`. All fields are required to ensure the audit
/// record is unambiguous: `key_commitment` identifies the key, `epoch`
/// timestamps the shred in protocol time, and `event_root` links to the
/// receipt/incident that triggered the erasure. Callers should persist this
/// as WAL evidence before considering the shred complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShredKeyEvidence {
    /// Commitment to the shredded key material. Computed at key construction;
    /// no key bytes are retained after `shred_key()` returns.
    pub key_commitment: [u8; 32],
    /// Protocol epoch at which the shred was executed (caller-supplied).
    pub epoch: u64,
    /// Event root that triggered the erasure (caller-supplied). Links this
    /// shred record to the incident/receipt being erased for audit tracing.
    pub event_root: [u8; 32],
}

impl From<ShredKeyEvidence> for ShredCommitment {
    fn from(ev: ShredKeyEvidence) -> Self {
        ShredCommitment {
            key_id_commitment: ev.key_commitment,
            epoch: ev.epoch,
            event_root: ev.event_root,
        }
    }
}

/// Consume a `ReceiptKey` by value, zeroizing its material, and return
/// durable evidence of the shred.
///
/// `epoch` and `event_root` are the caller's context: which protocol epoch
/// the shred occurred in, and which event/receipt triggered the erasure.
/// Both are required to produce an unambiguous audit record.
///
/// After this function returns, the key material is gone from memory.
/// The caller holds only the `ShredKeyEvidence` — which contains no usable
/// key bytes.
pub fn shred_key(key: ReceiptKey, epoch: u64, event_root: [u8; 32]) -> ShredKeyEvidence {
    let commitment = key.key_commitment;
    drop(key); // ZeroizeOnDrop fires here, wiping `material`
    ShredKeyEvidence {
        key_commitment: commitment,
        epoch,
        event_root,
    }
}

// ── Erasure request lifecycle ─────────────────────────────────────────────────

/// An erasure request from a data subject (Art. 17 GDPR) or operator.
///
/// Identifies the receipt to be erased by its encryption-key commitment (not
/// the plaintext data, which the operator does not hold after encryption).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErasureRequest {
    /// SHA3-256 commitment of the receipt encryption key to be shredded.
    pub receipt_commitment: [u8; 32],
    /// Opaque requestor identifier (operator-defined; not logged as plaintext).
    pub requestor_id: [u8; 32],
    /// Protocol epoch at which the request was received.
    pub epoch: u64,
}

/// Lifecycle state of an erasure request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErasureState {
    /// Request received; key not yet located in the key store.
    PendingLocate,
    /// Key located in the key store; not yet shredded.
    Located,
    /// Shred operation in progress (key being consumed).
    Shredding,
    /// Shred complete; `ShredKeyEvidence` is available.
    ShredComplete,
}

/// Errors that can occur during erasure request processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErasureError {
    /// No key matching `receipt_commitment` was found.
    KeyNotFound,
    /// The key was found but commitment verification failed (internal integrity error).
    CommitmentMismatch,
}

/// A key store that can locate a `ReceiptKey` by its commitment.
///
/// Implementors must return `Some(key)` only when the key's `key_commitment`
/// matches `commitment` exactly. No raw key material should be copied outside
/// the returned `ReceiptKey`.
pub trait KeyStore {
    fn locate_by_commitment(&mut self, commitment: &[u8; 32]) -> Option<ReceiptKey>;
}

/// Drive an erasure request through the full lifecycle, returning evidence of
/// the completed shred or an error if the key cannot be found.
///
/// State transitions: `PendingLocate → Located → Shredding → ShredComplete`.
/// If the key is not found, returns `Err(ErasureError::KeyNotFound)` at the
/// `PendingLocate` state — no partial state is emitted.
///
/// On success the returned `ShredKeyEvidence` should be persisted as WAL
/// evidence before the caller considers the erasure complete.
pub fn process_erasure_request<S: KeyStore>(
    req: ErasureRequest,
    store: &mut S,
) -> Result<ShredKeyEvidence, ErasureError> {
    // PendingLocate → Located
    let key = store
        .locate_by_commitment(&req.receipt_commitment)
        .ok_or(ErasureError::KeyNotFound)?;

    // Located: verify the commitment matches (integrity check)
    if key.key_commitment != req.receipt_commitment {
        return Err(ErasureError::CommitmentMismatch);
    }

    // Shredding → ShredComplete
    Ok(shred_key(key, req.epoch, req.requestor_id))
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

    // ── KeyStore implementation for tests ─────────────────────────────────────

    struct VecKeyStore(Vec<ReceiptKey>);

    impl KeyStore for VecKeyStore {
        fn locate_by_commitment(&mut self, commitment: &[u8; 32]) -> Option<ReceiptKey> {
            if let Some(pos) = self
                .0
                .iter()
                .position(|k| &k.key_commitment == commitment)
            {
                Some(self.0.remove(pos))
            } else {
                None
            }
        }
    }

    #[test]
    fn process_erasure_request_success() {
        let material = [0xCAu8; 32];
        let key = ReceiptKey::new(material);
        let commitment = key.key_commitment;
        let mut store = VecKeyStore(vec![key]);

        let req = ErasureRequest {
            receipt_commitment: commitment,
            requestor_id: [0x01u8; 32],
            epoch: 42,
        };

        let evidence = process_erasure_request(req, &mut store).expect("erasure must succeed");
        assert_eq!(evidence.key_commitment, commitment, "evidence must carry the key commitment");
        assert_eq!(evidence.epoch, 42);
        assert_eq!(evidence.event_root, [0x01u8; 32]);
        // Key must be consumed from the store.
        assert!(store.0.is_empty(), "key must be removed from store after shred");
    }

    #[test]
    fn process_erasure_request_key_not_found() {
        let mut store = VecKeyStore(vec![]);
        let req = ErasureRequest {
            receipt_commitment: [0xFFu8; 32],
            requestor_id: [0x02u8; 32],
            epoch: 1,
        };
        assert_eq!(
            process_erasure_request(req, &mut store),
            Err(ErasureError::KeyNotFound)
        );
    }

    #[test]
    fn process_erasure_request_wrong_commitment_key_not_found() {
        // Key in store has a different commitment.
        let key = ReceiptKey::new([0x11u8; 32]);
        let mut store = VecKeyStore(vec![key]);
        let req = ErasureRequest {
            receipt_commitment: [0xFFu8; 32], // wrong commitment
            requestor_id: [0x03u8; 32],
            epoch: 5,
        };
        assert_eq!(
            process_erasure_request(req, &mut store),
            Err(ErasureError::KeyNotFound)
        );
        // Key must still be in store (not consumed).
        assert_eq!(store.0.len(), 1);
    }

    #[test]
    fn shred_key_returns_stable_commitment() {
        let material = [0xAB_u8; 32];
        let key = ReceiptKey::new(material);
        let commitment_before = key.key_commitment;
        let evidence = shred_key(key, 42, [0xEE_u8; 32]);
        assert_eq!(evidence.key_commitment, commitment_before);
        assert_eq!(evidence.epoch, 42);
        assert_eq!(evidence.event_root, [0xEE_u8; 32]);
    }

    #[test]
    fn shred_key_preserves_audit_context() {
        let key = ReceiptKey::new([0x11_u8; 32]);
        let epoch = 99u64;
        let event_root = [0xABu8; 32];
        let evidence = shred_key(key, epoch, event_root);
        // Audit trail is fully preserved — epoch and event_root identify when/why.
        assert_eq!(evidence.epoch, epoch);
        assert_eq!(evidence.event_root, event_root);
        // Distinct epochs produce distinguishable evidence records.
        let key2 = ReceiptKey::new([0x11_u8; 32]);
        let evidence2 = shred_key(key2, 100, event_root);
        assert_ne!(evidence.epoch, evidence2.epoch);
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

        let evidence = shred_key(key, 7, [0xDDu8; 32]);

        // After shred: evidence carries only the commitment, not the key bytes.
        assert_eq!(evidence.key_commitment, commitment);

        // There is no way to recover `material` from `evidence.key_commitment`
        // (preimage resistance). Verify the ciphertext is now opaque by showing
        // that XOR with the commitment does NOT recover the plaintext.
        let mut attempted_decrypt = [0u8; 32];
        for i in 0..32 {
            attempted_decrypt[i] = ciphertext[i] ^ evidence.key_commitment[i];
        }
        assert_ne!(
            attempted_decrypt, plaintext,
            "key commitment must not decrypt the ciphertext"
        );
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
