//! Regulated receipt disclosure — Class IV epoch-scoped decryption gate.
//!
//! Implements the receipt disclosure path from §P7 of `09_privacy_model.md`:
//! receipt bodies are encrypted by default; selective disclosure requires an
//! explicit disclosure key scoped to a genesis-authorised disclosure domain.
//!
//! # Access model (normative)
//!
//! 1. Caller presents a `DisclosureRequest` with a valid `LawfulBasis`.
//! 2. Caller presents a `DisclosureKey` covering the requested epoch.
//! 3. `RegulatedReceiptDecrypt::decrypt()` validates both, derives the epoch
//!    key, and decrypts the receipt ciphertext using ChaCha20-Poly1305 AEAD.
//! 4. On any validation failure, all key material is zeroized before returning.
//!
//! # Forward secrecy (normative)
//!
//! Disclosure is non-retroactive. A key with activation_epoch = T cannot
//! decrypt receipts from epochs < T. After epoch_seed destruction (max_offline_epochs),
//! the epoch-scoped decryption key is also gone — even the regulatory authority
//! cannot decrypt past-epoch receipts after the key rotation window.

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305,
};
use sha3::{Digest, Sha3_256};
use zeroize::Zeroize;

use super::disclosure::{
    validate_disclosure_request, DisclosureDomain, DisclosureKey, DisclosureRequest,
    DisclosureRequestError,
};

/// Errors from regulated receipt decryption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegulatedDecryptError {
    /// Disclosure request validation failed.
    RequestInvalid(DisclosureRequestError),
    /// AEAD authentication tag failed — ciphertext corrupt or wrong key.
    AuthenticationFailed,
    /// Nonce supplied is not 12 bytes.
    InvalidNonce,
    /// Decrypted plaintext is empty (likely wrong key or corrupt ciphertext).
    EmptyPlaintext,
    /// `receipt.domain_tag` does not match `domain.domain_id`.
    DomainTagMismatch,
    /// Decrypted plaintext hash does not match the stored `plaintext_commitment`.
    PlaintextCommitmentMismatch,
    /// OS RNG failure — `getrandom` could not produce random bytes.
    RngFailure,
}

/// Encrypted receipt body for Class IV disclosure.
///
/// The `ciphertext` is a ChaCha20-Poly1305 AEAD ciphertext (plaintext + 16-byte tag).
/// The `nonce` is a 12-byte random value generated fresh for each encryption.
/// The `domain_tag` identifies which disclosure domain encrypted this receipt.
#[derive(Debug, Clone)]
pub struct EncryptedRegulatedReceipt {
    pub epoch: u64,
    pub nonce: [u8; 12],
    pub domain_tag: [u8; 32],
    pub ciphertext: Vec<u8>,
    /// Commitment to the plaintext receipt (for integrity verification after decryption).
    pub plaintext_commitment: [u8; 32],
}

/// Class IV regulated receipt decryption gate.
///
/// Decrypts receipts only after validating the lawful-basis disclosure request
/// and confirming the disclosure key covers the requested epoch.
pub struct RegulatedReceiptDecrypt;

impl RegulatedReceiptDecrypt {
    /// Decrypt a regulated receipt after validating the disclosure request.
    ///
    /// Returns the plaintext receipt bytes on success. Key material is zeroized
    /// internally on all exit paths.
    ///
    /// # Errors
    ///
    /// - `DomainTagMismatch` — receipt.domain_tag ≠ domain.domain_id.
    /// - `RequestInvalid` — lawful basis or epoch scope validation failed.
    /// - `AuthenticationFailed` — AEAD tag mismatch (corrupt ciphertext or wrong key).
    /// - `EmptyPlaintext` — decrypted to empty bytes.
    /// - `PlaintextCommitmentMismatch` — decrypted content does not match stored commitment.
    pub fn decrypt(
        receipt: &EncryptedRegulatedReceipt,
        request: &DisclosureRequest,
        domain: &DisclosureDomain,
        key: &DisclosureKey,
    ) -> Result<Vec<u8>, RegulatedDecryptError> {
        // Pre-check: domain tag must match the presented domain.
        if receipt.domain_tag != domain.domain_id {
            return Err(RegulatedDecryptError::DomainTagMismatch);
        }

        // Gate 1: validate the disclosure request.
        validate_disclosure_request(request, domain, key)
            .map_err(RegulatedDecryptError::RequestInvalid)?;

        // Gate 2: derive the epoch-scoped decryption key.
        let mut epoch_key = key
            .derive_epoch_key(receipt.epoch)
            .map_err(RegulatedDecryptError::RequestInvalid)?;

        // Gate 3: decrypt with ChaCha20-Poly1305.
        let cipher = ChaCha20Poly1305::new_from_slice(&epoch_key.key)
            .expect("32-byte key is always valid for ChaCha20Poly1305");

        // AAD = domain_tag || epoch_be8 — binds plaintext to this domain + epoch.
        let mut aad = [0u8; 40];
        aad[..32].copy_from_slice(&receipt.domain_tag);
        aad[32..40].copy_from_slice(&receipt.epoch.to_be_bytes());

        let payload = Payload { msg: &receipt.ciphertext, aad: &aad };

        let plaintext = cipher
            .decrypt(&receipt.nonce.into(), payload)
            .map_err(|_| {
                epoch_key.key.zeroize();
                RegulatedDecryptError::AuthenticationFailed
            })?;

        epoch_key.key.zeroize();

        if plaintext.is_empty() {
            return Err(RegulatedDecryptError::EmptyPlaintext);
        }

        // Gate 4: verify plaintext commitment (belt-and-suspenders over AEAD integrity).
        let mut hc = Sha3_256::new();
        hc.update(&plaintext);
        let computed: [u8; 32] = hc.finalize().into();
        if computed != receipt.plaintext_commitment {
            return Err(RegulatedDecryptError::PlaintextCommitmentMismatch);
        }

        Ok(plaintext)
    }

    /// Encrypt a receipt body for Class IV disclosure.
    ///
    /// Used at receipt creation time to prepare a receipt for potential future
    /// Class IV disclosure within the declared epoch and domain scope.
    pub fn encrypt(
        plaintext: &[u8],
        epoch: u64,
        domain: &DisclosureDomain,
        key: &DisclosureKey,
    ) -> Result<EncryptedRegulatedReceipt, RegulatedDecryptError> {
        let mut epoch_key = key
            .derive_epoch_key(epoch)
            .map_err(RegulatedDecryptError::RequestInvalid)?;

        let cipher = ChaCha20Poly1305::new_from_slice(&epoch_key.key)
            .expect("32-byte key is always valid for ChaCha20Poly1305");

        // Generate a fresh random nonce for each encryption.
        // A deterministic nonce would allow correlation attacks on identical plaintexts
        // at the same epoch (CWE-323). The nonce is stored in the ciphertext envelope
        // and passed to decrypt(), so randomness here causes no key-management issue.
        let mut nonce = [0u8; 12];
        getrandom::fill(&mut nonce).map_err(|_| {
            epoch_key.key.zeroize();
            RegulatedDecryptError::RngFailure
        })?;

        let mut aad = [0u8; 40];
        aad[..32].copy_from_slice(&domain.domain_id);
        aad[32..40].copy_from_slice(&epoch.to_be_bytes());

        let payload = Payload { msg: plaintext, aad: &aad };
        let ciphertext = cipher
            .encrypt(&nonce.into(), payload)
            .expect("encrypt cannot fail for valid key/nonce");

        // Compute plaintext commitment for integrity verification on decrypt.
        let mut hc = Sha3_256::new();
        hc.update(plaintext);
        let plaintext_commitment: [u8; 32] = hc.finalize().into();

        epoch_key.key.zeroize();

        Ok(EncryptedRegulatedReceipt {
            epoch,
            nonce,
            domain_tag: domain.domain_id,
            ciphertext,
            plaintext_commitment,
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{LawfulBasis, disclosure::DisclosureRequest};
    use super::super::disclosure::{DisclosureDomain, DisclosureKey};

    fn test_key() -> DisclosureKey {
        DisclosureKey::from_genesis_material(
            [0x42u8; 32], 100, 200,
            *b"QASH-TEST-DOMAIN-EU-AML-0000000\x00",
        )
    }

    fn test_domain() -> DisclosureDomain {
        DisclosureDomain {
            domain_id: *b"QASH-TEST-DOMAIN-EU-AML-0000000\x00",
            jurisdiction: *b"DE",
            activation_epoch: 100,
            expiry_epoch: 200,
            key_commitment: test_key().compute_commitment(),
        }
    }

    fn test_request(epoch_start: u64, epoch_end: u64) -> DisclosureRequest {
        DisclosureRequest {
            lawful_basis: LawfulBasis::GdprArt6LegalObligation,
            requester_id: [1u8; 32],
            epoch_start,
            epoch_end,
            case_reference: [2u8; 32],
        }
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let domain = test_domain();
        let key = test_key();
        let plaintext = b"regulated receipt body test data";

        let encrypted = RegulatedReceiptDecrypt::encrypt(plaintext, 150, &domain, &key).unwrap();
        let request = test_request(100, 199);
        let decrypted = RegulatedReceiptDecrypt::decrypt(&encrypted, &request, &domain, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_rejects_wrong_epoch() {
        let domain = test_domain();
        let key = test_key();
        let plaintext = b"receipt data";

        let encrypted = RegulatedReceiptDecrypt::encrypt(plaintext, 150, &domain, &key).unwrap();
        let bad_request = test_request(50, 99);

        let err = RegulatedReceiptDecrypt::decrypt(&encrypted, &bad_request, &domain, &key)
            .unwrap_err();
        assert!(matches!(err, RegulatedDecryptError::RequestInvalid(_)));
    }

    #[test]
    fn decrypt_rejects_tampered_ciphertext() {
        let domain = test_domain();
        let key = test_key();
        let plaintext = b"tamper test receipt";

        let mut encrypted = RegulatedReceiptDecrypt::encrypt(plaintext, 150, &domain, &key).unwrap();
        encrypted.ciphertext[0] ^= 0xFF;

        let request = test_request(100, 199);
        let err = RegulatedReceiptDecrypt::decrypt(&encrypted, &request, &domain, &key).unwrap_err();
        assert_eq!(err, RegulatedDecryptError::AuthenticationFailed);
    }

    #[test]
    fn different_epochs_produce_different_ciphertexts() {
        let domain = test_domain();
        let key = test_key();
        let plaintext = b"same plaintext";

        let e1 = RegulatedReceiptDecrypt::encrypt(plaintext, 100, &domain, &key).unwrap();
        let e2 = RegulatedReceiptDecrypt::encrypt(plaintext, 101, &domain, &key).unwrap();
        assert_ne!(e1.ciphertext, e2.ciphertext, "different epochs must produce different ciphertexts");
    }

    #[test]
    fn decrypt_rejects_blank_requester() {
        let domain = test_domain();
        let key = test_key();
        let plaintext = b"some receipt";

        let encrypted = RegulatedReceiptDecrypt::encrypt(plaintext, 150, &domain, &key).unwrap();
        let bad_request = DisclosureRequest {
            lawful_basis: LawfulBasis::GdprArt6LegalObligation,
            requester_id: [0u8; 32],
            epoch_start: 100,
            epoch_end: 199,
            case_reference: [2u8; 32],
        };
        let err = RegulatedReceiptDecrypt::decrypt(&encrypted, &bad_request, &domain, &key).unwrap_err();
        assert!(matches!(err, RegulatedDecryptError::RequestInvalid(
            DisclosureRequestError::RequesterIdentityBlank
        )));
    }

    // ── Hardening tests ──────────────────────────────────────────────────────

    #[test]
    fn decrypt_rejects_wrong_domain_tag() {
        let domain = test_domain();
        let key = test_key();
        let plaintext = b"test receipt";

        let mut encrypted = RegulatedReceiptDecrypt::encrypt(plaintext, 150, &domain, &key).unwrap();
        encrypted.domain_tag = [0xFFu8; 32];

        let request = test_request(100, 199);
        let err = RegulatedReceiptDecrypt::decrypt(&encrypted, &request, &domain, &key).unwrap_err();
        assert_eq!(err, RegulatedDecryptError::DomainTagMismatch);
    }

    #[test]
    fn decrypt_rejects_tampered_plaintext_commitment() {
        let domain = test_domain();
        let key = test_key();
        let plaintext = b"test receipt";

        let mut encrypted = RegulatedReceiptDecrypt::encrypt(plaintext, 150, &domain, &key).unwrap();
        encrypted.plaintext_commitment[0] ^= 0xFF;

        let request = test_request(100, 199);
        let err = RegulatedReceiptDecrypt::decrypt(&encrypted, &request, &domain, &key).unwrap_err();
        assert_eq!(err, RegulatedDecryptError::PlaintextCommitmentMismatch);
    }
}
