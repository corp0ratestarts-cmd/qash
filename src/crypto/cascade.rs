// Astronomical hash cascade — H_cascade as specified in GENESIS_CONSTANTS.toml [crypto.cascade].
//
// Domain: Domain B (PAL/operational).  The output [u8; 64] is deterministic
// across all authorized ISAs; no Domain B nondeterminism influences the bytes.
//
// Architecture:
//   L1  — five primitives in parallel, each domain-separated
//   L2  — SHA3-512 binding of the 160-byte L1 concatenation
//   L3–L6 — SHA3-512 recursive expansion
//   L7  — SHA3-512 finalize → [u8; 64]

use sha3::{Digest, Sha3_256, Sha3_512};
use blake3;
use tiny_keccak::{Hasher as TinyHasher, KangarooTwelve};
use sm3::Sm3;
use streebog::Streebog256;

/// 7-level astronomical cascade over five hash primitives.
/// Output: 64 bytes (SHA3-512 of the L7 layer).
pub fn h_cascade(input: &[u8]) -> [u8; 64] {
    let l1_sep = b"QASH:CASCADE:L1:PARALLEL";

    let h1_sha3   = l1_sha3_256(l1_sep, input);
    let h1_blake3 = l1_blake3(l1_sep, input);
    let h1_k12    = l1_k12(l1_sep, input);
    let h1_sm3    = l1_sm3(l1_sep, input);
    let h1_streeb = l1_streebog(l1_sep, input);

    // Concatenate L1 outputs in canonical order: SHA3-256, BLAKE3, K12, SM3, Streebog
    let mut parallel = [0u8; 160];
    parallel[  0.. 32].copy_from_slice(&h1_sha3);
    parallel[ 32.. 64].copy_from_slice(&h1_blake3);
    parallel[ 64.. 96].copy_from_slice(&h1_k12);
    parallel[ 96..128].copy_from_slice(&h1_sm3);
    parallel[128..160].copy_from_slice(&h1_streeb);

    // L2–L7: SHA3-512 layers with domain separators
    let l2 = sha3_512_layer(b"QASH:CASCADE:L2:BIND",     &parallel);
    let l3 = sha3_512_layer(b"QASH:CASCADE:L3:EXPAND",   &l2);
    let l4 = sha3_512_layer(b"QASH:CASCADE:L4:EXPAND",   &l3);
    let l5 = sha3_512_layer(b"QASH:CASCADE:L5:EXPAND",   &l4);
    let l6 = sha3_512_layer(b"QASH:CASCADE:L6:EXPAND",   &l5);
    sha3_512_layer(b"QASH:CASCADE:L7:FINALIZE", &l6)
}

fn l1_sha3_256(sep: &[u8], input: &[u8]) -> [u8; 32] {
    let mut h = Sha3_256::new();
    h.update(sep);
    h.update(input);
    h.finalize().into()
}

fn l1_blake3(sep: &[u8], input: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(sep);
    h.update(input);
    *h.finalize().as_bytes()
}

fn l1_k12(sep: &[u8], input: &[u8]) -> [u8; 32] {
    let mut h = KangarooTwelve::new(b"");
    TinyHasher::update(&mut h, sep);
    TinyHasher::update(&mut h, input);
    let mut out = [0u8; 32];
    TinyHasher::finalize(h, &mut out);
    out
}

fn l1_sm3(sep: &[u8], input: &[u8]) -> [u8; 32] {
    let mut h = Sm3::new();
    h.update(sep);
    h.update(input);
    h.finalize().into()
}

fn l1_streebog(sep: &[u8], input: &[u8]) -> [u8; 32] {
    let mut h = Streebog256::new();
    h.update(sep);
    h.update(input);
    h.finalize().into()
}

fn sha3_512_layer(sep: &[u8], input: &[u8]) -> [u8; 64] {
    let mut h = Sha3_512::new();
    h.update(sep);
    h.update(input);
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h_cascade_returns_64_nonzero_bytes() {
        let out = h_cascade(b"test input");
        assert_eq!(out.len(), 64);
        assert_ne!(out, [0u8; 64], "cascade must not return all-zeros");
    }

    #[test]
    fn h_cascade_is_deterministic() {
        let a = h_cascade(b"determinism check");
        let b = h_cascade(b"determinism check");
        assert_eq!(a, b);
    }

    #[test]
    fn h_cascade_differs_by_input() {
        let a = h_cascade(b"input A");
        let b = h_cascade(b"input B");
        assert_ne!(a, b);
    }
}
