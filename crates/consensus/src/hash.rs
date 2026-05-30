//! Domain-separated hashing (consensus-critical).
//!
//! H_domain(tag, input) = SHA3-256( tag_u32_le || input )
//!
//! SECURITY NOTE: `h_domain` and `sha3_256` are NOT constant-time with respect to
//! their `input` argument. This is intentional and safe because all current callers in
//! Domain A pass **public consensus data** (state roots, validator IDs, entropy
//! seeds, transaction bytes). These functions MUST NOT be called on secret material
//! (private keys, blinding scalars, signing nonces). Such use belongs in Domain B
//! with an appropriate constant-time wrapper.
//!
//! Threat-model boundary: collision/injectivity guarantees used by Coq proofs are
//! modeled as AX-3 cryptographic assumptions (see `proofs/contractivity/encode_injectivity.v`),
//! not as implementation-level side-channel proofs.

use sha3::{Digest, Sha3_256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DomainTag {
    StateRoot = 0x0000_0001,
    EntropyAdvance = 0x0000_0002,
    ValidatorId = 0x0000_0003,
    LeafHash = 0x0000_0004,
    InternalHash = 0x0000_0005,
    TxId = 0x0000_0010,
    CausalOrder = 0x0000_0020,
    LineageSkip = 0x0000_0040,
    CausalFingerprint = 0x0000_0030,
    ShardAssignment = 0x0000_0050,
    CrossShardReceipt = 0x0000_0051,
    EpochFinalityBeacon = 0x0000_0052,
}

pub fn h_domain(tag: DomainTag, input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update((tag as u32).to_le_bytes());
    hasher.update(input);
    let out = hasher.finalize();
    let mut res = [0u8; 32];
    res.copy_from_slice(&out);
    res
}

/// Return a SHA3-256 hasher pre-seeded with `tag` as the domain separator.
/// Feed subsequent data with `hasher.update(...)`, then finish with
/// `h_domain_finish`. Equivalent to `h_domain(tag, concat(chunks))`.
pub(crate) fn h_domain_start(tag: DomainTag) -> Sha3_256 {
    let mut h = Sha3_256::new();
    h.update((tag as u32).to_le_bytes());
    h
}

/// Consume a hasher started with `h_domain_start` and return the digest.
pub(crate) fn h_domain_finish(h: Sha3_256) -> [u8; 32] {
    let out = h.finalize();
    let mut res = [0u8; 32];
    res.copy_from_slice(&out);
    res
}

/// Untagged SHA3-256 (only where the spec explicitly requires it).
pub fn sha3_256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(input);
    let out = hasher.finalize();
    let mut res = [0u8; 32];
    res.copy_from_slice(&out);
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_tags_have_correct_values() {
        assert_eq!(DomainTag::StateRoot as u32, 0x0000_0001);
        assert_eq!(DomainTag::EntropyAdvance as u32, 0x0000_0002);
        assert_eq!(DomainTag::ValidatorId as u32, 0x0000_0003);
        assert_eq!(DomainTag::LeafHash as u32, 0x0000_0004);
        assert_eq!(DomainTag::InternalHash as u32, 0x0000_0005);
        assert_eq!(DomainTag::TxId as u32, 0x0000_0010);
    }

    #[test]
    fn h_domain_deterministic() {
        let a = h_domain(DomainTag::StateRoot, b"hello");
        let b = h_domain(DomainTag::StateRoot, b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn h_domain_different_tags_produce_different_output() {
        let a = h_domain(DomainTag::StateRoot, b"data");
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
    fn h_domain_state_root_hello_known_vector() {
        // sha3_256([01 00 00 00] || "hello")
        let expected: [u8; 32] = [
            0x73, 0xde, 0x9c, 0xe1, 0xb7, 0x47, 0xa5, 0xff, 0xf1, 0xaa, 0x27, 0x0e, 0xa5, 0xc6,
            0x75, 0xc1, 0xb3, 0x84, 0xb5, 0xe9, 0xf9, 0xc1, 0x3f, 0x84, 0x9f, 0x47, 0xb2, 0xde,
            0xf6, 0xe5, 0x55, 0xca,
        ];
        assert_eq!(h_domain(DomainTag::StateRoot, b"hello"), expected);
    }

    #[test]
    fn sha3_256_known_vector() {
        // SHA3-256 of the empty string — NIST FIPS 202 reference value.
        let expected: [u8; 32] = [
            0xa7, 0xff, 0xc6, 0xf8, 0xbf, 0x1e, 0xd7, 0x66, 0x51, 0xc1, 0x47, 0x56, 0xa0, 0x61,
            0xd6, 0x62, 0xf5, 0x80, 0xff, 0x4d, 0xe4, 0x3b, 0x49, 0xfa, 0x82, 0xd8, 0x0a, 0x4b,
            0x80, 0xf8, 0x43, 0x4a,
        ];
        assert_eq!(sha3_256(b""), expected);
    }

    // ── CAVP KAT: SHA3-256 (NIST FIPS 202) ──────────────────────────────────────

    /// CAVP gate: NIST FIPS 202 SHA3-256 known-answer test vectors.
    ///
    /// These three vectors are taken directly from the NIST FIPS 202 standard
    /// (Appendix B). Any implementation divergence from these values indicates
    /// a non-compliant SHA3-256 primitive and MUST block the CI merge gate.
    ///
    /// CI gate: `cargo test -p qash-consensus --no-default-features -- cavp_sha3_256`
    #[test]
    fn cavp_sha3_256() {
        // FIPS 202 §B.1 — empty message
        assert_eq!(
            sha3_256(b""),
            [
                0xa7, 0xff, 0xc6, 0xf8, 0xbf, 0x1e, 0xd7, 0x66, 0x51, 0xc1, 0x47, 0x56, 0xa0, 0x61,
                0xd6, 0x62, 0xf5, 0x80, 0xff, 0x4d, 0xe4, 0x3b, 0x49, 0xfa, 0x82, 0xd8, 0x0a, 0x4b,
                0x80, 0xf8, 0x43, 0x4a,
            ],
            "SHA3-256 empty-message vector mismatch"
        );
        // FIPS 202 §B.1 — "abc" (3 bytes)
        assert_eq!(
            sha3_256(b"abc"),
            [
                0x3a, 0x98, 0x5d, 0xa7, 0x4f, 0xe2, 0x25, 0xb2, 0x04, 0x5c, 0x17, 0x2d, 0x6b, 0xd3,
                0x90, 0xbd, 0x85, 0x5f, 0x08, 0x6e, 0x3e, 0x9d, 0x52, 0x5b, 0x46, 0xbf, 0xe2, 0x45,
                0x11, 0x43, 0x15, 0x32,
            ],
            "SHA3-256 'abc' vector mismatch"
        );
        // FIPS 202 — 200 bytes of 0xa3 (Appendix A.1 reference)
        assert_eq!(
            sha3_256(&[0xa3u8; 200]),
            [
                0x79, 0xf3, 0x8a, 0xde, 0xc5, 0xc2, 0x03, 0x07, 0xa9, 0x8e, 0xf7, 0x6e, 0x83, 0x24,
                0xaf, 0xbf, 0xd4, 0x6c, 0xfd, 0x81, 0xb2, 0x2e, 0x39, 0x73, 0xc6, 0x5f, 0xa1, 0xbd,
                0x9d, 0xe3, 0x17, 0x87,
            ],
            "SHA3-256 200-byte 0xa3 vector mismatch"
        );
    }

    /// TV-8: domain-separated hash cascade is deterministic across two independent
    /// calls on the same input.  Guards against platform nondeterminism in SHA3-256.
    /// BLAKE3 and KangarooTwelve cascade stages are deferred to Domain B.
    #[test]
    fn cascade_determinism_same_input() {
        let seed = [0x5a_u8; 32];
        let step1_a = h_domain(DomainTag::EntropyAdvance, &seed);
        let step2_a = h_domain(DomainTag::InternalHash, &step1_a);

        let step1_b = h_domain(DomainTag::EntropyAdvance, &seed);
        let step2_b = h_domain(DomainTag::InternalHash, &step1_b);

        assert_eq!(step1_a, step1_b);
        assert_eq!(step2_a, step2_b);
        // Output must be non-zero (sanity check against identity hash bugs).
        assert_ne!(step2_a, [0u8; 32]);
    }
}
