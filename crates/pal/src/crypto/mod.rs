//! Domain B cryptographic primitives.
//!
//! All code here is Domain B: it may use `std`, heap allocation, and `unsafe`
//! (under audit). It MUST NOT be called from Domain A (`qash-consensus`).
//!
//! Modules:
//! - `dual_hash`   — QASH-specific hedged dual-hash utility (SHA3-512 + BLAKE3 XOR combiner)
//! - `hedged_drbg` — QASH-specific non-FIPS hedged DRBG (uses dual_hash_32 as PRF)
//! - `kem`         — ML-KEM-768 post-quantum key encapsulation (feature = "pqc")
//! - `drbg`        — HMAC-DRBG (NIST SP 800-90A) deterministic random bit generator
//! - `traits`      — Crypto-agility traits and suite definitions (Standard, Guomi, Korea)
//! - `tls`         — TLS config validation and log pseudonym helpers

pub mod dual_hash;
#[cfg(feature = "std")]
pub mod hedged_drbg;

#[cfg(feature = "pqc")]
pub mod kem;

pub mod agility;
pub mod drbg;
#[cfg(feature = "fips-post")]
pub mod post;
pub mod tls;
pub mod traits;
