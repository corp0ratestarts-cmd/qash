//! Crypto-agility traits for Domain B cryptographic operations.
//!
//! These traits decouple protocol logic from concrete algorithm choices,
//! allowing suite substitution (Standard, Guomi, Korea) without touching
//! Domain A. The underlying algorithms are FIPS-aligned; validation evidence
//! is documented in `docs/compliance/fips_compliance.md`.
//!
//! Feature guards:
//! - `SuiteStandard`: always available (no feature flag needed)
//! - `SuiteGuomi`: requires `feature = "suite_guomi"` (SM3/SM4)
//! - `SuiteKorea`: requires `feature = "suite_korea"` (LSH-512)
//!
//! Domain A parity check: toggling these features must not change
//! `crates/consensus/` — `git diff crates/consensus/` must remain empty.

/// Trait for domain-tagged hash functions used in Domain B.
///
/// Domain B hash outputs must never flow into Domain A state fields without
/// passing through the Domain A admission boundary.
pub trait HasherTrait {
    /// Hash `input` and return a 32-byte digest.
    fn hash(input: &[u8]) -> [u8; 32];

    /// Hash `input` with a domain tag prefix byte.
    fn hash_tagged(domain_tag: u8, input: &[u8]) -> [u8; 32] {
        let mut buf = Vec::with_capacity(1 + input.len());
        buf.push(domain_tag);
        buf.extend_from_slice(input);
        Self::hash(&buf)
    }
}

/// Trait for key encapsulation mechanisms (KEM).
pub trait KemTrait {
    type PublicKey: AsRef<[u8]>;
    type SecretKey;
    type Ciphertext: AsRef<[u8]>;

    fn encapsulate(
        pk: &Self::PublicKey,
        randomness: &[u8; 32],
    ) -> (Self::Ciphertext, [u8; 32]);

    fn decapsulate(sk: &Self::SecretKey, ct: &Self::Ciphertext) -> [u8; 32];
}

/// Trait for authenticated encryption with associated data (AEAD cipher).
///
/// Requires `std` (heap-allocated ciphertext output).
#[cfg(feature = "std")]
pub trait CipherTrait {
    type Error;

    fn encrypt(
        key: &[u8; 32],
        nonce: &[u8; 12],
        plaintext: &[u8],
        aad: &[u8],
    ) -> Vec<u8>;

    fn decrypt(
        key: &[u8; 32],
        nonce: &[u8; 12],
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, Self::Error>;
}

/// Trait for digital signatures.
///
/// Requires `std` (heap-allocated key/signature types).
#[cfg(feature = "std")]
pub trait SignatureTrait {
    type PublicKey;
    type SecretKey;
    type Signature;
    type Error;

    fn sign(sk: &Self::SecretKey, message: &[u8]) -> Self::Signature;
    fn verify(pk: &Self::PublicKey, message: &[u8], sig: &Self::Signature)
        -> Result<(), Self::Error>;
}

// ─── Suite definitions ────────────────────────────────────────────────────

/// Standard QASH Domain B crypto suite.
///
/// Algorithms: SHA3-256 (hash), ML-KEM-768 (KEM), Dilithium5 (signature).
/// All FIPS-aligned; see `docs/compliance/fips_compliance.md`.
pub struct SuiteStandard;

impl HasherTrait for SuiteStandard {
    fn hash(input: &[u8]) -> [u8; 32] {
        use sha3::{Digest, Sha3_256};
        let mut h = Sha3_256::new();
        h.update(input);
        h.finalize().into()
    }
}

/// Guomi (SM) crypto suite — Chinese national standards.
///
/// Feature-gated: `feature = "suite_guomi"`. Uses SM3 hash, SM4 cipher.
/// Non-FIPS; enabled only when sovereign policy requires GM/T standards.
/// Gating this feature off must produce no change in `crates/consensus/`.
#[cfg(feature = "suite_guomi")]
pub struct SuiteGuomi;

#[cfg(feature = "suite_guomi")]
impl HasherTrait for SuiteGuomi {
    fn hash(input: &[u8]) -> [u8; 32] {
        // SM3 (GB/T 32905-2016) — 256-bit output.
        // Implemented via `sm3` crate when `suite_guomi` feature is active.
        use sm3::{Digest, Sm3};
        let mut h = Sm3::new();
        h.update(input);
        h.finalize().into()
    }
}

/// Korean national crypto suite (KS X 3262).
///
/// Feature-gated: `feature = "suite_korea"`. Uses LSH-512 hash.
/// The `lsh-rs` crate on crates.io is a similarity-search library — do NOT
/// use it. The LSH-512 implementation lives in `crates/consensus/src/lsh512.rs`
/// (Domain A). Domain B re-exports it through this suite wrapper.
/// Gating this feature off must produce no change in `crates/consensus/`.
#[cfg(feature = "suite_korea")]
pub struct SuiteKorea;

#[cfg(feature = "suite_korea")]
impl HasherTrait for SuiteKorea {
    fn hash(input: &[u8]) -> [u8; 32] {
        // LSH-512/256 (KS X 3262): 512-bit hash truncated to first 256 bits.
        // Domain A implementation exported from qash_consensus::lsh512.
        let full = qash_consensus::lsh512(input);
        let mut out = [0u8; 32];
        out.copy_from_slice(&full[..32]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_suite_hash_is_deterministic() {
        let a = SuiteStandard::hash(b"qash-test");
        let b = SuiteStandard::hash(b"qash-test");
        assert_eq!(a, b);
        assert_ne!(a, [0u8; 32]);
    }

    #[test]
    fn standard_suite_tagged_hash_differs_from_untagged() {
        let untagged = SuiteStandard::hash(b"data");
        let tagged = SuiteStandard::hash_tagged(0xAB, b"data");
        assert_ne!(untagged, tagged);
    }

    #[test]
    fn standard_suite_hash_binds_all_input() {
        let a = SuiteStandard::hash(b"input_a");
        let b = SuiteStandard::hash(b"input_b");
        assert_ne!(a, b);
    }

    #[cfg(feature = "suite_guomi")]
    #[test]
    fn guomi_suite_hash_is_deterministic() {
        let a = SuiteGuomi::hash(b"qash-test");
        let b = SuiteGuomi::hash(b"qash-test");
        assert_eq!(a, b);
        assert_ne!(a, [0u8; 32]);
    }

    #[cfg(feature = "suite_guomi")]
    #[test]
    fn guomi_hash_differs_from_standard() {
        let standard = SuiteStandard::hash(b"same-input");
        let guomi = SuiteGuomi::hash(b"same-input");
        assert_ne!(standard, guomi);
    }
}
