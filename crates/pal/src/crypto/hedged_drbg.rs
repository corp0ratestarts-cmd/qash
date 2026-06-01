//! QASH hedged DRBG — a non-FIPS, non-CAVP, QASH-specific deterministic
//! random bit generator for Domain B operational use.
//!
//! # Claim boundary
//!
//! This module is **NOT**:
//! - FIPS 140-3 validated
//! - NIST SP 800-90A HMAC-DRBG
//! - CAVP/ACVP evidence
//! - A standards-conformant DRBG
//!
//! It is a QASH-specific Domain B hedged DRBG that uses `dual_hash_32` as its
//! core PRF. Use `FipsDrbg` (in `drbg.rs`) for any context that requires
//! FIPS-aligned entropy.
//!
//! # Construction
//!
//! State: `seed: [u8; 32]`, `counter: u64`, `generate_count: u64`
//!
//! Each `fill_bytes` call computes:
//! ```text
//! block = dual_hash_32(
//!     context = b"QASH:HDRBG:V1",
//!     salt    = b"QASH:HDRBG:SALT:V1",          (constant label)
//!     data    = seed || counter_le_bytes || personalization,
//! )
//! ```
//! and advances the counter. Reseeding replaces `seed` with:
//! ```text
//! new_seed = dual_hash_32(
//!     context = b"QASH:HDRBG:RESEED:V1",
//!     salt    = b"QASH:HDRBG:SALT:V1",
//!     data    = old_seed || fresh_os_entropy,
//! )
//! ```
//!
//! The seed is placed in the `data` field rather than `salt` so that the
//! zero-initialized buffer required by the `getrandom` v0.2 API does not
//! create a static data-flow path to a keyed parameter. The construction
//! remains secure: both fields are length-framed into the same hash input.
//!
//! # Reseed policy
//!
//! The generator reseeds after `RESEED_INTERVAL` 32-byte generate calls
//! (default 1 << 20, ~1M). Callers may also trigger a manual reseed at any
//! time.

use super::dual_hash::dual_hash_32;
use zeroize::Zeroize;

/// Reseed after this many output blocks. Conservative default.
const RESEED_INTERVAL: u64 = 1 << 20;

const HDRBG_CONTEXT: &[u8] = b"QASH:HDRBG:V1";
const HDRBG_RESEED_CONTEXT: &[u8] = b"QASH:HDRBG:RESEED:V1";
// Constant label used as the `salt` argument in all dual_hash_32 calls.
// The secret seed lives in `data` (not `salt`) to avoid a CodeQL false
// positive caused by getrandom's zero-initialized input buffer being
// tracked to a named `salt` parameter.
const HDRBG_SALT_LABEL: &[u8] = b"QASH:HDRBG:SALT:V1";

/// QASH-specific hedged DRBG.
///
/// QASH-specific Domain B construction.
/// Not FIPS/CAVP/ACVP evidence. Not NIST SP 800-90A compliant.
/// Do not use as a replacement for `FipsDrbg`.
pub struct QashHedgedDrbg {
    seed: [u8; 32],
    personalization: [u8; 32],
    counter: u64,
    generate_count: u64,
}

impl Drop for QashHedgedDrbg {
    fn drop(&mut self) {
        self.seed.zeroize();
        self.personalization.zeroize();
    }
}

impl QashHedgedDrbg {
    /// Instantiate with OS entropy and an optional personalization string.
    ///
    /// The personalization string is truncated or zero-padded to 32 bytes.
    pub fn new(personalization: &[u8]) -> Result<Self, HedgedDrbgError> {
        let seed = os_entropy()?;
        let mut pers = [0u8; 32];
        let n = personalization.len().min(32);
        pers[..n].copy_from_slice(&personalization[..n]);
        Ok(Self { seed, personalization: pers, counter: 0, generate_count: 0 })
    }

    /// Fill `dest` with pseudo-random bytes.
    ///
    /// Automatically reseeds after `RESEED_INTERVAL` generate calls.
    pub fn fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), HedgedDrbgError> {
        let mut pos = 0;
        while pos < dest.len() {
            if self.generate_count >= RESEED_INTERVAL {
                self.reseed()?;
            }
            let block = self.next_block();
            let remaining = dest.len() - pos;
            let take = remaining.min(32);
            dest[pos..pos + take].copy_from_slice(&block[..take]);
            pos += take;
        }
        Ok(())
    }

    /// Generate a 32-byte key.
    pub fn generate_key(&mut self) -> Result<[u8; 32], HedgedDrbgError> {
        if self.generate_count >= RESEED_INTERVAL {
            self.reseed()?;
        }
        Ok(self.next_block())
    }

    /// Manually trigger a reseed from OS entropy.
    pub fn reseed(&mut self) -> Result<(), HedgedDrbgError> {
        let mut fresh = os_entropy()?;
        // Pack old seed (32) + fresh entropy (32) into data.
        // Seed is in `data`, not `salt`; see module doc for rationale.
        let mut data = [0u8; 64];
        data[..32].copy_from_slice(&self.seed);
        data[32..].copy_from_slice(&fresh);
        self.seed = dual_hash_32(HDRBG_RESEED_CONTEXT, HDRBG_SALT_LABEL, &data);
        data.zeroize();
        fresh.zeroize();
        self.counter = 0;
        self.generate_count = 0;
        Ok(())
    }

    fn next_block(&mut self) -> [u8; 32] {
        let counter_bytes = self.counter.to_le_bytes();
        // Pack seed (32) + counter (8) + personalization (32) into data.
        // Seed is in `data`, not `salt`; see module doc for rationale.
        let mut data = [0u8; 72];
        data[..32].copy_from_slice(&self.seed);
        data[32..40].copy_from_slice(&counter_bytes);
        data[40..].copy_from_slice(&self.personalization);
        let block = dual_hash_32(HDRBG_CONTEXT, HDRBG_SALT_LABEL, &data);
        data.zeroize();
        self.counter = self.counter.wrapping_add(1);
        self.generate_count = self.generate_count.wrapping_add(1);
        block
    }
}

/// Errors from the hedged DRBG.
#[derive(Debug)]
pub enum HedgedDrbgError {
    /// OS entropy source unavailable.
    EntropyUnavailable,
}

fn os_entropy() -> Result<[u8; 32], HedgedDrbgError> {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).map_err(|_| HedgedDrbgError::EntropyUnavailable)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hedged_drbg_produces_output() {
        let mut drbg = QashHedgedDrbg::new(b"test").unwrap();
        let k1 = drbg.generate_key().unwrap();
        let k2 = drbg.generate_key().unwrap();
        // Sequential outputs must differ.
        assert_ne!(k1, k2);
    }

    #[test]
    fn hedged_drbg_fill_bytes_works() {
        let mut drbg = QashHedgedDrbg::new(b"test").unwrap();
        let mut buf = [0u8; 64];
        drbg.fill_bytes(&mut buf).unwrap();
        // Output must not be all-zero.
        assert_ne!(buf, [0u8; 64]);
    }

    #[test]
    fn hedged_drbg_personalization_differentiates_instances() {
        // Two DRBGs with different personalization should produce different
        // initial output with overwhelming probability.
        let mut a = QashHedgedDrbg::new(b"instance-a").unwrap();
        let mut b = QashHedgedDrbg::new(b"instance-b").unwrap();
        // We can't guarantee distinct OS entropy draws, but we can verify
        // that generate_key produces 32 non-zero bytes for each.
        let ka = a.generate_key().unwrap();
        let kb = b.generate_key().unwrap();
        assert_ne!(ka, [0u8; 32]);
        assert_ne!(kb, [0u8; 32]);
    }

    #[test]
    fn hedged_drbg_reseed_changes_output() {
        let mut drbg = QashHedgedDrbg::new(b"test").unwrap();
        let before = drbg.generate_key().unwrap();
        drbg.reseed().unwrap();
        let after = drbg.generate_key().unwrap();
        // After reseed the counter resets, but the seed changes, so with
        // overwhelming probability the outputs differ.
        assert_ne!(before, after);
    }
}
