// Blinding framework (spec v1.1.1, §3.7).
//
// Purpose: SIDE-CHANNEL RESISTANCE — not transaction privacy.
// QASH is transparent-by-design; blinding prevents:
//   - Key extraction via power/timing analysis
//   - Fault injection attacks on signing/hashing
//   - Memory access pattern leakage (partial — ORAM deferred to v1.2)
//   - Cross-validator timing correlation
//
// Two `consensus_critical=true` operations (§3.7.2 blinding registry, Option 2):
//   1. cascade_hash_input  — additive masking via H_cascade_keyed(epoch_key, input)
//   2. dilithium_sign      — multiplicative blinding via per-signing scalar derivation
//   3. chunk_key_derivation — 2-share secret sharing (included in Option 2 scope)
//
// ORAM patterns and dummy-op insertion are deferred to v1.2 per the performance table.
// The remaining ~5% exposure (program flow, non-secret memory addresses) is accepted
// risk because it reveals metadata only, not key material — see §3.7.5 analysis.
//
// Domain B caller obligations (ref: "Hardening QASH Domain B on Consumer Hardware"):
//   Directive 1 — ML-DSA NTT must use Code-Based Masking (CBM) on linear complementary
//     codes plus 2-redundant bitsliced NTT on ARM Cortex-M / RISC-V targets.
//   Directive 2 — ML-DSA must run in hedged mode; the caller must perform self-check
//     verification (Az - c·t₁·2^d) before broadcasting any signature; on failure the
//     caller must trigger absorbing halt and zeroize all registers.
//   Protection of the blinding epoch_key itself (storage, derivation, zeroization) is
//   the Domain B caller's responsibility; this module only consumes the key.
//
// Non-interference theorem (§3.7.5):
//   For any blinded operation with valid blinding_params:
//     Observations(exec(secret₁)) ≈ Observations(exec(secret₂))
//   i.e. even with full side-channel visibility, outputs are computationally
//   indistinguishable for different secret inputs.
// Non-interference proved: proofs/blinding/blinding_non_interference.v (AX-3 + cascade_prf_security)
//
// v1.1.1 Lyapunov rebalancing (activated when WEIGHT_BH goes non-zero):
//   D: 350_000 → 320_000
//   C: 300_000 → 280_000
//   Σ: 200_000 → 180_000
//   CH: 150_000 → 130_000
//   BH (new): 0 → 90_000   (sum still 1_000_000)
//
// Key derivation (Domain B) lives in src/crypto/blinding.rs.
// This module is Domain A: all operations are deterministic given the blinding key.

use crate::cascade::{h_cascade, h_cascade_keyed};

/// Blinding mode selector (part of on-chain state type once blinding activates).
///
/// Until WEIGHT_BH is set to 90_000, `None` is the operative mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlindingMode {
    /// No blinding — pre-v1.1.1 behaviour. Identity for all operations.
    #[default]
    None,
    /// Epoch-bound PRF: H_cascade_keyed(epoch_key, input).
    /// epoch_key derived via h_cascade_derive(epoch_root, epoch, entropy_seed).
    EpochBoundPRF,
}

// ---------------------------------------------------------------------------
// Operation 1: Additive cascade-input masking (§3.7.2: cascade_hash_input)
// ---------------------------------------------------------------------------

/// Additive cascade blinding (spec v1.1.1 §3.7, §5).
///
/// - `BlindingMode::None`:          returns `h_cascade(input)` (unmasked)
/// - `BlindingMode::EpochBoundPRF`: returns `h_cascade_keyed(epoch_key, input)`
///
/// The epoch_key acts as a per-epoch additive mask at the L2 binding layer,
/// making the cascade output computationally indistinguishable across epochs
/// even when the input is identical.  `epoch_key` may be `&[]` when mode is None.
pub fn blind_cascade_input(mode: BlindingMode, epoch_key: &[u8], input: &[u8]) -> [u8; 64] {
    match mode {
        BlindingMode::None => h_cascade(input),
        BlindingMode::EpochBoundPRF => h_cascade_keyed(epoch_key, input),
    }
}

// ---------------------------------------------------------------------------
// Operation 2: Multiplicative Dilithium blinding (§3.7.2: dilithium_sign)
// ---------------------------------------------------------------------------

/// Derive the per-signing blinding scalar for Dilithium (spec v1.1.1 §6).
///
/// The caller incorporates `blinding_scalar` into the Dilithium signing call:
///   NTT-domain: msg_blinded = msg ⊗ blinding_scalar (mod q)
///   OR additive: msg_blinded = msg ⊕ blinding_scalar
///
/// Returns `[u8; 32]` — low 32 bytes of H_cascade_keyed(epoch_key, nonce).
/// Mode::None returns an all-zero scalar (identity for XOR, not for NTT-mul).
///
/// Note: `signing_nonce` MUST be unique per signing operation to prevent
/// reuse-based attacks. In practice, derive it from the message hash + epoch.
pub fn derive_dilithium_blinding_scalar(
    mode: BlindingMode,
    epoch_key: &[u8],
    signing_nonce: &[u8; 32],
) -> [u8; 32] {
    match mode {
        BlindingMode::None => [0u8; 32],
        BlindingMode::EpochBoundPRF => {
            let out = h_cascade_keyed(epoch_key, signing_nonce);
            let mut scalar = [0u8; 32];
            scalar.copy_from_slice(&out[0..32]);
            scalar
        }
    }
}

// ---------------------------------------------------------------------------
// Operation 3: 2-share secret sharing for chunk key derivation (§3.7.2)
// ---------------------------------------------------------------------------

/// Split `key` into two XOR shares using epoch-bound PRF material.
///
/// share_0 = H_cascade_keyed(epoch_key, nonce)[0..32]
/// share_1 = key ⊕ share_0
///
/// Reconstruction: share_0 ⊕ share_1 == key
///
/// Both shares must be zeroized after use (caller responsibility).
/// Mode::None returns (key_copy, zeros) — i.e. share_0 = key, share_1 = 0.
pub fn split_chunk_key(
    mode: BlindingMode,
    epoch_key: &[u8],
    key: &[u8; 32],
    nonce: &[u8; 32],
) -> ([u8; 32], [u8; 32]) {
    match mode {
        BlindingMode::None => (*key, [0u8; 32]),
        BlindingMode::EpochBoundPRF => {
            let prf_out = h_cascade_keyed(epoch_key, nonce);
            let mut share0 = [0u8; 32];
            share0.copy_from_slice(&prf_out[0..32]);
            let mut share1 = [0u8; 32];
            for i in 0..32 {
                share1[i] = key[i] ^ share0[i];
            }
            (share0, share1)
        }
    }
}

/// Reconstruct the key from two XOR shares.
pub fn reconstruct_chunk_key(share0: &[u8; 32], share1: &[u8; 32]) -> [u8; 32] {
    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = share0[i] ^ share1[i];
    }
    key
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- cascade blinding ---

    #[test]
    fn none_mode_equals_unkeyed() {
        let input = b"blinding framework test";
        assert_eq!(
            blind_cascade_input(BlindingMode::None, &[], input),
            h_cascade(input)
        );
    }

    #[test]
    fn epoch_prf_uses_key() {
        let input = b"epoch bound test";
        let key = b"epoch_key_64bytes_padding_here!_32_bytes_of_key_material_00000";
        assert_ne!(
            blind_cascade_input(BlindingMode::EpochBoundPRF, key, input),
            blind_cascade_input(BlindingMode::None, &[], input),
            "blinded output must differ from unblinded"
        );
    }

    #[test]
    fn epoch_prf_is_deterministic() {
        let (input, key) = (b"determinism", b"key");
        assert_eq!(
            blind_cascade_input(BlindingMode::EpochBoundPRF, key, input),
            blind_cascade_input(BlindingMode::EpochBoundPRF, key, input),
        );
    }

    // --- dilithium blinding ---

    #[test]
    fn dilithium_scalar_none_is_zeroes() {
        let nonce = [0x01u8; 32];
        assert_eq!(
            derive_dilithium_blinding_scalar(BlindingMode::None, &[], &nonce),
            [0u8; 32]
        );
    }

    #[test]
    fn dilithium_scalar_prf_nonzero_deterministic() {
        let nonce = [0x42u8; 32];
        let key = b"epoch_key";
        let s1 = derive_dilithium_blinding_scalar(BlindingMode::EpochBoundPRF, key, &nonce);
        let s2 = derive_dilithium_blinding_scalar(BlindingMode::EpochBoundPRF, key, &nonce);
        assert_ne!(s1, [0u8; 32], "PRF scalar must be non-zero");
        assert_eq!(s1, s2, "PRF scalar must be deterministic");
    }

    #[test]
    fn different_epoch_keys_different_scalars() {
        let nonce = [0x01u8; 32];
        assert_ne!(
            derive_dilithium_blinding_scalar(BlindingMode::EpochBoundPRF, b"epoch0", &nonce),
            derive_dilithium_blinding_scalar(BlindingMode::EpochBoundPRF, b"epoch1", &nonce),
        );
    }

    // --- chunk key splitting ---

    #[test]
    fn split_and_reconstruct_roundtrip() {
        let key = [0xABu8; 32];
        let nonce = [0x01u8; 32];
        let epoch_key = b"epoch_key_material_here";
        let (s0, s1) = split_chunk_key(BlindingMode::EpochBoundPRF, epoch_key, &key, &nonce);
        assert_eq!(reconstruct_chunk_key(&s0, &s1), key);
    }

    #[test]
    fn split_none_mode_share0_is_key() {
        let key = [0x77u8; 32];
        let nonce = [0u8; 32];
        let (s0, s1) = split_chunk_key(BlindingMode::None, &[], &key, &nonce);
        assert_eq!(s0, key);
        assert_eq!(s1, [0u8; 32]);
    }

    #[test]
    fn prf_shares_differ_from_key() {
        let key = [0xFFu8; 32];
        let nonce = [0x01u8; 32];
        let (s0, s1) = split_chunk_key(BlindingMode::EpochBoundPRF, b"k", &key, &nonce);
        // shares should differ from raw key
        assert_ne!(s0, key);
        assert_ne!(s1, [0u8; 32]);
        // but reconstruct correctly
        assert_eq!(reconstruct_chunk_key(&s0, &s1), key);
    }
}
