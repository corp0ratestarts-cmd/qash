// Astronomical hash cascade — H_cascade, H_cascade_keyed, H_cascade_derive.
//
// Spec: docs/spec/07_hash_cascade.md (normative reference).
// Domain: Domain B (PAL/operational).  Outputs are deterministic across all
// Tier A ISAs; no Domain B nondeterminism influences any output byte.
//
// Primitive purity requirement (spec §6):
//   All five L1 primitives and SHA3-512 must be pure-Rust / safe-Rust.
//   BLAKE3 uses features=["pure"] to disable assembly/C backends.
//   See Cargo.toml for pinned crate versions.

use sha3::{Digest, Sha3_256, Sha3_512};
use blake3;
use tiny_keccak::{Hasher as TinyHasher, KangarooTwelve};
use sm3::Sm3;
use streebog::Streebog256;

// ---------------------------------------------------------------------------
// §1–§3 of 07_hash_cascade.md: unkeyed 7-layer cascade
// ---------------------------------------------------------------------------

/// 7-level astronomical cascade (spec §1–§3).
/// Output: 64 bytes (SHA3-512 of L7 layer).
///
/// Equivalent to `h_cascade_keyed(&[], input)` per spec §4.
pub fn h_cascade(input: &[u8]) -> [u8; 64] {
    h_cascade_keyed(&[], input)
}

// ---------------------------------------------------------------------------
// §4 of 07_hash_cascade.md: context-keyed cascade (blinding)
// ---------------------------------------------------------------------------

/// Context-keyed cascade (spec §4).
///
/// Identical to `h_cascade` except `context_key` is prepended to the L2
/// binding layer input:
///
///   L2 = SHA3-512( L2_sep ∥ context_key ∥ parallel )
///
/// `context_key` must be a deterministic protocol value, never a secret or
/// entropy-sampled nonce — preserving Domain A replay invariance (spec §4,
/// axiom A5 of 02_transition_axioms.md).
///
/// Usage:
///   - Obfuscation leaf hashing: `h_cascade_keyed(seed_t, validator_id ∥ epoch_le8)`
///   - Genesis hash: `h_cascade_keyed(&[], canonical_genesis_bytes)` (no blinding key)
pub fn h_cascade_keyed(context_key: &[u8], input: &[u8]) -> [u8; 64] {
    let l1_sep = b"QASH:CASCADE:L1:PARALLEL";

    // L1: five primitives in parallel — spec §1
    let h1_sha3   = l1_sha3_256(l1_sep, input);
    let h1_blake3 = l1_blake3(l1_sep, input);
    let h1_k12    = l1_k12(l1_sep, input);
    let h1_sm3    = l1_sm3(l1_sep, input);
    let h1_streeb = l1_streebog(l1_sep, input);

    // Canonical concatenation order: SHA3-256, BLAKE3, K12, SM3, Streebog — spec §1
    let mut parallel = [0u8; 160];
    parallel[  0.. 32].copy_from_slice(&h1_sha3);
    parallel[ 32.. 64].copy_from_slice(&h1_blake3);
    parallel[ 64.. 96].copy_from_slice(&h1_k12);
    parallel[ 96..128].copy_from_slice(&h1_sm3);
    parallel[128..160].copy_from_slice(&h1_streeb);

    // L2: binding layer with optional context_key — spec §2, §4
    let l2 = sha3_512_layer_keyed(b"QASH:CASCADE:L2:BIND", context_key, &parallel);

    // L3–L7: recursive expansion — spec §2, §3
    let l3 = sha3_512_layer(b"QASH:CASCADE:L3:EXPAND",   &l2);
    let l4 = sha3_512_layer(b"QASH:CASCADE:L4:EXPAND",   &l3);
    let l5 = sha3_512_layer(b"QASH:CASCADE:L5:EXPAND",   &l4);
    let l6 = sha3_512_layer(b"QASH:CASCADE:L6:EXPAND",   &l5);
    sha3_512_layer(b"QASH:CASCADE:L7:FINALIZE", &l6)
}

// ---------------------------------------------------------------------------
// §5 of 07_hash_cascade.md: hierarchical deterministic derivation
// ---------------------------------------------------------------------------

/// Epoch-keyed cascade root derivation (spec §5).
///
/// Derives the cascade root for epoch `epoch` from:
///   - `parent_root`: the cascade root of the preceding epoch (or genesis root for epoch 0)
///   - `epoch`: current epoch counter (little-endian 8 bytes)
///   - `seed`: `S_t.entropy_seed` — the epoch entropy seed (01_consensus.md §1)
///
/// ```text
/// derive_input = epoch_le8 ∥ seed       // 40 bytes
/// result       = H_cascade_keyed(parent_root, derive_input)
/// ```
///
/// The epoch chain:
///   cascade_root_0 = H_cascade(canonical_genesis_bytes)
///   cascade_root_t = H_cascade_derive(cascade_root_{t-1}, t, seed_t)
///
/// Cascade proofs in epoch t must be verified against `cascade_root_t`.
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

    // §4: empty key == unkeyed (degenerate case per spec)
    #[test]
    fn keyed_with_empty_key_equals_unkeyed() {
        let input = b"blinding test";
        assert_eq!(h_cascade(input), h_cascade_keyed(&[], input));
    }

    // §4: non-empty key produces distinct output
    #[test]
    fn keyed_differs_from_unkeyed() {
        let input = b"blinding test";
        let key = b"epoch_seed_32_bytes_padded_here!";
        assert_ne!(h_cascade(input), h_cascade_keyed(key, input));
    }

    // §4: two different keys produce different outputs (same input)
    #[test]
    fn different_keys_produce_different_outputs() {
        let input = b"shared input";
        let key_a = b"key_for_epoch_0_________________";
        let key_b = b"key_for_epoch_1_________________";
        assert_ne!(h_cascade_keyed(key_a, input), h_cascade_keyed(key_b, input));
    }

    // §5: HD derivation is deterministic
    #[test]
    fn derive_is_deterministic() {
        let root = h_cascade(b"genesis");
        let seed = [0xabu8; 32];
        assert_eq!(h_cascade_derive(&root, 1, &seed), h_cascade_derive(&root, 1, &seed));
    }

    // §5: epoch differentiation — same root+seed, different epoch → different root
    #[test]
    fn derive_differs_by_epoch() {
        let root = h_cascade(b"genesis");
        let seed = [0x42u8; 32];
        assert_ne!(h_cascade_derive(&root, 0, &seed), h_cascade_derive(&root, 1, &seed));
    }

    // §5: parent differentiation — same epoch+seed, different parent → different root
    #[test]
    fn derive_differs_by_parent() {
        let root_a = h_cascade(b"genesis_a");
        let root_b = h_cascade(b"genesis_b");
        let seed = [0x01u8; 32];
        assert_ne!(h_cascade_derive(&root_a, 5, &seed), h_cascade_derive(&root_b, 5, &seed));
    }
}
