//! Canonical deterministic encoding (no hashing in this module).
//!
//! The commitment preimage format (used for prior-root-bound state root computation,
//! ADR-003 accepted) is documented below and must remain frozen after genesis lock.

use crate::fixed_point::{self, FixedPoint};

pub const ENCODING_VERSION: u32 = 0;

/// State header size (bytes).
/// Layout:
/// [version:u32][epoch:u64][validator_count:u32][halt_reason:u8][pad:3][entropy_seed:32]
pub const STATE_HEADER_SIZE: u32 = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    DecodeInvalid,   // H4
    RoundtripFailure, // H5
    BufferTooSmall,
    InvalidHaltCode, // halt_reason byte has no known variant
}

pub fn encode_state_header(
    epoch: u64,
    validator_count: u32,
    halt_reason: u8,
    entropy_seed: &[u8; 32],
    out: &mut [u8; STATE_HEADER_SIZE as usize],
) {
    out[0..4].copy_from_slice(&ENCODING_VERSION.to_le_bytes());
    out[4..12].copy_from_slice(&epoch.to_le_bytes());
    out[12..16].copy_from_slice(&validator_count.to_le_bytes());
    out[16] = halt_reason;
    out[17] = 0x00;
    out[18] = 0x00;
    out[19] = 0x00;
    out[20..52].copy_from_slice(entropy_seed);
}

pub fn decode_state_header(
    bytes: &[u8; STATE_HEADER_SIZE as usize],
) -> Result<(u64, u32, u8, [u8; 32]), EncodeError> {
    let mut ver_bytes = [0u8; 4];
    ver_bytes.copy_from_slice(&bytes[0..4]);
    let version = u32::from_le_bytes(ver_bytes);
    if version != ENCODING_VERSION {
        return Err(EncodeError::DecodeInvalid);
    }

    // canonical padding
    if bytes[17] != 0x00 || bytes[18] != 0x00 || bytes[19] != 0x00 {
        return Err(EncodeError::DecodeInvalid);
    }

    let mut epoch_bytes = [0u8; 8];
    epoch_bytes.copy_from_slice(&bytes[4..12]);

    let mut vc_bytes = [0u8; 4];
    vc_bytes.copy_from_slice(&bytes[12..16]);

    let halt_reason = bytes[16];

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes[20..52]);

    Ok((
        u64::from_le_bytes(epoch_bytes),
        u32::from_le_bytes(vc_bytes),
        halt_reason,
        seed,
    ))
}

/// 48-byte cascade-derived leaf index (spec v1.1 §3.2).
///
/// Computes `h_cascade(validator_id_le8 ‖ epoch_le8 ‖ epoch_seed)[0..48]`.
/// The 64-byte cascade output is truncated to 48 bytes (384 bits), which is
/// the sparse Merkle tree leaf width defined in GENESIS_CONSTANTS.toml
/// [obfuscation] leaf_index_bytes = 48.
pub fn compute_cascade_leaf_index(
    validator_id: u64,
    epoch: u64,
    epoch_seed: &[u8; 32],
) -> [u8; 48] {
    let mut input = [0u8; 48]; // 8 + 8 + 32
    input[0..8].copy_from_slice(&validator_id.to_le_bytes());
    input[8..16].copy_from_slice(&epoch.to_le_bytes());
    input[16..48].copy_from_slice(epoch_seed);

    let cascade_out = crate::cascade::h_cascade(&input);

    let mut leaf = [0u8; 48];
    leaf.copy_from_slice(&cascade_out[0..48]);
    leaf
}

pub const VALIDATOR_DYNAMIC_SIZE: u32 = 48;

/// Encode (divergence, conflict, slash_accum) as 3×i128 LE (16 bytes each).
pub fn encode_validator_dynamic(
    divergence: FixedPoint,
    conflict: FixedPoint,
    slash_accum: FixedPoint,
    out: &mut [u8; VALIDATOR_DYNAMIC_SIZE as usize],
) {
    out[0..16].copy_from_slice(&fixed_point::encode_fixed_point(divergence));
    out[16..32].copy_from_slice(&fixed_point::encode_fixed_point(conflict));
    out[32..48].copy_from_slice(&fixed_point::encode_fixed_point(slash_accum));
}

// ─── Commitment preimage (ADR-003 accepted) ───────────────────────────────

/// Max validators constant (must match transition::MAX_VALIDATORS_WIRE = 1024).
const MAX_VALIDATORS_PREIMAGE: usize = 1024;

/// Commitment preimage layout (all integers little-endian):
///
/// [version: u32][epoch: u64][prior_root: 32 B][ledger_root: 32 B]
/// [entropy_seed: 32 B][halt_flag: u8][pad: 3 B][validator_count: u32]
/// [for each of validator_count validators: div(i64) + con(i64) + slash(i64)]
/// [window_filled: u8][w0: i64][w1: i64][w2: i64]
///
/// = 116 + 24·N + 25 bytes  (max with N=1024: 24717 bytes)
///
/// NOTE: spec §2 wire format additionally includes per-validator id[48]/score/active
/// which are not yet tracked; those fields will be added before genesis lock.
pub const MAX_COMMITMENT_PREIMAGE: usize = 116 + 24 * MAX_VALIDATORS_PREIMAGE + 25;

/// Writes the commitment preimage for `H_domain(STATE_ROOT, ...)` into `out`.
/// Returns the number of bytes written (varies with validator_count).
///
/// Preconditions (caller-enforced, checked by step_1_validate):
///   - validators_d/c/s slices have length == validator_count
///   - all values are within i64 range (slash already validated by step_1_validate)
#[allow(clippy::too_many_arguments)]
pub fn encode_commitment_preimage(
    epoch: u64,
    prior_root: &[u8; 32],
    ledger_root: &[u8; 32],
    entropy_seed: &[u8; 32],
    halt_flag: u8,
    validator_count: u32,
    validators_d: &[i64],
    validators_c: &[i64],
    validators_s: &[i64],
    window_filled: u8,
    window_v0: i64,
    window_v1: i64,
    window_v2: i64,
    out: &mut [u8; MAX_COMMITMENT_PREIMAGE],
) -> u32 {
    let mut o: usize = 0;

    out[o..o+4].copy_from_slice(&ENCODING_VERSION.to_le_bytes()); o += 4;
    out[o..o+8].copy_from_slice(&epoch.to_le_bytes()); o += 8;
    out[o..o+32].copy_from_slice(prior_root); o += 32;
    out[o..o+32].copy_from_slice(ledger_root); o += 32;
    out[o..o+32].copy_from_slice(entropy_seed); o += 32;
    out[o] = halt_flag; o += 1;
    out[o] = 0x00; out[o+1] = 0x00; out[o+2] = 0x00; o += 3;
    out[o..o+4].copy_from_slice(&validator_count.to_le_bytes()); o += 4;

    let vc = validator_count as usize;
    let mut i = 0usize;
    while i < vc {
        out[o..o+8].copy_from_slice(&validators_d[i].to_le_bytes()); o += 8;
        out[o..o+8].copy_from_slice(&validators_c[i].to_le_bytes()); o += 8;
        out[o..o+8].copy_from_slice(&validators_s[i].to_le_bytes()); o += 8;
        i += 1;
    }

    out[o] = window_filled; o += 1;
    out[o..o+8].copy_from_slice(&window_v0.to_le_bytes()); o += 8;
    out[o..o+8].copy_from_slice(&window_v1.to_le_bytes()); o += 8;
    out[o..o+8].copy_from_slice(&window_v2.to_le_bytes()); o += 8;

    o as u32
}

/// Decode as 3×i128 LE; enforce D,C bounds in [0, SCALE] (H4 on violation).
pub fn decode_validator_dynamic(
    bytes: &[u8; VALIDATOR_DYNAMIC_SIZE as usize],
) -> Result<(FixedPoint, FixedPoint, FixedPoint), EncodeError> {
    let mut d_bytes = [0u8; 16];
    let mut c_bytes = [0u8; 16];
    let mut s_bytes = [0u8; 16];
    d_bytes.copy_from_slice(&bytes[0..16]);
    c_bytes.copy_from_slice(&bytes[16..32]);
    s_bytes.copy_from_slice(&bytes[32..48]);

    let d = fixed_point::decode_fixed_point(d_bytes);
    let c = fixed_point::decode_fixed_point(c_bytes);
    let s = fixed_point::decode_fixed_point(s_bytes);

    let scale = fixed_point::SCALE;
    if d.raw() < 0 || d.raw() > scale || c.raw() < 0 || c.raw() > scale {
        return Err(EncodeError::DecodeInvalid);
    }

    Ok((d, c, s))
}
