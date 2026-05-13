//! Domain-separated hashing (consensus-critical).
//!
//! State roots use a multi-primitive commitment:
//!
//! `R = SHA3-256(CONSENSUS_ROOT || n || id_1 || H_1(tag || input) || ...)`
//!
//! Every authorized primitive contributes a sub-root. A primitive that is only
//! logged but omitted from this construction is not consensus-active.

use sha3::{Digest, Sha3_256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DomainTag {
    StateRoot = 0x0000_0001,
    EntropyAdvance = 0x0000_0002,
    ValidatorId = 0x0000_0003,
    LeafHash = 0x0000_0004,
    InternalHash = 0x0000_0005,
    ConsensusRoot = 0x0000_0006,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum HashPrimitive {
    Sha3_256 = 0x0000_0001,
    Sm3 = 0x0000_0002,
}

pub const CONSENSUS_HASH_PRIMITIVE_COUNT: usize = 2;
pub const DIGEST_BYTES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimitiveDigest {
    pub primitive: HashPrimitive,
    pub digest: [u8; DIGEST_BYTES],
}

pub type ConsensusDigestSet = [PrimitiveDigest; CONSENSUS_HASH_PRIMITIVE_COUNT];

/// Domain-separated SHA3-256. This remains the canonical primitive for entropy
/// advancement and for folding the active primitive digest set into one root.
pub fn h_domain(tag: DomainTag, input: &[u8]) -> [u8; DIGEST_BYTES] {
    sha3_256_tagged(tag, input)
}

/// Domain-separated multi-primitive root. Use for consensus commitments whose
/// security must not depend on a single primitive.
pub fn h_consensus_domain(tag: DomainTag, input: &[u8]) -> [u8; DIGEST_BYTES] {
    let roots = consensus_primitive_digests(tag, input);
    combine_primitive_digests(&roots)
}

pub fn consensus_primitive_digests(tag: DomainTag, input: &[u8]) -> ConsensusDigestSet {
    [
        PrimitiveDigest {
            primitive: HashPrimitive::Sha3_256,
            digest: sha3_256_tagged(tag, input),
        },
        PrimitiveDigest {
            primitive: HashPrimitive::Sm3,
            digest: sm3_256_tagged(tag, input),
        },
    ]
}

pub fn combine_primitive_digests(roots: &ConsensusDigestSet) -> [u8; DIGEST_BYTES] {
    let mut hasher = Sha3_256::new();
    hasher.update((DomainTag::ConsensusRoot as u32).to_le_bytes());
    hasher.update((roots.len() as u32).to_le_bytes());

    for root in roots {
        hasher.update((root.primitive as u32).to_le_bytes());
        hasher.update(root.digest);
    }

    let out = hasher.finalize();
    let mut res = [0u8; DIGEST_BYTES];
    res.copy_from_slice(&out);
    res
}

/// Untagged SHA3-256 (only where the spec explicitly requires it).
pub fn sha3_256(input: &[u8]) -> [u8; DIGEST_BYTES] {
    let mut hasher = Sha3_256::new();
    hasher.update(input);
    let out = hasher.finalize();
    let mut res = [0u8; DIGEST_BYTES];
    res.copy_from_slice(&out);
    res
}

fn sha3_256_tagged(tag: DomainTag, input: &[u8]) -> [u8; DIGEST_BYTES] {
    let mut hasher = Sha3_256::new();
    hasher.update((tag as u32).to_le_bytes());
    hasher.update(input);
    let out = hasher.finalize();
    let mut res = [0u8; DIGEST_BYTES];
    res.copy_from_slice(&out);
    res
}

fn sm3_256_tagged(tag: DomainTag, input: &[u8]) -> [u8; DIGEST_BYTES] {
    let mut sm3_input = [0u8; 4];
    sm3_input.copy_from_slice(&(tag as u32).to_le_bytes());
    sm3_256_two_part(&sm3_input, input)
}

/// SM3 over `prefix || input`.
///
/// This small no-std implementation is intentionally local to the consensus
/// crate so the active multi-primitive root does not depend on platform-specific
/// acceleration or external optional features.
fn sm3_256_two_part(prefix: &[u8], input: &[u8]) -> [u8; DIGEST_BYTES] {
    const IV: [u32; 8] = [
        0x7380_166f,
        0x4914_b2b9,
        0x1724_42d7,
        0xda8a_0600,
        0xa96f_30bc,
        0x1631_38aa,
        0xe38d_ee4d,
        0xb0fb_0e4e,
    ];

    let mut state = IV;
    let total_len = prefix.len() + input.len();
    let mut block = [0u8; 64];
    let mut block_len = 0usize;

    for b in prefix.iter().chain(input.iter()) {
        block[block_len] = *b;
        block_len += 1;
        if block_len == 64 {
            sm3_compress(&mut state, &block);
            block = [0u8; 64];
            block_len = 0;
        }
    }

    block[block_len] = 0x80;
    block_len += 1;

    if block_len > 56 {
        while block_len < 64 {
            block[block_len] = 0;
            block_len += 1;
        }
        sm3_compress(&mut state, &block);
        block = [0u8; 64];
        block_len = 0;
    }

    while block_len < 56 {
        block[block_len] = 0;
        block_len += 1;
    }

    let bit_len = (total_len as u64).wrapping_mul(8);
    block[56..64].copy_from_slice(&bit_len.to_be_bytes());
    sm3_compress(&mut state, &block);

    let mut out = [0u8; DIGEST_BYTES];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn sm3_compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 68];
    let mut w1 = [0u32; 64];

    let mut i = 0usize;
    while i < 16 {
        let offset = i * 4;
        w[i] = u32::from_be_bytes([
            block[offset],
            block[offset + 1],
            block[offset + 2],
            block[offset + 3],
        ]);
        i += 1;
    }

    while i < 68 {
        w[i] = sm3_p1(w[i - 16] ^ w[i - 9] ^ w[i - 3].rotate_left(15))
            ^ w[i - 13].rotate_left(7)
            ^ w[i - 6];
        i += 1;
    }

    i = 0;
    while i < 64 {
        w1[i] = w[i] ^ w[i + 4];
        i += 1;
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    let mut f = state[5];
    let mut g = state[6];
    let mut h = state[7];

    i = 0;
    while i < 64 {
        let t: u32 = if i < 16 { 0x79cc_4519 } else { 0x7a87_9d8a };
        let ss1 = (a
            .rotate_left(12)
            .wrapping_add(e)
            .wrapping_add(t.rotate_left(i as u32)))
        .rotate_left(7);
        let ss2 = ss1 ^ a.rotate_left(12);
        let tt1 = sm3_ff(i, a, b, c)
            .wrapping_add(d)
            .wrapping_add(ss2)
            .wrapping_add(w1[i]);
        let tt2 = sm3_gg(i, e, f, g)
            .wrapping_add(h)
            .wrapping_add(ss1)
            .wrapping_add(w[i]);
        d = c;
        c = b.rotate_left(9);
        b = a;
        a = tt1;
        h = g;
        g = f.rotate_left(19);
        f = e;
        e = sm3_p0(tt2);
        i += 1;
    }

    state[0] ^= a;
    state[1] ^= b;
    state[2] ^= c;
    state[3] ^= d;
    state[4] ^= e;
    state[5] ^= f;
    state[6] ^= g;
    state[7] ^= h;
}

#[inline]
fn sm3_ff(round: usize, x: u32, y: u32, z: u32) -> u32 {
    if round < 16 {
        x ^ y ^ z
    } else {
        (x & y) | (x & z) | (y & z)
    }
}

#[inline]
fn sm3_gg(round: usize, x: u32, y: u32, z: u32) -> u32 {
    if round < 16 {
        x ^ y ^ z
    } else {
        (x & y) | (!x & z)
    }
}

#[inline]
fn sm3_p0(x: u32) -> u32 {
    x ^ x.rotate_left(9) ^ x.rotate_left(17)
}

#[inline]
fn sm3_p1(x: u32) -> u32 {
    x ^ x.rotate_left(15) ^ x.rotate_left(23)
}

#[cfg(test)]
mod tests {
    use super::{sm3_256_two_part, DIGEST_BYTES};

    #[test]
    fn consensus_root_depends_on_each_active_primitive() {
        let roots = super::consensus_primitive_digests(super::DomainTag::StateRoot, b"state");
        assert_eq!(roots[0].primitive, super::HashPrimitive::Sha3_256);
        assert_eq!(roots[1].primitive, super::HashPrimitive::Sm3);

        let baseline = super::combine_primitive_digests(&roots);

        let mut changed_sha3 = roots;
        changed_sha3[0].digest[0] ^= 0x01;
        assert_ne!(super::combine_primitive_digests(&changed_sha3), baseline);

        let mut changed_sm3 = roots;
        changed_sm3[1].digest[0] ^= 0x01;
        assert_ne!(super::combine_primitive_digests(&changed_sm3), baseline);
    }

    #[test]
    fn sm3_abc_vector() {
        let expected: [u8; DIGEST_BYTES] = [
            0x66, 0xc7, 0xf0, 0xf4, 0x62, 0xee, 0xed, 0xd9, 0xd1, 0xf2, 0xd4, 0x6b, 0xdc, 0x10,
            0xe4, 0xe2, 0x41, 0x67, 0xc4, 0x87, 0x5c, 0xf2, 0xf7, 0xa2, 0x29, 0x7d, 0xa0, 0x2b,
            0x8f, 0x4b, 0xa8, 0xe0,
        ];
        assert_eq!(sm3_256_two_part(&[], b"abc"), expected);
    }
}
