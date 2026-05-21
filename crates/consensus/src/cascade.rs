// 7-level astronomical cascade — Domain A implementation.
//
// This is the canonical, proof-eligible, no_std version used by
// the consensus path. The qash binary re-exports from here.
//
// Spec: docs/spec/07_hash_cascade.md (normative reference).
//
// Primitive purity (spec §6): all five L1 primitives and SHA3-512 must be
// pure-Rust / safe-Rust.  blake3 uses features=["pure"] to disable assembly.

use sha3::{Digest, Sha3_256, Sha3_512};
use sm3::Sm3;
use streebog::Streebog256;
use tiny_keccak::{Hasher as TinyHasher, KangarooTwelve};

// ---------------------------------------------------------------------------
// Domain separators (spec §7) — pub const so the parity test can compare
// them against GENESIS_CONSTANTS.toml.
// ---------------------------------------------------------------------------

pub const DOM_SEP_L1: &[u8] = b"QASH:CASCADE:L1:PARALLEL";
pub const DOM_SEP_L2: &[u8] = b"QASH:CASCADE:L2:BIND";
pub const DOM_SEP_L3: &[u8] = b"QASH:CASCADE:L3:EXPAND";
pub const DOM_SEP_L4: &[u8] = b"QASH:CASCADE:L4:EXPAND";
pub const DOM_SEP_L5: &[u8] = b"QASH:CASCADE:L5:EXPAND";
pub const DOM_SEP_L6: &[u8] = b"QASH:CASCADE:L6:EXPAND";
pub const DOM_SEP_L7: &[u8] = b"QASH:CASCADE:L7:FINALIZE";

// ---------------------------------------------------------------------------
// §1–§3: unkeyed 7-layer cascade
// ---------------------------------------------------------------------------

/// 7-level astronomical cascade (spec §1–§3). Output: 64 bytes.
pub fn h_cascade(input: &[u8]) -> [u8; 64] {
    h_cascade_keyed(&[], input)
}

// ---------------------------------------------------------------------------
// §4: context-keyed cascade
// ---------------------------------------------------------------------------

/// Context-keyed cascade (spec §4).
///
/// `context_key` is prepended to the L2 binding layer:
///   L2 = SHA3-512( L2_sep ∥ context_key ∥ parallel )
///
/// Must be a deterministic protocol value — never a secret nonce.
pub fn h_cascade_keyed(context_key: &[u8], input: &[u8]) -> [u8; 64] {
    // L1: five primitives in parallel — spec §1
    let h1_sha3 = l1_sha3_256(DOM_SEP_L1, input);
    let h1_blake3 = l1_blake3(DOM_SEP_L1, input);
    let h1_k12 = l1_k12(DOM_SEP_L1, input);
    let h1_sm3 = l1_sm3(DOM_SEP_L1, input);
    let h1_streeb = l1_streebog(DOM_SEP_L1, input);

    // Canonical concat: SHA3-256, BLAKE3, K12, SM3, Streebog — spec §1
    let mut parallel = [0u8; 160];
    parallel[0..32].copy_from_slice(&h1_sha3);
    parallel[32..64].copy_from_slice(&h1_blake3);
    parallel[64..96].copy_from_slice(&h1_k12);
    parallel[96..128].copy_from_slice(&h1_sm3);
    parallel[128..160].copy_from_slice(&h1_streeb);

    // L2: binding layer with optional context_key — spec §2, §4
    let l2 = sha3_512_layer_keyed(DOM_SEP_L2, context_key, &parallel);

    // L3–L7: recursive expansion — spec §2, §3
    let l3 = sha3_512_layer(DOM_SEP_L3, &l2);
    let l4 = sha3_512_layer(DOM_SEP_L4, &l3);
    let l5 = sha3_512_layer(DOM_SEP_L5, &l4);
    let l6 = sha3_512_layer(DOM_SEP_L6, &l5);
    sha3_512_layer(DOM_SEP_L7, &l6)
}

// ---------------------------------------------------------------------------
// §5: hierarchical deterministic derivation
// ---------------------------------------------------------------------------

/// Epoch-keyed cascade root derivation (spec §5).
pub fn h_cascade_derive(parent_root: &[u8; 64], epoch: u64, seed: &[u8; 32]) -> [u8; 64] {
    let mut derive_input = [0u8; 40];
    derive_input[0..8].copy_from_slice(&epoch.to_le_bytes());
    derive_input[8..40].copy_from_slice(seed);
    h_cascade_keyed(parent_root, &derive_input)
}

// ---------------------------------------------------------------------------
// L1 primitive implementations — spec §1, §6 (pure-Rust requirement)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// SHA3-512 layer helpers
// ---------------------------------------------------------------------------

fn sha3_512_layer(sep: &[u8], input: &[u8]) -> [u8; 64] {
    sha3_512_layer_keyed(sep, &[], input)
}

fn sha3_512_layer_keyed(sep: &[u8], key: &[u8], input: &[u8]) -> [u8; 64] {
    let mut h = Sha3_512::new();
    h.update(sep);
    if !key.is_empty() {
        h.update(key);
    }
    h.update(input);
    h.finalize().into()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h_cascade_returns_64_nonzero_bytes() {
        let out = h_cascade(b"test input");
        assert_eq!(out.len(), 64);
        assert_ne!(out, [0u8; 64]);
    }

    #[test]
    fn h_cascade_is_deterministic() {
        assert_eq!(
            h_cascade(b"determinism check"),
            h_cascade(b"determinism check")
        );
    }

    #[test]
    fn h_cascade_differs_by_input() {
        assert_ne!(h_cascade(b"input A"), h_cascade(b"input B"));
    }

    #[test]
    fn keyed_with_empty_key_equals_unkeyed() {
        let input = b"blinding test";
        assert_eq!(h_cascade(input), h_cascade_keyed(&[], input));
    }

    #[test]
    fn keyed_differs_from_unkeyed() {
        let input = b"blinding test";
        let key = b"epoch_seed_32_bytes_padded_here!";
        assert_ne!(h_cascade(input), h_cascade_keyed(key, input));
    }

    #[test]
    fn derive_is_deterministic() {
        let root = h_cascade(b"genesis");
        let seed = [0xabu8; 32];
        assert_eq!(
            h_cascade_derive(&root, 1, &seed),
            h_cascade_derive(&root, 1, &seed)
        );
    }

    #[test]
    fn derive_differs_by_epoch() {
        let root = h_cascade(b"genesis");
        let seed = [0x42u8; 32];
        assert_ne!(
            h_cascade_derive(&root, 0, &seed),
            h_cascade_derive(&root, 1, &seed)
        );
    }
}
