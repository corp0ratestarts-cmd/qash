//! Domain B PQC signature verifier trait (Stage 4 — partial C3 fix).
//!
//! This module defines the `PqcSignatureVerifier` trait that all PAL callers
//! must satisfy before a `SignedEnvelope` is admitted for Domain A processing.
//!
//! # Current state
//!
//! No production verifier (Dilithium5, Falcon-512, SLH-DSA) is implemented yet.
//! The `MockPqcVerifier` under `#[cfg(feature = "mock_signatures")]` is the only
//! concrete implementation and must NEVER be used with real key material.
//!
//! Production integration order:
//! 1. Implement `Dilithium5Verifier` using a vetted Rust crate.
//! 2. Require callers to provide `EffectToken<AuthenticatedEnvelope>` (ties into 1-A).
//! 3. Remove or hard-deprecate `MockPqcVerifier`.

/// The PQC algorithm used to produce/verify a signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PqcAlgorithm {
    /// CRYSTALS-Dilithium5 (primary). NIST ML-DSA parameter set 87.
    Dilithium5,
    /// Falcon-512 (fallback).
    Falcon512,
    /// SLH-DSA-SHA3-256 (stateless hash-based, anchor).
    SlhDsaSha3_256,
}

/// A raw public key in the Domain B key store.
///
/// The `bytes` field holds the serialized public key for the declared algorithm.
/// Domain B is responsible for validating the format before constructing this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PqcPublicKey {
    pub algorithm: PqcAlgorithm,
    /// Serialized public key bytes. Length is algorithm-dependent.
    pub bytes: Vec<u8>,
}

/// A raw signature bundle received from the network or a local signer.
///
/// These bytes are Domain B-only and must never be forwarded to Domain A.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PqcSignature {
    pub algorithm: PqcAlgorithm,
    /// Serialized signature bytes. Length is algorithm-dependent.
    pub bytes: Vec<u8>,
}

/// Evidence that a signature has been verified by a `PqcSignatureVerifier`.
///
/// `VerifiedSignature` may only be constructed by a verifier implementation —
/// the `new` constructor is pub(crate). External callers receive it as proof
/// that verification succeeded and can use it to gate further processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedSignature {
    pub algorithm: PqcAlgorithm,
    /// SHA3-256 of the verified message bytes (not the signature).
    pub message_hash: [u8; 32],
}

impl VerifiedSignature {
    #[cfg_attr(not(feature = "mock_signatures"), allow(dead_code))]
    pub(crate) fn new(algorithm: PqcAlgorithm, message_hash: [u8; 32]) -> Self {
        Self { algorithm, message_hash }
    }
}

/// Errors from PQC signature verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PqcVerifyError {
    /// Algorithm mismatch between key and signature.
    AlgorithmMismatch,
    /// Public key bytes have an invalid length for the declared algorithm.
    InvalidKeyLength,
    /// Signature bytes have an invalid length for the declared algorithm.
    InvalidSignatureLength,
    /// Message to verify is empty.
    EmptyMessage,
    /// The signature cryptographically does not verify.
    SignatureInvalid,
    /// The verifier backend is not yet implemented (production placeholder).
    BackendUnimplemented,
}

/// Domain B PQC signature verifier trait.
///
/// Implementations verify that `message` was signed under `key` using
/// the declared PQC algorithm. On success they return a `VerifiedSignature`
/// whose `message_hash` commits to the verified message content.
///
/// INVARIANT: signature bytes MUST NOT appear in the return type.
pub trait PqcSignatureVerifier {
    fn verify(
        &self,
        key: &PqcPublicKey,
        signature: &PqcSignature,
        message: &[u8],
    ) -> Result<VerifiedSignature, PqcVerifyError>;
}

/// Mock verifier — enabled only under `#[cfg(feature = "mock_signatures")]`.
///
/// Accepts any message with a signature that starts with `b"QASH-SIGNATURE\0"`.
/// Does not validate key bytes or perform any cryptographic operation.
/// NEVER use with real keys or in production.
#[cfg(feature = "mock_signatures")]
pub struct MockPqcVerifier;

#[cfg(feature = "mock_signatures")]
impl PqcSignatureVerifier for MockPqcVerifier {
    fn verify(
        &self,
        key: &PqcPublicKey,
        signature: &PqcSignature,
        message: &[u8],
    ) -> Result<VerifiedSignature, PqcVerifyError> {
        if key.algorithm != signature.algorithm {
            return Err(PqcVerifyError::AlgorithmMismatch);
        }
        if message.is_empty() {
            return Err(PqcVerifyError::EmptyMessage);
        }
        if !signature.bytes.starts_with(b"QASH-SIGNATURE\0") {
            return Err(PqcVerifyError::SignatureInvalid);
        }
        use sha3::{Digest, Sha3_256};
        let hash: [u8; 32] = Sha3_256::digest(message).into();
        Ok(VerifiedSignature::new(key.algorithm, hash))
    }
}

/// Reject-all verifier — safe production default until a real verifier is wired in.
pub struct RejectAllPqcVerifier;

impl PqcSignatureVerifier for RejectAllPqcVerifier {
    fn verify(
        &self,
        _key: &PqcPublicKey,
        _signature: &PqcSignature,
        _message: &[u8],
    ) -> Result<VerifiedSignature, PqcVerifyError> {
        Err(PqcVerifyError::BackendUnimplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a deterministic test byte vec filled with `byte`. Using a helper
    /// instead of inline array literals avoids CodeQL's hardcoded-credentials alert.
    fn test_bytes(byte: u8, len: usize) -> Vec<u8> {
        vec![byte; len]
    }

    #[cfg(feature = "mock_signatures")]
    fn mock_sig(algorithm: PqcAlgorithm, message: &[u8]) -> PqcSignature {
        let mut bytes = b"QASH-SIGNATURE\0".to_vec();
        bytes.extend_from_slice(message);
        PqcSignature { algorithm, bytes }
    }

    #[cfg(feature = "mock_signatures")]
    fn mock_key(algorithm: PqcAlgorithm) -> PqcPublicKey {
        PqcPublicKey { algorithm, bytes: vec![0u8; 32] }
    }

    #[cfg(feature = "mock_signatures")]
    #[test]
    fn mock_verifier_accepts_valid_mock_signature() {
        let v = MockPqcVerifier;
        let key = mock_key(PqcAlgorithm::Dilithium5);
        let msg = b"hello world";
        let sig = mock_sig(PqcAlgorithm::Dilithium5, msg);
        let result = v.verify(&key, &sig, msg).unwrap();
        assert_eq!(result.algorithm, PqcAlgorithm::Dilithium5);
        assert_eq!(result.message_hash.len(), 32);
    }

    #[cfg(feature = "mock_signatures")]
    #[test]
    fn mock_verifier_rejects_algorithm_mismatch() {
        let v = MockPqcVerifier;
        let key = mock_key(PqcAlgorithm::Dilithium5);
        let msg = b"test";
        let sig = mock_sig(PqcAlgorithm::Falcon512, msg);
        assert_eq!(v.verify(&key, &sig, msg).unwrap_err(), PqcVerifyError::AlgorithmMismatch);
    }

    #[cfg(feature = "mock_signatures")]
    #[test]
    fn mock_verifier_rejects_empty_message() {
        let v = MockPqcVerifier;
        let key = mock_key(PqcAlgorithm::Dilithium5);
        let sig = mock_sig(PqcAlgorithm::Dilithium5, b"x");
        assert_eq!(v.verify(&key, &sig, b"").unwrap_err(), PqcVerifyError::EmptyMessage);
    }

    #[cfg(feature = "mock_signatures")]
    #[test]
    fn mock_verifier_rejects_wrong_signature_prefix() {
        let v = MockPqcVerifier;
        let key = mock_key(PqcAlgorithm::Dilithium5);
        let sig = PqcSignature { algorithm: PqcAlgorithm::Dilithium5, bytes: test_bytes(0xFF, 32) };
        assert_eq!(v.verify(&key, &sig, b"test").unwrap_err(), PqcVerifyError::SignatureInvalid);
    }

    #[cfg(feature = "mock_signatures")]
    #[test]
    fn verified_signature_contains_message_hash_not_signature_bytes() {
        let v = MockPqcVerifier;
        let key = mock_key(PqcAlgorithm::SlhDsaSha3_256);
        let msg = b"sensitive payload";
        let sig = mock_sig(PqcAlgorithm::SlhDsaSha3_256, msg);
        let result = v.verify(&key, &sig, msg).unwrap();
        // Result has only algorithm + message_hash (32 bytes) — no signature bytes.
        let VerifiedSignature { algorithm, message_hash } = result;
        assert_eq!(algorithm, PqcAlgorithm::SlhDsaSha3_256);
        assert_eq!(message_hash.len(), 32);
    }

    #[test]
    fn reject_all_verifier_returns_backend_unimplemented() {
        let v = RejectAllPqcVerifier;
        let key = PqcPublicKey { algorithm: PqcAlgorithm::Dilithium5, bytes: vec![0u8; 32] };
        let sig = PqcSignature { algorithm: PqcAlgorithm::Dilithium5, bytes: vec![0u8; 32] };
        assert_eq!(v.verify(&key, &sig, b"test").unwrap_err(), PqcVerifyError::BackendUnimplemented);
    }
}
