//! Crypto conformance scaffolding for Domain B (PAL).
//!
//! These tests intentionally begin as skeletons so CI wiring and vector
//! provenance can land before full algorithmic harnesses are implemented.

#![cfg(feature = "std")]

#[test]
#[ignore = "skeleton: wire ML-KEM KAT parser and known-answer checks"]
fn ml_kem_kats() {
    // TODO: Load tests/vectors/crypto/ml_kem_kat.json and validate against
    // the selected ML-KEM implementation.
}

#[test]
#[ignore = "skeleton: wire X-Wing combiner vectors"]
fn x_wing_combiner_vectors() {
    // TODO: Validate X-Wing combiner outputs against source-tagged vectors.
}

#[test]
#[ignore = "skeleton: wire HMAC-DRBG KAT vectors"]
fn hmac_drbg_known_answer() {
    // TODO: Assert deterministic output blocks match NIST KAT fixtures.
}

#[test]
#[ignore = "skeleton: implement HMAC-DRBG reseed behavior assertions"]
fn hmac_drbg_reseed_behavior() {
    // TODO: Assert post-reseed output divergence and reseed_counter behavior.
}
