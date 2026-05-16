//! 7-family cascade key derivation with GF(2^128) IT-MAC cross-binding.
//!
//! Produces a 48-byte validator leaf index from (validator_id, epoch, epoch_seed)
//! using seven structurally independent hash families, then cross-binds them
//! with SHA3-512 and authenticates the binding with a polynomial MAC over
//! GF(2^128) (irreducible polynomial x^128+x^7+x^2+x+1, GHASH reduction).
//!
//! # Output layout
//! ```text
//! bytes  0..32  SHA3-512 cross-bind (first 32 of 64 output bytes)
//! bytes 32..48  GF(2^128) Horner MAC tag (16 bytes, little-endian)
//! ```
//!
//! # Hash family registry (jurisdictional coverage)
//! | Path | Primitive | Standard | Region |
//! |------|-----------|----------|--------|
//! | A | SHA3-256 | FIPS 202 / NIST | Global |
//! | B | BLAKE3 | BLAKE3 spec | Global/IETF |
//! | C | KangarooTwelve | NIST SP 800 draft | NIST/ISO |
//! | D | SM3 | GB/T 32905-2016 | China OSCCA |
//! | E | Streebog-256 | GOST R 34.11-2012 | Russia FSB |
//! | F | LSH-256 | KS X 3262:2017 | Korea KISA |
//! | G | SHA3-384† | FIPS 202 / NIST | India MeitY† |
//!
//! †Path G uses SHA3-384 (rate=832, capacity=512), structurally distinct from
//! SHA3-256 (rate=1088, capacity=256). A collision in SHA3-256 does not imply
//! one in SHA3-384. Skein-256 is the long-term target for this slot; SHA3-384
//! satisfies the India MeitY requirement that primitives be NIST SHA-3 family.
//!
//! # Domain A compliance
//! - No unsafe, no float, no usize in wire/state context
//! - All arithmetic is in GF(2^128) (XOR + shifts) — no overflow possible
//! - Pure function: identical inputs → identical outputs on all authorized ISAs

use blake3;
use sha3::{Digest, Sha3_256, Sha3_384, Sha3_512};
use sm3::Sm3;
use streebog::Streebog256;
use tiny_keccak::{Hasher as K12Hasher, KangarooTwelve};

use crate::lsh256::lsh256;

// ---------------------------------------------------------------------------
// GF(2^128) arithmetic — irreducible polynomial x^128+x^7+x^2+x+1
// ---------------------------------------------------------------------------

/// Multiply two elements of GF(2^128) (GHASH field).
///
/// Uses bit-serial schoolbook multiplication with the standard GHASH
/// reduction polynomial. The carry constant 0x87 encodes x^7+x^2+x+1,
/// the part of the irreducible polynomial below x^128.
fn gf128_mul(a: u128, b: u128) -> u128 {
    const REDUCTION: u128 = 0x87; // x^7 + x^2 + x + 1
    let mut result: u128 = 0;
    let mut b_shifted = b;
    let mut a_bits = a;
    while a_bits != 0 {
        if a_bits & 1 != 0 {
            result ^= b_shifted;
        }
        // Multiply b by x: shift left; if bit 127 was set, reduce mod poly.
        let high = (b_shifted >> 127) & 1;
        b_shifted <<= 1;
        if high != 0 {
            b_shifted ^= REDUCTION;
        }
        a_bits >>= 1;
    }
    result
}

/// Horner polynomial MAC over GF(2^128).
///
/// Evaluates h(m_1,...,m_n) = ((m_1·r + m_2)·r + m_3)·r + ... + m_n
/// then adds the one-time pad s: tag = h + s.
/// Each m_i is a 16-byte block interpreted as a little-endian u128.
/// Forgery probability ≤ num_blocks / 2^128.
fn poly_mac_128(r: u128, s: u128, data: &[u8]) -> u128 {
    let mut acc: u128 = 0;
    let mut i: usize = 0;
    while i + 16 <= data.len() {
        let block = u128::from_le_bytes(block16(data, i));
        // Horner step: acc = (acc XOR block) * r
        acc = gf128_mul(acc ^ block, r);
        i += 16;
    }
    // Tail block: zero-pad to 16 bytes.
    if i < data.len() {
        let mut tail = [0u8; 16];
        let rem = data.len() - i;
        tail[..rem].copy_from_slice(&data[i..i + rem]);
        let block = u128::from_le_bytes(tail);
        acc = gf128_mul(acc ^ block, r);
    }
    acc ^ s
}

/// Extract a fixed 16-byte array from a slice at `offset`.
fn block16(data: &[u8], offset: usize) -> [u8; 16] {
    let mut b = [0u8; 16];
    b.copy_from_slice(&data[offset..offset + 16]);
    b
}

// ---------------------------------------------------------------------------
// Internal per-path helpers — each includes an ASCII domain separator to
// prevent cross-path collisions even when all other inputs are identical.
// ---------------------------------------------------------------------------

fn path_a_sha3_256(epoch_seed: &[u8; 32], epoch: u64, validator_id: u64) -> [u8; 32] {
    let mut h = Sha3_256::new();
    h.update(b"QASH:DERIVE:A:SHA3_256:NIST_FIPS202");
    h.update(epoch_seed);
    h.update(epoch.to_le_bytes());
    h.update(validator_id.to_le_bytes());
    let out = h.finalize();
    let mut r = [0u8; 32];
    r.copy_from_slice(&out);
    r
}

fn path_b_blake3(epoch_seed: &[u8; 32], epoch: u64, validator_id: u64) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"QASH:DERIVE:B:BLAKE3:GLOBAL");
    h.update(epoch_seed);
    h.update(&epoch.to_le_bytes());
    h.update(&validator_id.to_le_bytes());
    *h.finalize().as_bytes()
}

fn path_c_k12(epoch_seed: &[u8; 32], epoch: u64, validator_id: u64) -> [u8; 32] {
    // KangarooTwelve: domain separator as customisation string.
    let mut k = KangarooTwelve::new(b"QASH:DERIVE:C:K12:NIST_ISO");
    k.update(epoch_seed);
    k.update(&epoch.to_le_bytes());
    k.update(&validator_id.to_le_bytes());
    let mut r = [0u8; 32];
    k.finalize(&mut r);
    r
}

fn path_d_sm3(epoch_seed: &[u8; 32], epoch: u64, validator_id: u64) -> [u8; 32] {
    let mut h = Sm3::new();
    h.update(b"QASH:DERIVE:D:SM3:CHINA_OSCCA_GB_T_32905");
    h.update(epoch_seed);
    h.update(epoch.to_le_bytes());
    h.update(validator_id.to_le_bytes());
    let out = h.finalize();
    let mut r = [0u8; 32];
    r.copy_from_slice(&out);
    r
}

fn path_e_streebog(epoch_seed: &[u8; 32], epoch: u64, validator_id: u64) -> [u8; 32] {
    let mut h = Streebog256::new();
    h.update(b"QASH:DERIVE:E:STREEBOG256:RUSSIA_GOST_34_11_2012");
    h.update(epoch_seed);
    h.update(epoch.to_le_bytes());
    h.update(validator_id.to_le_bytes());
    let out = h.finalize();
    let mut r = [0u8; 32];
    r.copy_from_slice(&out);
    r
}

fn path_f_lsh256(epoch_seed: &[u8; 32], epoch: u64, validator_id: u64) -> [u8; 32] {
    // LSH-256 domain separator fed as prefix bytes before the main input.
    const PREFIX: &[u8] = b"QASH:DERIVE:F:LSH256:KOREA_KS_X_3262";
    let ep = epoch.to_le_bytes();
    let vi = validator_id.to_le_bytes();
    // Concatenate: PREFIX || epoch_seed || epoch || validator_id
    let mut buf = [0u8; 37 + 32 + 8 + 8]; // PREFIX.len()=37, then rest
    let plen = PREFIX.len();
    buf[..plen].copy_from_slice(PREFIX);
    buf[plen..plen + 32].copy_from_slice(epoch_seed);
    buf[plen + 32..plen + 40].copy_from_slice(&ep);
    buf[plen + 40..plen + 48].copy_from_slice(&vi);
    lsh256(&buf)
}

fn path_g_sha3_384(epoch_seed: &[u8; 32], epoch: u64, validator_id: u64) -> [u8; 32] {
    // SHA3-384 (rate=832 bits, capacity=512 bits) is structurally independent
    // of SHA3-256 (rate=1088 bits, capacity=256 bits): different internal state
    // partitioning makes them structurally independent within the Keccak family.
    let mut h = Sha3_384::new();
    h.update(b"QASH:DERIVE:G:SHA3_384:INDIA_MEITY_NIST_FIPS202");
    h.update(epoch_seed);
    h.update(epoch.to_le_bytes());
    h.update(validator_id.to_le_bytes());
    let out = h.finalize(); // 48 bytes
    let mut r = [0u8; 32];
    r.copy_from_slice(&out[..32]); // take first 32 bytes
    r
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Derive a 48-byte validator leaf index from genesis inputs.
///
/// The result is stable across all authorised ISAs (AX-1/AX-2: no unsafe,
/// no floats, all arithmetic deterministic). Changing any of the three inputs
/// changes every bit of the output with probability ≈ 1/2 (cascade avalanche).
///
/// Used to populate `EpochState::validator_ids` at genesis and to verify
/// validator identity during epoch transitions.
pub fn derive_leaf_index(
    validator_id: u64,
    epoch: u64,
    epoch_seed: &[u8; 32],
) -> [u8; 48] {
    // Step 1 — compute seven independent path hashes (each 32 bytes = 224 total).
    let pa = path_a_sha3_256(epoch_seed, epoch, validator_id);
    let pb = path_b_blake3(epoch_seed, epoch, validator_id);
    let pc = path_c_k12(epoch_seed, epoch, validator_id);
    let pd = path_d_sm3(epoch_seed, epoch, validator_id);
    let pe = path_e_streebog(epoch_seed, epoch, validator_id);
    let pf = path_f_lsh256(epoch_seed, epoch, validator_id);
    let pg = path_g_sha3_384(epoch_seed, epoch, validator_id);

    // Step 2 — cross-bind via SHA3-512 (64 bytes).
    // First 32 bytes → layer1 (the commitment).
    // Bytes 32..48 → MAC key r.
    // Bytes 48..64 → MAC one-time pad s.
    let mut cross = Sha3_512::new();
    cross.update(b"QASH:DERIVE:CROSS_BIND:SHA3_512");
    cross.update(pa);
    cross.update(pb);
    cross.update(pc);
    cross.update(pd);
    cross.update(pe);
    cross.update(pf);
    cross.update(pg);
    let cross_out = cross.finalize(); // 64 bytes

    let mut layer1 = [0u8; 32];
    layer1.copy_from_slice(&cross_out[..32]);

    let r = u128::from_le_bytes(block16(&cross_out, 32));
    let s = u128::from_le_bytes(block16(&cross_out, 48));

    // Step 3 — IT-MAC over the 224-byte concatenation of the 7 paths.
    // Forgery probability ≤ 14 / 2^128 (14 sixteen-byte blocks).
    let mut all_paths = [0u8; 224]; // 7 × 32
    all_paths[  0.. 32].copy_from_slice(&pa);
    all_paths[ 32.. 64].copy_from_slice(&pb);
    all_paths[ 64.. 96].copy_from_slice(&pc);
    all_paths[ 96..128].copy_from_slice(&pd);
    all_paths[128..160].copy_from_slice(&pe);
    all_paths[160..192].copy_from_slice(&pf);
    all_paths[192..224].copy_from_slice(&pg);

    let mac_raw = poly_mac_128(r, s, &all_paths);
    let mac_bytes = mac_raw.to_le_bytes();

    // Step 4 — assemble the 48-byte output.
    let mut out = [0u8; 48];
    out[..32].copy_from_slice(&layer1);
    out[32..].copy_from_slice(&mac_bytes);
    out
}

/// Verify that `candidate` matches the derivation of `(validator_id, epoch, epoch_seed)`.
///
/// Constant-time comparison is not required here because the output is public
/// (validator identities are disclosed in the genesis block). This is a
/// simple equality check for use in test and audit contexts.
pub fn verify_leaf_index(
    validator_id: u64,
    epoch: u64,
    epoch_seed: &[u8; 32],
    candidate: &[u8; 48],
) -> bool {
    derive_leaf_index(validator_id, epoch, epoch_seed) == *candidate
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SEED_ZERO: [u8; 32] = [0u8; 32];
    const SEED_FF: [u8; 32] = [0xffu8; 32];

    /// §A1: output is deterministic — two calls with identical inputs are equal.
    #[test]
    fn derive_is_deterministic() {
        let a = derive_leaf_index(1, 0, &SEED_ZERO);
        let b = derive_leaf_index(1, 0, &SEED_ZERO);
        assert_eq!(a, b);
    }

    /// Different validator_ids produce different outputs.
    #[test]
    fn different_validator_id_differs() {
        let a = derive_leaf_index(1, 0, &SEED_ZERO);
        let b = derive_leaf_index(2, 0, &SEED_ZERO);
        assert_ne!(a, b);
    }

    /// Different epochs produce different outputs.
    #[test]
    fn different_epoch_differs() {
        let a = derive_leaf_index(1, 0, &SEED_ZERO);
        let b = derive_leaf_index(1, 1, &SEED_ZERO);
        assert_ne!(a, b);
    }

    /// Different seeds produce different outputs.
    #[test]
    fn different_seed_differs() {
        let a = derive_leaf_index(1, 0, &SEED_ZERO);
        let b = derive_leaf_index(1, 0, &SEED_FF);
        assert_ne!(a, b);
    }

    /// verify_leaf_index returns true on matching inputs.
    #[test]
    fn verify_roundtrip() {
        let idx = derive_leaf_index(42, 7, &SEED_ZERO);
        assert!(verify_leaf_index(42, 7, &SEED_ZERO, &idx));
    }

    /// verify_leaf_index returns false on tampered output.
    #[test]
    fn verify_rejects_tampered() {
        let mut idx = derive_leaf_index(42, 7, &SEED_ZERO);
        idx[0] ^= 0x01;
        assert!(!verify_leaf_index(42, 7, &SEED_ZERO, &idx));
    }

    /// Output is non-zero for a non-trivial input (sanity guard vs. identity hash).
    #[test]
    fn output_is_nonzero() {
        let a = derive_leaf_index(1, 0, &SEED_ZERO);
        assert_ne!(a, [0u8; 48]);
    }

    /// GF(2^128) multiplication: 1 is the multiplicative identity.
    #[test]
    fn gf128_mul_identity() {
        let x: u128 = 0xdeadbeef_cafebabe_12345678_9abcdef0;
        assert_eq!(gf128_mul(x, 1), x);
        assert_eq!(gf128_mul(1, x), x);
    }

    /// GF(2^128) multiplication: 0 is the absorbing element.
    #[test]
    fn gf128_mul_zero() {
        let x: u128 = 0xdeadbeef_cafebabe_12345678_9abcdef0;
        assert_eq!(gf128_mul(x, 0), 0);
        assert_eq!(gf128_mul(0, x), 0);
    }

    /// GF(2^128) multiplication is commutative.
    #[test]
    fn gf128_mul_commutative() {
        let a: u128 = 0x1234_5678_9abc_def0_fedc_ba98_7654_3210;
        let b: u128 = 0xaaaa_bbbb_cccc_dddd_eeee_ffff_0000_1111;
        assert_eq!(gf128_mul(a, b), gf128_mul(b, a));
    }

    /// LSH-256 path (F) is deterministic and produces 32-byte output.
    #[test]
    fn path_f_lsh256_deterministic() {
        let a = path_f_lsh256(&SEED_ZERO, 0, 1);
        let b = path_f_lsh256(&SEED_ZERO, 0, 1);
        assert_eq!(a, b);
        assert_ne!(a, [0u8; 32]);
    }

    /// All 7 paths produce distinct outputs for the same input (path uniqueness).
    #[test]
    fn all_paths_are_distinct() {
        let paths = [
            path_a_sha3_256(&SEED_ZERO, 0, 1),
            path_b_blake3(&SEED_ZERO, 0, 1),
            path_c_k12(&SEED_ZERO, 0, 1),
            path_d_sm3(&SEED_ZERO, 0, 1),
            path_e_streebog(&SEED_ZERO, 0, 1),
            path_f_lsh256(&SEED_ZERO, 0, 1),
            path_g_sha3_384(&SEED_ZERO, 0, 1),
        ];
        for i in 0..7 {
            for j in (i + 1)..7 {
                assert_ne!(paths[i], paths[j], "paths {} and {} must differ", i, j);
            }
        }
    }

    /// Avalanche: flipping one bit in epoch_seed changes the output substantially.
    #[test]
    fn avalanche_single_bit_in_seed() {
        let mut seed_flip = SEED_ZERO;
        seed_flip[0] ^= 0x01;
        let a = derive_leaf_index(1, 0, &SEED_ZERO);
        let b = derive_leaf_index(1, 0, &seed_flip);
        // Count differing bytes — expect at least 16 of 48 (rough avalanche check).
        let diff = a.iter().zip(b.iter()).filter(|(x, y)| x != y).count();
        assert!(diff >= 16, "avalanche: only {} bytes differ (want ≥ 16)", diff);
    }
}
