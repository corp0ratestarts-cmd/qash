//! Domain B cryptographic primitives.
//!
//! All code here is Domain B: it may use `std`, heap allocation, and `unsafe`
//! (under audit). It MUST NOT be called from Domain A (`qash-consensus`).
//!
//! Modules:
//! - `kem`    — ML-KEM-768 post-quantum key encapsulation (feature = "pqc")
//! - `drbg`   — HMAC-DRBG (NIST SP 800-90A) deterministic random bit generator
//! - `traits` — Crypto-agility traits and suite definitions (Standard, Guomi, Korea)
//! - `tls`    — TLS config validation and log pseudonym helpers

#[cfg(feature = "pqc")]
pub mod kem;

pub mod drbg;
pub mod tls;
pub mod traits;
