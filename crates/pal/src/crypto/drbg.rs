//! HMAC-DRBG (NIST SP 800-90A) deterministic random bit generator.
//!
//! Domain B only. Provides a FIPS-aligned entropy source intended for
//! FIPS 140-3 validation evidence for Domain B operations (key generation,
//! nonce derivation, etc.). "FIPS-aligned" is not "FIPS validated" —
//! FIPS 140-3 module validation requires CMVP lab engagement.
//!
//! The underlying `hmac-drbg` crate implements HMAC-SHA-256 DRBG per
//! NIST SP 800-90A Rev 1. This wrapper:
//! - enforces personalisation string binding to prevent cross-context reuse
//! - caps `generate` calls at the 2^48 reseed interval (conservative)
//! - exposes a `fill_bytes` interface compatible with Domain B key-gen code

use hmac_drbg::HmacDRBG;
use sha2::Sha256; // sha2 0.9 — compatible with hmac-drbg 0.3
use typenum::U32;

/// Reseed interval: we reseed after this many 32-byte generate calls.
/// NIST allows up to 2^48; we use 1 << 20 (~1M) for conservatism.
const RESEED_INTERVAL: u64 = 1 << 20;

pub struct FipsDrbg {
    inner: HmacDRBG<Sha256>,
    generate_count: u64,
    /// Entropy source callback: returns 32 bytes of OS entropy.
    entropy_fn: fn() -> [u8; 32],
}

impl FipsDrbg {
    /// Instantiate the DRBG with OS entropy and a personalisation string.
    ///
    /// `personalization` binds this instance to a specific use case
    /// (e.g. `b"qash/pal/kem_keygen/v1"`). Must be ≤ 32 bytes.
    pub fn new(personalization: &[u8], entropy_fn: fn() -> [u8; 32]) -> Self {
        assert!(
            personalization.len() <= 32,
            "personalization must be ≤ 32 bytes"
        );
        let entropy = entropy_fn();
        let nonce = entropy_fn(); // second call for nonce per SP 800-90A §8.6.7
        let inner = HmacDRBG::<Sha256>::new(&entropy, &nonce, personalization);
        Self {
            inner,
            generate_count: 0,
            entropy_fn,
        }
    }

    /// Fill `out` with deterministic pseudorandom bytes.
    ///
    /// Automatically reseeds when the reseed interval is reached.
    pub fn fill_bytes(&mut self, out: &mut [u8]) {
        // Chunk into 32-byte blocks (HmacDRBG generates 32 bytes per call).
        let mut pos = 0;
        while pos < out.len() {
            if self.generate_count >= RESEED_INTERVAL {
                self.reseed();
            }
            let block = self.inner.generate::<U32>(None);
            let remaining = out.len() - pos;
            let take = remaining.min(32);
            out[pos..pos + take].copy_from_slice(&block[..take]);
            pos += take;
            self.generate_count += 1;
        }
    }

    /// Generate a 32-byte key, reseeding if required.
    pub fn generate_key(&mut self) -> [u8; 32] {
        let mut out = [0u8; 32];
        self.fill_bytes(&mut out);
        out
    }

    /// Generate a 64-byte seed (for ML-KEM-768 key generation).
    pub fn generate_seed_64(&mut self) -> [u8; 64] {
        let mut out = [0u8; 64];
        self.fill_bytes(&mut out);
        out
    }

    fn reseed(&mut self) {
        let entropy = (self.entropy_fn)();
        self.inner.reseed(&entropy, None);
        self.generate_count = 0;
    }
}

/// OS entropy source using `getrandom`.
#[cfg(feature = "std")]
pub fn os_entropy() -> [u8; 32] {
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf).expect("OS entropy unavailable");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_entropy() -> [u8; 32] {
        [0x5A_u8; 32]
    }

    #[test]
    fn drbg_is_deterministic_same_seed() {
        let mut a = FipsDrbg::new(b"test/determinism", mock_entropy);
        let mut b = FipsDrbg::new(b"test/determinism", mock_entropy);
        let ka = a.generate_key();
        let kb = b.generate_key();
        assert_eq!(ka, kb, "same entropy + personalization → same output");
    }

    #[test]
    fn drbg_different_personalization_differs() {
        let mut a = FipsDrbg::new(b"ctx_a", mock_entropy);
        let mut b = FipsDrbg::new(b"ctx_b", mock_entropy);
        let ka = a.generate_key();
        let kb = b.generate_key();
        assert_ne!(ka, kb, "different personalization → different output");
    }

    #[test]
    fn drbg_outputs_are_nonzero() {
        let mut drbg = FipsDrbg::new(b"nonzero_test", mock_entropy);
        let k = drbg.generate_key();
        assert_ne!(k, [0u8; 32]);
    }

    #[test]
    fn drbg_sequential_outputs_differ() {
        let mut drbg = FipsDrbg::new(b"seq_test", mock_entropy);
        let k1 = drbg.generate_key();
        let k2 = drbg.generate_key();
        assert_ne!(k1, k2, "sequential DRBG outputs must differ");
    }

    #[test]
    fn generate_seed_64_is_nonzero() {
        let mut drbg = FipsDrbg::new(b"seed64_test", mock_entropy);
        let s = drbg.generate_seed_64();
        assert_ne!(s, [0u8; 64]);
    }

    #[test]
    fn fill_bytes_partial_block() {
        let mut drbg = FipsDrbg::new(b"partial_block", mock_entropy);
        let mut out = [0u8; 17]; // not a multiple of 32
        drbg.fill_bytes(&mut out);
        assert_ne!(out, [0u8; 17]);
    }
}
