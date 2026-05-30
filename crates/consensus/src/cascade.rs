// 7-level astronomical cascade — Domain A implementation.
//
// This is the canonical, proof-eligible, no_std version used by
// the consensus path. The qash binary re-exports from here.
//
// Spec: docs/spec/07_hash_cascade.md (normative reference).
//
// Primitive purity (spec §6): all seven L1 primitives and SHA3-512 must be
// pure-Rust / safe-Rust.  blake3 uses features=["pure"] to disable assembly.

use sha3::{Digest, Sha3_512};
use sm3::Sm3;
use streebog::Streebog512;
use kupyna::Kupyna512;
use tiny_keccak::{Hasher as TinyHasher, KangarooTwelve};
use crate::lsh512::lsh512_parts;

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
    // L1: seven 512-bit primitives in parallel — spec §1 (QASH-CASCADE-7)
    let h1_sha3   = l1_sha3_512(DOM_SEP_L1, input);
    let h1_blake3 = l1_blake3_64(DOM_SEP_L1, input);
    let h1_k12    = l1_k12_64(DOM_SEP_L1, input);
    let h1_sm3    = l1_sm3_512(DOM_SEP_L1, input);
    let h1_streeb = l1_streebog_512(DOM_SEP_L1, input);
    let h1_kupyna = l1_kupyna_512(DOM_SEP_L1, input);
    let h1_lsh    = l1_lsh_512(DOM_SEP_L1, input);

    // Canonical concat: SHA3-512, BLAKE3-XOF, K12-XOF, SM3×2, Streebog-512,
    //                   Kupyna-512, LSH-512 — spec §1 (7 × 64 = 448 bytes)
    let mut parallel = [0u8; 448];
    parallel[0..64].copy_from_slice(&h1_sha3);
    parallel[64..128].copy_from_slice(&h1_blake3);
    parallel[128..192].copy_from_slice(&h1_k12);
    parallel[192..256].copy_from_slice(&h1_sm3);
    parallel[256..320].copy_from_slice(&h1_streeb);
    parallel[320..384].copy_from_slice(&h1_kupyna);
    parallel[384..448].copy_from_slice(&h1_lsh);

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
// §6: GRC hedge-root exposure
// ---------------------------------------------------------------------------

/// Returns the seven 64-byte L1 primitive outputs in slot order:
/// [SHA3-512, BLAKE3-XOF-64, K12-XOF-64, SM3-double, Streebog-512, Kupyna-512, LSH-512].
/// Used by the genesis certificate generator to populate [genesis.hedge_roots].
/// Not part of the consensus path; Domain B use only.
pub fn h_cascade_l1_primitives(input: &[u8]) -> [[u8; 64]; 7] {
    [
        l1_sha3_512(DOM_SEP_L1, input),
        l1_blake3_64(DOM_SEP_L1, input),
        l1_k12_64(DOM_SEP_L1, input),
        l1_sm3_512(DOM_SEP_L1, input),
        l1_streebog_512(DOM_SEP_L1, input),
        l1_kupyna_512(DOM_SEP_L1, input),
        l1_lsh_512(DOM_SEP_L1, input),
    ]
}

// ---------------------------------------------------------------------------
// L1 primitive implementations — spec §1, §6 (pure-Rust requirement)
// All return 64 bytes for uniform 256-bit quantum collision resistance.
// ---------------------------------------------------------------------------

fn l1_sha3_512(sep: &[u8], input: &[u8]) -> [u8; 64] {
    let mut h = Sha3_512::new();
    h.update(sep);
    h.update(input);
    h.finalize().into()
}

fn l1_blake3_64(sep: &[u8], input: &[u8]) -> [u8; 64] {
    let mut h = blake3::Hasher::new();
    h.update(sep);
    h.update(input);
    let mut out = [0u8; 64];
    h.finalize_xof().fill(&mut out);
    out
}

fn l1_k12_64(sep: &[u8], input: &[u8]) -> [u8; 64] {
    let mut h = KangarooTwelve::new(b"");
    TinyHasher::update(&mut h, sep);
    TinyHasher::update(&mut h, input);
    let mut out = [0u8; 64];
    TinyHasher::finalize(h, &mut out);
    out
}

fn l1_sm3_512(sep: &[u8], input: &[u8]) -> [u8; 64] {
    // SM3 is natively 256-bit; double-width domain-separated construction
    // yields 64 bytes of independent SM3 output (prefix bytes 0x01, 0x02).
    let mut out = [0u8; 64];
    let mut h1 = Sm3::new();
    h1.update([0x01]);
    h1.update(sep);
    h1.update(input);
    out[..32].copy_from_slice(&h1.finalize());
    let mut h2 = Sm3::new();
    h2.update([0x02]);
    h2.update(sep);
    h2.update(input);
    out[32..64].copy_from_slice(&h2.finalize());
    out
}

fn l1_streebog_512(sep: &[u8], input: &[u8]) -> [u8; 64] {
    let mut h = Streebog512::new();
    h.update(sep);
    h.update(input);
    h.finalize().into()
}

fn l1_kupyna_512(sep: &[u8], input: &[u8]) -> [u8; 64] {
    use kupyna::digest::Digest as _;
    let mut h = Kupyna512::new();
    h.update(sep);
    h.update(input);
    h.finalize().into()
}

fn l1_lsh_512(sep: &[u8], input: &[u8]) -> [u8; 64] {
    lsh512_parts(sep, input)
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
