//! Power-On Self-Test (POST) framework for FIPS-aligned crypto module.
//!
//! Feature gate: `fips-post`
//!
//! Runs Known-Answer Tests (KATs) for all in-boundary Domain B cryptographic
//! algorithms at startup. If any KAT fails, `run_post()` returns
//! `Err(PostError::KatFailed)` and the crypto service should be considered
//! unavailable.
//!
//! **Module boundary**: SHA3-256 in Domain A (`qash-consensus`) is OUTSIDE
//! this FIPS boundary. The FIPS module covers Domain B crypto in `crates/pal/`:
//! SHA3-256 (as used for key derivation in Domain B), SHA-256, HMAC-DRBG,
//! and ML-KEM-768 (when `pqc` feature is enabled).
//!
//! This is internal self-test evidence. FIPS 140-3 module validation requires
//! engagement with a CMVP-accredited testing laboratory.

use sha2::Digest as Sha2Digest;
use sha3::Digest as Sha3Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostError {
    /// A known-answer test produced the wrong output.
    KatFailed(&'static str),
}

impl core::fmt::Display for PostError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PostError::KatFailed(alg) => write!(f, "POST KAT failed for {}", alg),
        }
    }
}

/// Run all POST self-tests. Returns `Ok(())` if all KATs pass.
///
/// Call once at crypto service initialization. If this returns `Err`,
/// the Domain B crypto module should refuse all operations.
pub fn run_post() -> Result<(), PostError> {
    post_sha3_256()?;
    post_sha256()?;
    post_hmac_drbg()?;
    #[cfg(feature = "pqc")]
    post_ml_kem()?;
    Ok(())
}

/// SHA3-256 KAT: NIST FIPS 202 empty-input vector.
fn post_sha3_256() -> Result<(), PostError> {
    // Source: NIST FIPS 202 / SHA3-256("") test vector
    let expected: [u8; 32] = [
        0xa7, 0xff, 0xc6, 0xf8, 0xbf, 0x1e, 0xd7, 0x66, 0x51, 0xc1, 0x47, 0x56, 0xa0, 0x61, 0xd6,
        0x62, 0xf5, 0x80, 0xff, 0x4d, 0xe4, 0x3b, 0x49, 0xfa, 0x82, 0xd8, 0x0a, 0x4b, 0x80, 0xf8,
        0x43, 0x4a,
    ];
    let got: [u8; 32] = sha3::Sha3_256::digest(b"").into();
    if got != expected {
        return Err(PostError::KatFailed("SHA3-256"));
    }
    Ok(())
}

/// SHA-256 KAT: NIST FIPS 180-4 empty-input vector.
fn post_sha256() -> Result<(), PostError> {
    // Source: NIST FIPS 180-4 SHA-256("") test vector
    let expected: [u8; 32] = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
        0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
        0xb8, 0x55,
    ];
    let got: [u8; 32] = sha2::Sha256::digest(b"").into();
    if got != expected {
        return Err(PostError::KatFailed("SHA-256"));
    }
    Ok(())
}

/// HMAC-DRBG KAT: pairwise consistency test.
///
/// Instantiate with fixed entropy, generate 32 bytes twice, verify the two
/// outputs are distinct (non-repeating generator property) and non-zero.
/// A full NIST SP 800-90A CAVP vector requires the full DRBG test harness;
/// this pairwise test catches catastrophic failures (constant output, zero
/// output, or self-replication) without requiring the full CAVP infrastructure.
fn post_hmac_drbg() -> Result<(), PostError> {
    use crate::crypto::drbg::FipsDrbg;
    fn fixed_entropy() -> [u8; 32] {
        [0x42u8; 32]
    }
    let mut drbg = FipsDrbg::new(b"post-kat", fixed_entropy);
    let mut out1 = [0u8; 32];
    let mut out2 = [0u8; 32];
    drbg.fill_bytes(&mut out1);
    drbg.fill_bytes(&mut out2);
    if out1 == [0u8; 32] || out2 == [0u8; 32] {
        return Err(PostError::KatFailed("HMAC-DRBG"));
    }
    if out1 == out2 {
        return Err(PostError::KatFailed("HMAC-DRBG"));
    }
    Ok(())
}

/// ML-KEM-768 KAT: encapsulate/decapsulate pairwise consistency test.
///
/// Generate a keypair from a fixed seed, encapsulate with fixed randomness,
/// decapsulate, and verify both sides recover the same shared secret.
#[cfg(feature = "pqc")]
fn post_ml_kem() -> Result<(), PostError> {
    use crate::crypto::kem::{encapsulate, MlKem768KeyPair};
    let seed = [0x5Au8; 64];
    let randomness = [0x3Cu8; 32];
    let keypair = MlKem768KeyPair::from_seed(&seed);
    let ek = keypair.encap_key();
    let (ct, ss_enc) = encapsulate(&ek, &randomness);
    let ss_dec = keypair.decapsulate(&ct);
    if ss_enc != ss_dec {
        return Err(PostError::KatFailed("ML-KEM-768"));
    }
    if ss_enc == [0u8; 32] {
        return Err(PostError::KatFailed("ML-KEM-768"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_all_pass() {
        run_post().expect("all POST KATs must pass");
    }

    #[test]
    fn post_sha3_256_known_answer() {
        post_sha3_256().expect("SHA3-256 KAT must pass");
    }

    #[test]
    fn post_sha256_known_answer() {
        post_sha256().expect("SHA-256 KAT must pass");
    }

    #[test]
    fn post_hmac_drbg_consistency() {
        post_hmac_drbg().expect("HMAC-DRBG pairwise consistency test must pass");
    }
}
