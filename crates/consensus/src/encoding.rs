//! Canonical deterministic encoding (no hashing in this module).

use crate::fixed_point::{self, FixedPoint};

pub const ENCODING_VERSION: u32 = 0;

/// State header size (bytes).
/// Layout:
/// [version:u32][epoch:u64][validator_count:u32][halt_reason:u8][pad:3][entropy_seed:32]
pub const STATE_HEADER_SIZE: u32 = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    DecodeInvalid,    // H4
    RoundtripFailure, // H5
    BufferTooSmall,
    InvalidHaltCode,  // halt_reason byte has no known variant
    ValueOutOfRange,  // a FixedPoint value does not fit in the wire i64 range
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

/// 48-byte leaf index (384-bit) from (validator_id, epoch, seed).
pub fn compute_leaf_index(validator_id: u64, epoch: u64, epoch_seed: &[u8; 32]) -> [u8; 48] {
    let mut out = [0u8; 48];
    out[0..8].copy_from_slice(&validator_id.to_le_bytes());
    out[8..16].copy_from_slice(&epoch.to_le_bytes());
    out[16..48].copy_from_slice(epoch_seed);
    out
}

pub fn decode_leaf_index(bytes: &[u8; 48]) -> (u64, u64, [u8; 32]) {
    let mut a = [0u8; 8];
    let mut b = [0u8; 8];
    let mut seed = [0u8; 32];
    a.copy_from_slice(&bytes[0..8]);
    b.copy_from_slice(&bytes[8..16]);
    seed.copy_from_slice(&bytes[16..48]);
    (u64::from_le_bytes(a), u64::from_le_bytes(b), seed)
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

/// Decode as 3×i128 LE; enforce D,C ∈ [0, SCALE] and S ∈ [0, i64::MAX] (H4 on violation).
///
/// NOTE: This is the 48-byte i128 commitment encoding, distinct from the 24-byte i64
/// compact wire format used inside `decode_full_state`. The slash_accum upper bound
/// mirrors the `to_i64()` invariant enforced by `advance_epoch` (transition line ~459)
/// so that any decoded state is immediately usable in a transition without triggering
/// a halt on that check alone.
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
    // slash_accum must be non-negative and fit in i64 (wire and transition invariant).
    if s.raw() < 0 || s.raw() > i64::MAX as i128 {
        return Err(EncodeError::DecodeInvalid);
    }

    Ok((d, c, s))
}
