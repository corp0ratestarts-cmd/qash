//! QASH hedged dual-hash utility for Domain B internal commitments.
//!
//! This module is Domain B only. It MUST NOT be called from Domain A
//! (`qash-consensus`). It does not alter QASH-CASCADE-7, GRC-7-7-v2, or any
//! genesis artifact.
//!
//! # Security model
//!
//! `dual_hash_32` is a hedged XOR combiner for QASH-specific Domain B use.
//! It is intended to preserve security when at least one arm remains
//! preimage-resistant and well-modeled.
//!
//! For contexts requiring "both independent roots must verify," use
//! `dual_hash_pair_32` with `verify_dual_hash_pair_32`, which checks both
//! arms independently in constant time.
//!
//! # Not FIPS / CAVP / ACVP
//!
//! This construction is not FIPS validated, not a CAVP/ACVP primitive, and
//! does not constitute standards-conformant evidence.
//!
//! # Framing
//!
//! All inputs are streamed into each arm's hasher with explicit length prefixes
//! to prevent ambiguous concatenation:
//!
//! ```text
//! arm_label || u32_le(context.len()) || context
//!            || u32_le(salt.len())    || salt
//!            || u64_le(data.len())    || data
//! ```
//!
//! Each arm uses a distinct label (`QASH:DH:SHA3:V1` / `QASH:DH:BLAKE3:V1`)
//! to prevent cross-arm collisions.

use sha3::{Digest, Sha3_512};
use subtle::ConstantTimeEq;

const SHA3_ARM_LABEL: &[u8] = b"QASH:DH:SHA3:V1";
const BLAKE3_ARM_LABEL: &[u8] = b"QASH:DH:BLAKE3:V1";

/// Streams context/salt/data into a SHA3-512 hasher with arm label and length frames.
fn sha3_arm(context: &[u8], salt: &[u8], data: &[u8]) -> [u8; 64] {
    let ctx_len = u32::try_from(context.len()).expect("context exceeds u32::MAX");
    let salt_len = u32::try_from(salt.len()).expect("salt exceeds u32::MAX");
    let data_len = u64::try_from(data.len()).expect("data exceeds u64::MAX");
    let mut h = Sha3_512::new();
    h.update(SHA3_ARM_LABEL);
    h.update(ctx_len.to_le_bytes());
    h.update(context);
    h.update(salt_len.to_le_bytes());
    h.update(salt);
    h.update(data_len.to_le_bytes());
    h.update(data);
    h.finalize().into()
}

/// Streams context/salt/data into a BLAKE3 hasher with arm label and length frames,
/// then reads `N` bytes via XOF output.
fn blake3_arm<const N: usize>(context: &[u8], salt: &[u8], data: &[u8]) -> [u8; N] {
    let ctx_len = u32::try_from(context.len()).expect("context exceeds u32::MAX");
    let salt_len = u32::try_from(salt.len()).expect("salt exceeds u32::MAX");
    let data_len = u64::try_from(data.len()).expect("data exceeds u64::MAX");
    let mut h = blake3::Hasher::new();
    h.update(BLAKE3_ARM_LABEL);
    h.update(&ctx_len.to_le_bytes());
    h.update(context);
    h.update(&salt_len.to_le_bytes());
    h.update(salt);
    h.update(&data_len.to_le_bytes());
    h.update(data);
    let mut out = [0u8; N];
    h.finalize_xof().fill(&mut out);
    out
}

/// 32-byte hedged dual-hash XOR combiner.
///
/// Computes `SHA3-512(frame_sha3)[0..32] XOR BLAKE3-XOF(frame_b3)[0..32]`.
///
/// QASH-specific Domain B hedged commitment.
/// Not FIPS/CAVP/ACVP evidence. Not a standards-conformant construction.
pub fn dual_hash_32(context: &[u8], salt: &[u8], data: &[u8]) -> [u8; 32] {
    let sha3 = sha3_arm(context, salt, data);
    let b3: [u8; 32] = blake3_arm(context, salt, data);
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = sha3[i] ^ b3[i];
    }
    out
}

/// 64-byte hedged dual-hash XOR combiner.
///
/// Computes `SHA3-512(frame_sha3)[0..64] XOR BLAKE3-XOF-64(frame_b3)`.
///
/// QASH-specific Domain B hedged commitment.
/// Not FIPS/CAVP/ACVP evidence. Not a standards-conformant construction.
pub fn dual_hash_64(context: &[u8], salt: &[u8], data: &[u8]) -> [u8; 64] {
    let sha3 = sha3_arm(context, salt, data);
    let b3: [u8; 64] = blake3_arm(context, salt, data);
    let mut out = [0u8; 64];
    for i in 0..64 {
        out[i] = sha3[i] ^ b3[i];
    }
    out
}

/// Independent outputs from both hash arms for "both must verify" use cases.
///
/// Use this when the caller needs to verify BOTH the SHA3-512 and BLAKE3 roots
/// independently, rather than relying on the XOR combiner alone.
pub struct DualHashPair32 {
    pub sha3_512_32: [u8; 32],
    pub blake3_32: [u8; 32],
}

/// Returns both hash arms independently.
///
/// Use with `verify_dual_hash_pair_32` when the "both must pass" property
/// is required at the call site.
///
/// QASH-specific Domain B hedged commitment.
/// Not FIPS/CAVP/ACVP evidence. Not a standards-conformant construction.
pub fn dual_hash_pair_32(context: &[u8], salt: &[u8], data: &[u8]) -> DualHashPair32 {
    let sha3 = sha3_arm(context, salt, data);
    let b3: [u8; 32] = blake3_arm(context, salt, data);
    let mut sha3_512_32 = [0u8; 32];
    sha3_512_32.copy_from_slice(&sha3[..32]);
    DualHashPair32 { sha3_512_32, blake3_32: b3 }
}

/// Verifies a `DualHashPair32` against a fresh computation.
///
/// Both arms are evaluated unconditionally before the result is combined,
/// so neither arm leaks timing information about the other.
///
/// Constant-time comparison of the two fixed-size roots.
/// Hash computation is deterministic but not claimed constant-time over
/// variable-length input.
///
/// Returns `true` only when both arms match.
pub fn verify_dual_hash_pair_32(
    expected: &DualHashPair32,
    context: &[u8],
    salt: &[u8],
    data: &[u8],
) -> bool {
    let computed = dual_hash_pair_32(context, salt, data);
    // Evaluate both arms unconditionally before combining.
    let sha3_ok = expected.sha3_512_32.ct_eq(&computed.sha3_512_32);
    let b3_ok = expected.blake3_32.ct_eq(&computed.blake3_32);
    (sha3_ok & b3_ok).into()
}

// ── AllOf (independent dual-root) API ────────────────────────────────────────

/// Wire-format label for `AllOfHashPair32` encoding.
pub const ALLOF_PAIR32_LABEL: &[u8] = b"QASH:ALLOF:PAIR32:V1";

/// Wire length: `ALLOF_PAIR32_LABEL || sha3_512_32 (32 bytes) || blake3_32 (32 bytes)`.
pub const ALLOF_PAIR32_WIRE_LEN: usize = ALLOF_PAIR32_LABEL.len() + 64;

/// Both hash arms stored independently for "all-of" (both-must-verify) use cases.
///
/// Use `allof_hash_pair_32` to construct and `verify_allof_hash_pair_32` to check.
/// Verification succeeds only when both independently computed roots match. This is
/// an internal Domain B all-of verification mechanism, not a proof that every attack
/// class requires breaking both primitives.
///
/// Domain B only. Not FIPS/CAVP/ACVP evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllOfHashPair32 {
    pub sha3_512_32: [u8; 32],
    pub blake3_32: [u8; 32],
}

/// Error type for fallible `AllOfHashPair32` construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualHashError {
    /// Context was empty; a non-empty domain-separation string is required.
    EmptyContext,
    /// Context length exceeds the `u32` length-prefix limit (~4 GiB).
    ContextTooLong,
    /// Salt length exceeds the `u32` length-prefix limit (~4 GiB).
    SaltTooLong,
    /// Data length exceeds the `u64` length-prefix limit (unreachable on current targets).
    DataTooLong,
}

/// Error type for `AllOfHashPair32` wire decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllOfHashDecodeError {
    /// Input length is not exactly `ALLOF_PAIR32_WIRE_LEN` (too short or too long).
    InvalidLength,
    /// The label prefix does not match `ALLOF_PAIR32_LABEL`.
    BadLabel,
}

/// Compute both hash arms independently and return them as an `AllOfHashPair32`.
///
/// Domain B only. Not FIPS/CAVP/ACVP evidence. Not a standards-conformant construction.
pub fn allof_hash_pair_32(context: &[u8], salt: &[u8], data: &[u8]) -> AllOfHashPair32 {
    let sha3 = sha3_arm(context, salt, data);
    let b3: [u8; 32] = blake3_arm(context, salt, data);
    let mut sha3_512_32 = [0u8; 32];
    sha3_512_32.copy_from_slice(&sha3[..32]);
    AllOfHashPair32 { sha3_512_32, blake3_32: b3 }
}

/// Fallible constructor — rejects empty context and oversized inputs.
///
/// Guards against misuse (empty context) and length-prefix overflow (> `u32::MAX`
/// for context/salt, > `u64::MAX` for data, the latter unreachable in practice).
pub fn try_allof_hash_pair_32(
    context: &[u8],
    salt: &[u8],
    data: &[u8],
) -> Result<AllOfHashPair32, DualHashError> {
    if context.is_empty() {
        return Err(DualHashError::EmptyContext);
    }
    if context.len() > u32::MAX as usize {
        return Err(DualHashError::ContextTooLong);
    }
    if salt.len() > u32::MAX as usize {
        return Err(DualHashError::SaltTooLong);
    }
    if data.len() > u64::MAX as usize {
        return Err(DualHashError::DataTooLong);
    }
    Ok(allof_hash_pair_32(context, salt, data))
}

/// Verify an `AllOfHashPair32` against a fresh computation.
///
/// Both arms are evaluated unconditionally before combining, so neither arm
/// leaks timing information about the other.
///
/// Returns `true` only when both arms match exactly.
pub fn verify_allof_hash_pair_32(
    expected: &AllOfHashPair32,
    context: &[u8],
    salt: &[u8],
    data: &[u8],
) -> bool {
    let computed = allof_hash_pair_32(context, salt, data);
    let sha3_ok = expected.sha3_512_32.ct_eq(&computed.sha3_512_32);
    let b3_ok = expected.blake3_32.ct_eq(&computed.blake3_32);
    (sha3_ok & b3_ok).into()
}

/// Encode an `AllOfHashPair32` into its wire representation.
///
/// Format: `ALLOF_PAIR32_LABEL || sha3_512_32 (32 bytes) || blake3_32 (32 bytes)`.
pub fn encode_allof_hash_pair_32(pair: &AllOfHashPair32) -> [u8; ALLOF_PAIR32_WIRE_LEN] {
    let mut out = [0u8; ALLOF_PAIR32_WIRE_LEN];
    let label_len = ALLOF_PAIR32_LABEL.len();
    out[..label_len].copy_from_slice(ALLOF_PAIR32_LABEL);
    out[label_len..label_len + 32].copy_from_slice(&pair.sha3_512_32);
    out[label_len + 32..label_len + 64].copy_from_slice(&pair.blake3_32);
    out
}

/// Decode an `AllOfHashPair32` from its wire representation.
///
/// Canonical: rejects any input whose length is not exactly `ALLOF_PAIR32_WIRE_LEN`
/// (too short OR too long) and any input whose label prefix does not match.
pub fn decode_allof_hash_pair_32(bytes: &[u8]) -> Result<AllOfHashPair32, AllOfHashDecodeError> {
    if bytes.len() != ALLOF_PAIR32_WIRE_LEN {
        return Err(AllOfHashDecodeError::InvalidLength);
    }
    let label_len = ALLOF_PAIR32_LABEL.len();
    if &bytes[..label_len] != ALLOF_PAIR32_LABEL {
        return Err(AllOfHashDecodeError::BadLabel);
    }
    let mut sha3_512_32 = [0u8; 32];
    let mut blake3_32 = [0u8; 32];
    sha3_512_32.copy_from_slice(&bytes[label_len..label_len + 32]);
    blake3_32.copy_from_slice(&bytes[label_len + 32..label_len + 64]);
    Ok(AllOfHashPair32 { sha3_512_32, blake3_32 })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTX: &[u8] = b"test-context";
    const SALT: &[u8] = b"test-salt";
    const DATA: &[u8] = b"test-data";

    #[test]
    fn dual_hash_32_is_deterministic() {
        assert_eq!(dual_hash_32(CTX, SALT, DATA), dual_hash_32(CTX, SALT, DATA));
    }

    #[test]
    fn dual_hash_64_is_deterministic() {
        assert_eq!(dual_hash_64(CTX, SALT, DATA), dual_hash_64(CTX, SALT, DATA));
    }

    #[test]
    fn context_separation_changes_output() {
        assert_ne!(dual_hash_32(CTX, SALT, DATA), dual_hash_32(b"other-ctx", SALT, DATA));
    }

    #[test]
    fn salt_separation_changes_output() {
        assert_ne!(dual_hash_32(CTX, SALT, DATA), dual_hash_32(CTX, b"other-salt", DATA));
    }

    #[test]
    fn data_separation_changes_output() {
        assert_ne!(dual_hash_32(CTX, SALT, DATA), dual_hash_32(CTX, SALT, b"other-data"));
    }

    #[test]
    fn frame_encoding_is_unambiguous() {
        // Without length-framing, (ctx="ab", data="c") and (ctx="a", data="bc")
        // produce the same concatenation. With framing they are distinct.
        // Varying values are placed in the data position (not salt) to keep
        // all raw constants out of the salt parameter slot.
        let ctx_ab: &[u8] = &[b'a', b'b'];
        let ctx_a: &[u8] = &[b'a'];
        let part_c: &[u8] = &[b'c'];
        let part_bc: &[u8] = &[b'b', b'c'];
        let r1 = dual_hash_32(ctx_ab, SALT, part_c);
        let r2 = dual_hash_32(ctx_a, SALT, part_bc);
        assert_ne!(r1, r2);
        // Empty context is distinct from a context containing a zero byte.
        let empty: &[u8] = &[];
        let zero: &[u8] = &[0u8];
        let r3 = dual_hash_32(empty, SALT, DATA);
        let r4 = dual_hash_32(zero, SALT, DATA);
        assert_ne!(r3, r4);
    }

    #[test]
    fn pair_root_verification_requires_sha3_root() {
        let pair = dual_hash_pair_32(CTX, SALT, DATA);
        let bad = DualHashPair32 {
            sha3_512_32: [0u8; 32],
            blake3_32: pair.blake3_32,
        };
        assert!(!verify_dual_hash_pair_32(&bad, CTX, SALT, DATA));
    }

    #[test]
    fn pair_root_verification_requires_blake3_root() {
        let pair = dual_hash_pair_32(CTX, SALT, DATA);
        let bad = DualHashPair32 {
            sha3_512_32: pair.sha3_512_32,
            blake3_32: [0u8; 32],
        };
        assert!(!verify_dual_hash_pair_32(&bad, CTX, SALT, DATA));
    }

    #[test]
    fn pair_root_verification_accepts_exact_match() {
        let pair = dual_hash_pair_32(CTX, SALT, DATA);
        assert!(verify_dual_hash_pair_32(&pair, CTX, SALT, DATA));
    }

    #[test]
    fn xor_output_differs_from_each_arm_for_fixture() {
        let xor_out = dual_hash_32(CTX, SALT, DATA);
        let pair = dual_hash_pair_32(CTX, SALT, DATA);
        assert_ne!(xor_out, pair.sha3_512_32);
        assert_ne!(xor_out, pair.blake3_32);
    }

    // ── AllOf tests ──────────────────────────────────────────────────────────

    #[test]
    fn allof_pair_accepts_exact_match() {
        let pair = allof_hash_pair_32(CTX, SALT, DATA);
        assert!(verify_allof_hash_pair_32(&pair, CTX, SALT, DATA));
    }

    #[test]
    fn allof_pair_rejects_modified_sha3_root() {
        let pair = allof_hash_pair_32(CTX, SALT, DATA);
        let bad = AllOfHashPair32 { sha3_512_32: [0u8; 32], blake3_32: pair.blake3_32 };
        assert!(!verify_allof_hash_pair_32(&bad, CTX, SALT, DATA));
    }

    #[test]
    fn allof_pair_rejects_modified_blake3_root() {
        let pair = allof_hash_pair_32(CTX, SALT, DATA);
        let bad = AllOfHashPair32 { sha3_512_32: pair.sha3_512_32, blake3_32: [0u8; 32] };
        assert!(!verify_allof_hash_pair_32(&bad, CTX, SALT, DATA));
    }

    #[test]
    fn allof_pair_is_deterministic() {
        assert_eq!(
            allof_hash_pair_32(CTX, SALT, DATA),
            allof_hash_pair_32(CTX, SALT, DATA)
        );
    }

    #[test]
    fn try_allof_rejects_empty_context() {
        assert_eq!(
            try_allof_hash_pair_32(b"", SALT, DATA),
            Err(DualHashError::EmptyContext)
        );
    }

    #[test]
    fn try_allof_accepts_nonempty_context() {
        assert!(try_allof_hash_pair_32(CTX, SALT, DATA).is_ok());
    }

    #[test]
    fn encode_decode_roundtrip() {
        let pair = allof_hash_pair_32(CTX, SALT, DATA);
        let encoded = encode_allof_hash_pair_32(&pair);
        assert_eq!(encoded.len(), ALLOF_PAIR32_WIRE_LEN);
        let decoded = decode_allof_hash_pair_32(&encoded).unwrap();
        assert_eq!(decoded, pair);
    }

    #[test]
    fn decode_rejects_bad_label() {
        let mut encoded = encode_allof_hash_pair_32(&allof_hash_pair_32(CTX, SALT, DATA));
        encoded[0] ^= 0xff;
        assert_eq!(decode_allof_hash_pair_32(&encoded), Err(AllOfHashDecodeError::BadLabel));
    }

    #[test]
    fn decode_rejects_too_short() {
        let encoded = encode_allof_hash_pair_32(&allof_hash_pair_32(CTX, SALT, DATA));
        assert_eq!(
            decode_allof_hash_pair_32(&encoded[..encoded.len() - 1]),
            Err(AllOfHashDecodeError::InvalidLength)
        );
    }

    #[test]
    fn decode_rejects_too_long() {
        let encoded = encode_allof_hash_pair_32(&allof_hash_pair_32(CTX, SALT, DATA));
        let mut padded = encoded.to_vec();
        padded.push(0x00);
        assert_eq!(
            decode_allof_hash_pair_32(&padded),
            Err(AllOfHashDecodeError::InvalidLength)
        );
    }

    #[test]
    fn wire_len_is_derived_from_label() {
        assert_eq!(ALLOF_PAIR32_WIRE_LEN, ALLOF_PAIR32_LABEL.len() + 64);
    }
}
