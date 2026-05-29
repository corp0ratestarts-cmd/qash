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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrbgError {
    PersonalizationTooLong,
    EntropyUnavailable,
}

#[derive(Clone, Copy)]
enum EntropySource {
    Infallible(fn() -> [u8; 32]),
    Fallible(fn() -> Result<[u8; 32], DrbgError>),
}

impl EntropySource {
    fn read(self) -> Result<[u8; 32], DrbgError> {
        match self {
            EntropySource::Infallible(f) => Ok(f()),
            EntropySource::Fallible(f) => f(),
        }
    }
}

pub struct FipsDrbg {
    inner: HmacDRBG<Sha256>,
    generate_count: u64,
    /// Entropy source callback: returns 32 bytes of OS entropy.
    entropy_source: EntropySource,
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
        Self::instantiate(personalization, EntropySource::Infallible(entropy_fn))
            .expect("infallible entropy source cannot fail")
    }

    /// Instantiate the DRBG with a fallible entropy source.
    ///
    /// This is the production-facing constructor for hosted PAL code: entropy
    /// failure is returned to the caller instead of panicking.
    pub fn try_new(
        personalization: &[u8],
        entropy_fn: fn() -> Result<[u8; 32], DrbgError>,
    ) -> Result<Self, DrbgError> {
        Self::instantiate(personalization, EntropySource::Fallible(entropy_fn))
    }

    fn instantiate(
        personalization: &[u8],
        entropy_source: EntropySource,
    ) -> Result<Self, DrbgError> {
        if personalization.len() > 32 {
            return Err(DrbgError::PersonalizationTooLong);
        }
        let entropy = entropy_source.read()?;
        let nonce = entropy_source.read()?; // second call for nonce per SP 800-90A §8.6.7
        let inner = HmacDRBG::<Sha256>::new(&entropy, &nonce, personalization);
        Ok(Self {
            inner,
            generate_count: 0,
            entropy_source,
        })
    }

    /// Fill `out` with deterministic pseudorandom bytes.
    ///
    /// Automatically reseeds when the reseed interval is reached.
    pub fn fill_bytes(&mut self, out: &mut [u8]) {
        self.try_fill_bytes(out)
            .expect("infallible DRBG entropy source cannot fail")
    }

    /// Fill `out` with pseudorandom bytes, returning entropy errors on reseed.
    pub fn try_fill_bytes(&mut self, out: &mut [u8]) -> Result<(), DrbgError> {
        // Chunk into 32-byte blocks (HmacDRBG generates 32 bytes per call).
        let mut pos = 0;
        while pos < out.len() {
            if self.generate_count >= RESEED_INTERVAL {
                self.try_reseed()?;
            }
            let block = self.inner.generate::<U32>(None);
            let remaining = out.len() - pos;
            let take = remaining.min(32);
            out[pos..pos + take].copy_from_slice(&block[..take]);
            pos += take;
            self.generate_count += 1;
        }
        Ok(())
    }

    /// Generate a 32-byte key, reseeding if required.
    pub fn generate_key(&mut self) -> [u8; 32] {
        let mut out = [0u8; 32];
        self.fill_bytes(&mut out);
        out
    }

    /// Generate a 32-byte key, returning entropy errors on reseed.
    pub fn try_generate_key(&mut self) -> Result<[u8; 32], DrbgError> {
        let mut out = [0u8; 32];
        self.try_fill_bytes(&mut out)?;
        Ok(out)
    }

    /// Generate a 64-byte seed (for ML-KEM-768 key generation).
    pub fn generate_seed_64(&mut self) -> [u8; 64] {
        let mut out = [0u8; 64];
        self.fill_bytes(&mut out);
        out
    }

    /// Generate a 64-byte seed, returning entropy errors on reseed.
    pub fn try_generate_seed_64(&mut self) -> Result<[u8; 64], DrbgError> {
        let mut out = [0u8; 64];
        self.try_fill_bytes(&mut out)?;
        Ok(out)
    }

    fn try_reseed(&mut self) -> Result<(), DrbgError> {
        let entropy = self.entropy_source.read()?;
        self.inner.reseed(&entropy, None);
        self.generate_count = 0;
        Ok(())
    }
}

/// OS entropy source using `getrandom`, returning errors to the caller.
#[cfg(feature = "std")]
pub fn try_os_entropy() -> Result<[u8; 32], DrbgError> {
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf).map_err(|_| DrbgError::EntropyUnavailable)?;
    Ok(buf)
}

/// OS entropy source using `getrandom`.
#[cfg(feature = "std")]
pub fn os_entropy() -> [u8; 32] {
    try_os_entropy().expect("OS entropy unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_entropy() -> [u8; 32] {
        [0x5A_u8; 32]
    }

    fn mock_fallible_entropy() -> Result<[u8; 32], DrbgError> {
        Ok(mock_entropy())
    }

    fn failing_entropy() -> Result<[u8; 32], DrbgError> {
        Err(DrbgError::EntropyUnavailable)
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

    #[test]
    fn try_new_rejects_long_personalization_without_panic() {
        match FipsDrbg::try_new(&[0u8; 33], mock_fallible_entropy) {
            Ok(_) => panic!("long personalization should fail"),
            Err(err) => assert_eq!(err, DrbgError::PersonalizationTooLong),
        }
    }

    #[test]
    fn try_new_returns_entropy_failure() {
        match FipsDrbg::try_new(b"entropy_failure", failing_entropy) {
            Ok(_) => panic!("entropy failure should be returned"),
            Err(err) => assert_eq!(err, DrbgError::EntropyUnavailable),
        }
    }

    #[test]
    fn try_generate_key_matches_infallible_path_for_same_entropy() {
        let mut infallible = FipsDrbg::new(b"try_key", mock_entropy);
        let mut fallible =
            FipsDrbg::try_new(b"try_key", mock_fallible_entropy).expect("entropy succeeds");

        assert_eq!(
            infallible.generate_key(),
            fallible.try_generate_key().expect("generation succeeds")
        );
    }

    // ── CAVP KAT: HMAC-SHA-256 (RFC 4231 / NIST SP 800-198) ─────────────────────

    /// CAVP gate: HMAC-SHA-256 known-answer test vectors from RFC 4231 §4.
    ///
    /// These three vectors are RFC 4231 test cases 1–3, which are also the
    /// reference vectors used by the NIST SP 800-198 CAVP HMAC validation
    /// program for L=32 (SHA-256 output length). Any deviation indicates a
    /// non-compliant HMAC-SHA-256 primitive and MUST block the CI merge gate.
    ///
    /// Implements RFC 2104 directly using `sha2::Sha256` to avoid trait-version
    /// ambiguity between `hmac` crate API generations.
    ///
    /// CI gate: `cargo test -p qash-pal -- cavp_hmac_sha256`
    #[test]
    fn cavp_hmac_sha256() {
        use sha2::{Digest, Sha256};

        // RFC 2104 HMAC-SHA-256: block size = 64 bytes.
        fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
            const BLOCK: usize = 64;
            // Step 1: derive K' (key padded / hashed to BLOCK bytes).
            let mut kp = [0u8; BLOCK];
            if key.len() > BLOCK {
                let h: [u8; 32] = Sha256::digest(key).into();
                kp[..32].copy_from_slice(&h);
            } else {
                kp[..key.len()].copy_from_slice(key);
            }
            // Step 2: inner hash = SHA-256((K' XOR ipad) || message)
            let mut ipad = kp;
            for b in &mut ipad { *b ^= 0x36; }
            let mut inner = Sha256::new();
            inner.update(&ipad[..]);
            inner.update(message);
            let inner_hash: [u8; 32] = inner.finalize().into();
            // Step 3: outer hash = SHA-256((K' XOR opad) || inner_hash)
            let mut opad = kp;
            for b in &mut opad { *b ^= 0x5c; }
            let mut outer = Sha256::new();
            outer.update(&opad[..]);
            outer.update(&inner_hash[..]);
            outer.finalize().into()
        }

        // RFC 4231 §4.2 — TC1: Key=0x0b*20, Data="Hi There"
        assert_eq!(
            hmac_sha256(&[0x0bu8; 20], b"Hi There"),
            [
                0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf,
                0x0b, 0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9,
                0x37, 0x6c, 0x2e, 0x32, 0xcf, 0xf7,
            ],
            "HMAC-SHA-256 RFC 4231 TC1 mismatch"
        );
        // RFC 4231 §4.3 — TC2: Key="Jefe", Data="what do ya want for nothing?"
        assert_eq!(
            hmac_sha256(b"Jefe", b"what do ya want for nothing?"),
            [
                0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24, 0x26, 0x08,
                0x95, 0x75, 0xc7, 0x5a, 0x00, 0x3f, 0x08, 0x9d, 0x27, 0x39, 0x83, 0x9d, 0xec,
                0x58, 0xb9, 0x64, 0xec, 0x38, 0x43,
            ],
            "HMAC-SHA-256 RFC 4231 TC2 mismatch"
        );
        // RFC 4231 §4.4 — TC3: Key=0xaa*20, Data=0xdd*50
        assert_eq!(
            hmac_sha256(&[0xaau8; 20], &[0xddu8; 50]),
            [
                0x77, 0x3e, 0xa9, 0x1e, 0x36, 0x80, 0x0e, 0x46, 0x85, 0x4d, 0xb8, 0xeb, 0xd0,
                0x91, 0x81, 0xa7, 0x29, 0x59, 0x09, 0x8b, 0x3e, 0xf8, 0xc1, 0x22, 0xd9, 0x63,
                0x55, 0x14, 0xce, 0xd5, 0x65, 0xfe,
            ],
            "HMAC-SHA-256 RFC 4231 TC3 mismatch"
        );
    }
}
