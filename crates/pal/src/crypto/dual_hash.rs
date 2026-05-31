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
    let mut h = Sha3_512::new();
    h.update(SHA3_ARM_LABEL);
    h.update((context.len() as u32).to_le_bytes());
    h.update(context);
    h.update((salt.len() as u32).to_le_bytes());
    h.update(salt);
    h.update((data.len() as u64).to_le_bytes());
    h.update(data);
    h.finalize().into()
}

/// Streams context/salt/data into a BLAKE3 hasher with arm label and length frames,
/// then reads `N` bytes via XOF output.
fn blake3_arm<const N: usize>(context: &[u8], salt: &[u8], data: &[u8]) -> [u8; N] {
    let mut h = blake3::Hasher::new();
    h.update(BLAKE3_ARM_LABEL);
    h.update(&(context.len() as u32).to_le_bytes());
    h.update(context);
    h.update(&(salt.len() as u32).to_le_bytes());
    h.update(salt);
    h.update(&(data.len() as u64).to_le_bytes());
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
        // "ab"+"c" and "a"+"bc" collide when concatenated raw but differ when framed.
        let r1 = dual_hash_32(b"ab", b"c", DATA);
        let r2 = dual_hash_32(b"a", b"bc", DATA);
        assert_ne!(r1, r2);
        // Empty context is distinct from a context containing a zero byte.
        let r3 = dual_hash_32(b"", SALT, DATA);
        let r4 = dual_hash_32(b"\x00", SALT, DATA);
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
}
