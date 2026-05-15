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
    ValidatorId    = 0x0000_0003,
    LeafHash       = 0x0000_0004,
    InternalHash   = 0x0000_0005,
    TxId           = 0x0000_0010,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_tags_have_correct_values() {
        assert_eq!(DomainTag::StateRoot      as u32, 0x0000_0001);
        assert_eq!(DomainTag::EntropyAdvance as u32, 0x0000_0002);
        assert_eq!(DomainTag::ValidatorId    as u32, 0x0000_0003);
        assert_eq!(DomainTag::LeafHash       as u32, 0x0000_0004);
        assert_eq!(DomainTag::InternalHash   as u32, 0x0000_0005);
        assert_eq!(DomainTag::TxId           as u32, 0x0000_0010);
    }

    #[test]
    fn h_domain_deterministic() {
        let a = h_domain(DomainTag::StateRoot, b"hello");
        let b = h_domain(DomainTag::StateRoot, b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn h_domain_different_tags_produce_different_output() {
        let a = h_domain(DomainTag::StateRoot,      b"data");
        let b = h_domain(DomainTag::EntropyAdvance, b"data");
        assert_ne!(a, b);
    }

    #[test]
    fn h_domain_different_inputs_produce_different_output() {
        let a = h_domain(DomainTag::StateRoot, b"input_a");
        let b = h_domain(DomainTag::StateRoot, b"input_b");
        assert_ne!(a, b);
    }

    #[test]
    fn sha3_256_known_vector() {
        // SHA3-256 of the empty string — NIST FIPS 202 reference value.
        let expected: [u8; 32] = [
            0xa7, 0xff, 0xc6, 0xf8, 0xbf, 0x1e, 0xd7, 0x66,
            0x51, 0xc1, 0x47, 0x56, 0xa0, 0x61, 0xd6, 0x62,
            0xf5, 0x80, 0xff, 0x4d, 0xe4, 0x3b, 0x49, 0xfa,
            0x82, 0xd8, 0x0a, 0x4b, 0x80, 0xf8, 0x43, 0x4a,
        ];
        assert_eq!(sha3_256(b""), expected);
    }

    /// TV-8: domain-separated hash cascade is deterministic across two independent
    /// calls on the same input.  Guards against platform nondeterminism in SHA3-256.
    /// BLAKE3 and KangarooTwelve cascade stages are deferred to Domain B.
    #[test]
    fn cascade_determinism_same_input() {
        let seed = [0x5a_u8; 32];
        let step1_a = h_domain(DomainTag::EntropyAdvance, &seed);
        let step2_a = h_domain(DomainTag::InternalHash,   &step1_a);

        let step1_b = h_domain(DomainTag::EntropyAdvance, &seed);
        let step2_b = h_domain(DomainTag::InternalHash,   &step1_b);

        assert_eq!(step1_a, step1_b);
        assert_eq!(step2_a, step2_b);
        // Output must be non-zero (sanity check against identity hash bugs).
        assert_ne!(step2_a, [0u8; 32]);
    }
}
