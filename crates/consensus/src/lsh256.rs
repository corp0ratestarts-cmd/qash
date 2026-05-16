//! LSH-256 (LSH-256-256): Korean standard hash function (KS X 3262, §4).
//!
//! Standard: KS X 3262 (Korean Industrial Standard, formerly TTA TTAS.KO-12.0223).
//!
//! ARX-based, `no_std`, no alloc, no unsafe.  All internal word arithmetic
//! uses wrapping addition because the spec ⊞ operator is defined as
//! `(X + Y) mod 2^w` (§3.4.4) — this is intentional modular arithmetic, not
//! an overflow condition.
//!
//! Parameters (§4, Table 4-1): w = 32, n = 256, N_s = 26,
//! chaining-variable = 512 bits (16 × u32), message block = 1024 bits (32 × u32).

use crate::hash::DomainTag;

// ── Parameters ──────────────────────────────────────────────────────────────

const NS: usize = 26; // number of compression steps

// ── IV: LSH-256-256 initialisation vector (§4.3.2) ─────────────────────────

const IV256: [u32; 16] = [
    0x46a10f1f, 0xfddce486, 0xb41443a8, 0x198e6b9d,
    0x3304388d, 0xb0f5a3c7, 0xb36061c4, 0x7adbd553,
    0x105d5378, 0x2f74de54, 0x5c2f2d95, 0xf2553fbe,
    0x8051357a, 0x138668c8, 0x47aa4484, 0xe01afb41,
];

// ── Permutation τ: MsgExp recurrence (§4.2.1, Table 4-2) ───────────────────

const TAU: [usize; 16] = [3, 2, 0, 1, 7, 4, 5, 6, 11, 10, 8, 9, 15, 12, 13, 14];

// ── Permutation σ: WordPerm (§4.2.4, Table 4-5) ────────────────────────────

const SIGMA: [usize; 16] = [6, 4, 5, 7, 12, 15, 14, 13, 2, 0, 1, 3, 8, 11, 10, 9];

// ── γ: Y-rotation amounts in Mix (§4.2.3, Table 4-3, w = 32) ───────────────

const GAMMA: [u32; 8] = [0, 8, 16, 24, 24, 16, 8, 0];

// ── SC₀: initial step constants (§4.2.3, Table 4-4, w = 32) ────────────────

const SC0: [u32; 8] = [
    0x917c_af90,
    0x6c1b_10a2,
    0x6f35_2943,
    0xcf77_8243,
    0x2ceb_7472,
    0x29e9_6ff2,
    0x8a9b_a428,
    0x2eeb_2642,
];

// ── Rotation selectors (§4.2.3, Table 4-3, w = 32) ─────────────────────────

// α: even step = 29, odd step = 5
#[inline(always)]
fn alpha(step: usize) -> u32 {
    if step & 1 == 0 { 29 } else { 5 }
}

// β: even step = 1, odd step = 17
#[inline(always)]
fn beta(step: usize) -> u32 {
    if step & 1 == 0 { 1 } else { 17 }
}

// ── Core primitives ─────────────────────────────────────────────────────────

/// Mix_{j,l}: two-word ARX mix (§4.2.3 pseudocode).
///
/// Operates on the pair (X = T[l], Y = T[l+8]) for one lane l.
/// Returns (new_X, new_Y).
#[inline(always)]
fn mix_pair(x: u32, y: u32, step: usize, sc_l: u32, gamma_l: u32) -> (u32, u32) {
    let a = alpha(step);
    let b = beta(step);
    // Spec lines, in order:
    //   X ← X ⊞ Y;  X ← X ⋘ αⱼ;
    let x1 = x.wrapping_add(y).rotate_left(a);
    //   X ← X ⊕ SCⱼ[l];
    let x2 = x1 ^ sc_l;
    //   Y ← X ⊞ Y;  Y ← Y ⋘ βⱼ;   (X already updated; Y is original)
    let y1 = x2.wrapping_add(y).rotate_left(b);
    //   X ← X ⊞ Y;  Y ← Y ⋘ γₗ;   (parallel: both read y1)
    let x3 = x2.wrapping_add(y1);
    let y2 = y1.rotate_left(gamma_l);
    (x3, y2)
}

/// Parses a 128-byte block into 32 little-endian u32 words (§4.1.1, eq 4.1).
///
/// Equation 4.1:  M[s] ← m[4s+3] ‖ m[4s+2] ‖ m[4s+1] ‖ m[4s]
/// i.e. byte at address 4s is the least-significant byte of word s.
fn parse_block(b: &[u8; 128]) -> [u32; 32] {
    let mut m = [0u32; 32];
    for s in 0..32usize {
        m[s] = u32::from_le_bytes([b[4 * s], b[4 * s + 1], b[4 * s + 2], b[4 * s + 3]]);
    }
    m
}

/// LSH-256 compression function CF: W¹⁶ × W³² → W¹⁶ (§4.2).
///
/// Uses a rolling 3-slot message buffer to compute the MsgExp recurrence
/// in O(1) space rather than materialising all (N_s + 1) = 27 arrays.
fn compress(cv: [u32; 16], block: &[u32; 32]) -> [u32; 16] {
    // Rolling message buffer: slot j % 3 holds M_j at the start of step j.
    let mut m: [[u32; 16]; 3] = [[0u32; 16]; 3];
    // M_0 = block[0..15], M_1 = block[16..31]  (§4.2.1, eq 4.6 base cases)
    m[0].copy_from_slice(&block[..16]);
    m[1].copy_from_slice(&block[16..32]);

    let mut t = cv;
    let mut sc = SC0; // advances to SC_j at each step via eq (4.9)

    for j in 0..NS {
        // ── MsgAdd: T ⊕= M_j  (§4.2.2, eq 4.7) ──────────────────────────
        let mj = m[j % 3]; // copy — [u32;16] is Copy
        for l in 0..16usize {
            t[l] ^= mj[l];
        }

        // ── Mix_j: 8 parallel two-word lanes  (§4.2.3, eq 4.8) ───────────
        for l in 0..8usize {
            let (x, y) = mix_pair(t[l], t[l + 8], j, sc[l], GAMMA[l]);
            t[l]     = x;
            t[l + 8] = y;
        }

        // ── WordPerm: T ← (T[σ(0)], …, T[σ(15)])  (§4.2.4, eq 4.10) ─────
        let pre = t; // copy
        for l in 0..16usize {
            t[l] = pre[SIGMA[l]];
        }

        // ── Advance SC_j → SC_{j+1}  (eq 4.9) ────────────────────────────
        // SC_j[l] ← SC_{j-1}[l] ⊞ SC_{j-1}[l] ⋘ 8
        if j + 1 < NS {
            for s in &mut sc {
                *s = s.wrapping_add(s.rotate_left(8));
            }
        }

        // ── Compute M_{j+2} for the step two ahead  (eq 4.6 recurrence) ───
        // M_{j+2}[l] = M_{j+1}[l] ⊞ M_j[τ(l)]
        // Write into slot (j+2) % 3, which is free (neither j%3 nor (j+1)%3).
        if j + 2 <= NS {
            let mj_copy  = m[j % 3];       // M_j   (already copied above)
            let mj1_copy = m[(j + 1) % 3]; // M_{j+1}
            let dest     = (j + 2) % 3;
            for l in 0..16usize {
                m[dest][l] = mj1_copy[l].wrapping_add(mj_copy[TAU[l]]);
            }
        }
    }

    // ── Final MsgAdd: CV^{i+1} = T ⊕ M_{N_s}  (§4.2 pseudocode) ──────────
    // After the loop, slot NS % 3 = 26 % 3 = 2 holds M_26.
    let m_ns = m[NS % 3];
    for l in 0..16usize {
        t[l] ^= m_ns[l];
    }

    t
}

/// Extracts the n = 256-bit hash from the final chaining variable (§4.1.3, eq 4.4).
///
/// For w = 32, n = 256:
///   H[l] = CV[l] ⊕ CV[l+8]  (l = 0..7)
///   output = H[0..7] serialised as little-endian u32 words.
fn finalize_256(cv: [u32; 16]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for l in 0..8usize {
        let h = cv[l] ^ cv[l + 8];
        let b = h.to_le_bytes();
        out[4 * l]     = b[0];
        out[4 * l + 1] = b[1];
        out[4 * l + 2] = b[2];
        out[4 * l + 3] = b[3];
    }
    out
}

// ── Public API ───────────────────────────────────────────────────────────────

/// LSH-256-256 of `input`.
///
/// Pads with the one-zeros scheme (§4.1.1): appends 0x80 then zeros to the
/// next 128-byte (1024-bit) boundary.
pub fn lsh256(input: &[u8]) -> [u8; 32] {
    let mut cv = IV256;
    let mut pos = 0usize;

    // Process complete 128-byte (= 1024-bit) message blocks.
    while pos + 128 <= input.len() {
        let mut blk = [0u8; 128];
        blk.copy_from_slice(&input[pos..pos + 128]);
        cv = compress(cv, &parse_block(&blk));
        pos += 128;
    }

    // Final block: copy tail, append 0x80, leave remaining bytes zero.
    let mut pad = [0u8; 128];
    let tail = input.len() - pos;
    if tail > 0 {
        pad[..tail].copy_from_slice(&input[pos..]);
    }
    pad[tail] = 0x80;
    cv = compress(cv, &parse_block(&pad));

    finalize_256(cv)
}

/// Domain-separated LSH-256-256: H(tag_le32 ‖ input).
///
/// Matches the `h_domain` convention in `hash.rs`: prepends the 4-byte
/// little-endian tag before hashing, without requiring heap allocation.
pub fn lsh256_domain(tag: DomainTag, input: &[u8]) -> [u8; 32] {
    let tag_bytes = (tag as u32).to_le_bytes();
    lsh256_parts(&tag_bytes, input)
}

/// LSH-256-256(prefix ‖ suffix) without heap allocation.
///
/// Assembles the virtual concatenation block by block.
pub fn lsh256_parts(prefix: &[u8], suffix: &[u8]) -> [u8; 32] {
    let mut cv   = IV256;
    let total    = prefix.len() + suffix.len();
    let mut pos  = 0usize; // position in the virtual prefix‖suffix stream

    while pos + 128 <= total {
        let mut blk = [0u8; 128];
        for (i, b) in blk.iter_mut().enumerate() {
            let v = pos + i;
            *b = if v < prefix.len() { prefix[v] } else { suffix[v - prefix.len()] };
        }
        cv = compress(cv, &parse_block(&blk));
        pos += 128;
    }

    // Final padded block.
    let mut pad = [0u8; 128];
    let tail = total - pos;
    for (i, b) in pad.iter_mut().enumerate().take(tail) {
        let v = pos + i;
        *b = if v < prefix.len() { prefix[v] } else { suffix[v - prefix.len()] };
    }
    pad[tail] = 0x80;
    cv = compress(cv, &parse_block(&pad));

    finalize_256(cv)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // The step-constant recurrence SC_j[l] = SC_{j-1}[l] ⊞ (SC_{j-1}[l] ⋘ 8).
    // Verify SC_1 derived from SC_0 matches the computed value for word 0:
    //   SC_0[0]          = 0x917c_af90
    //   SC_0[0] ⋘ 8     = 0x7caf_9091   (rotate_left 8)
    //   SC_1[0] (mod 2³²) = 0x0e2c_4021
    #[test]
    fn sc_recurrence_word0() {
        let sc0 = SC0[0];
        let sc1 = sc0.wrapping_add(sc0.rotate_left(8));
        assert_eq!(sc1, 0x0e2c_4021);
    }

    // WordPerm is a bijection: applying it once and then its inverse recovers T.
    // Verify that SIGMA is a valid permutation of 0..15.
    #[test]
    fn sigma_is_permutation() {
        let mut seen = [false; 16];
        for &s in SIGMA.iter() {
            assert!(!seen[s], "sigma has duplicate: {}", s);
            seen[s] = true;
        }
    }

    // TAU must also be a permutation of 0..15.
    #[test]
    fn tau_is_permutation() {
        let mut seen = [false; 16];
        for &t in TAU.iter() {
            assert!(!seen[t], "tau has duplicate: {}", t);
            seen[t] = true;
        }
    }

    // Hashing the same input twice must give identical output (determinism).
    #[test]
    fn determinism() {
        let msg = b"QASH deterministic consensus hash test vector";
        assert_eq!(lsh256(msg), lsh256(msg));
    }

    // Avalanche: single-bit change in input changes output significantly.
    #[test]
    fn avalanche_single_bit() {
        let a = lsh256(b"hello");
        let b = lsh256(b"iello"); // first byte differs by one bit
        assert_ne!(a, b);
        // Count differing bytes — expect roughly half to differ.
        let diff: u32 = a.iter().zip(b.iter()).map(|(x, y)| (x ^ y).count_ones()).sum();
        assert!(diff >= 64, "poor avalanche: only {} bits differ", diff);
    }

    // Output length is always exactly 32 bytes.
    #[test]
    fn output_length() {
        assert_eq!(lsh256(b"").len(), 32);
        assert_eq!(lsh256(b"a").len(), 32);
        assert_eq!(lsh256(&[0u8; 128]).len(), 32); // exactly one block
        assert_eq!(lsh256(&[0u8; 129]).len(), 32); // one full + one partial block
    }

    // Domain separation: different tags produce different hashes for same payload.
    #[test]
    fn domain_separation() {
        use crate::hash::DomainTag;
        let input = b"validator_id_bytes";
        let h1 = lsh256_domain(DomainTag::ValidatorId, input);
        let h2 = lsh256_domain(DomainTag::StateRoot,   input);
        assert_ne!(h1, h2);
    }

    // Prefix concatenation: lsh256_parts(a, b) == lsh256(a ++ b) for small inputs.
    #[test]
    fn parts_matches_lsh256() {
        extern crate std;
        let prefix = b"pre";
        let suffix = b"suffix_data_here";
        let mut combined = std::vec::Vec::new();
        combined.extend_from_slice(prefix);
        combined.extend_from_slice(suffix);
        assert_eq!(lsh256_parts(prefix, suffix), lsh256(&combined));
    }

    // Cross-block boundary: a message that straddles two 128-byte blocks
    // must hash consistently.
    #[test]
    fn two_block_message() {
        let msg_short = [0x5au8; 100];
        let msg_long  = [0x5au8; 200]; // second block required
        // They must differ (different lengths → different padding).
        assert_ne!(lsh256(&msg_short), lsh256(&msg_long));
        // And the two-block result must be deterministic.
        assert_eq!(lsh256(&msg_long), lsh256(&msg_long));
    }

    // Official KAT: LSH-256-256("abc") from KISA source code distribution v1.0.2
    // (seed.kisa.or.kr — Korea Cryptographic Module Validation Programme).
    // This is the pre-genesis CI gate for LSH-256 correctness.
    #[test]
    fn lsh256_official_kat() {
        let expected = [
            0x5f, 0xbf, 0x36, 0x5d, 0xae, 0xa5, 0x44, 0x6a,
            0x70, 0x53, 0xc5, 0x2b, 0x57, 0x40, 0x4d, 0x77,
            0xa0, 0x7a, 0x5f, 0x48, 0xa1, 0xf7, 0xc1, 0x96,
            0x3a, 0x08, 0x98, 0xba, 0x1b, 0x71, 0x47, 0x41,
        ];
        assert_eq!(lsh256(b"abc"), expected);
    }
}
