//! Epoch transition types for Pure QASH.
//!
//! Domain A rules: no floats, no usize in state fields, all arithmetic checked.

use crate::economics::EconomicsState;

/// Reason an epoch transition entered an absorbing halt.
///
/// Variant codes mirror the umbrella QASH protocol (H1–H8) to keep forensics
/// tools compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HaltReason {
    None               = 0x00,
    LyapunovViolation  = 0x01, // H1: state invariant failure
    ArithOverflow      = 0x02, // H2: checked arithmetic overflow
    EpochOverflow      = 0x03, // H3: epoch counter overflow
    DecodeInvalid      = 0x04, // H4: malformed wire input
    RoundtripFailure   = 0x05, // H5: encode/decode round-trip mismatch
    HaltFlagSet        = 0x06, // H6: explicit external halt
    PhiSafetyViolation = 0x07, // H7: Lyapunov safety bound exceeded
    IncompatibleVersion= 0x08, // H8: unsupported protocol version
}

impl HaltReason {
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Minimal epoch state for Pure QASH consensus.
///
/// Contains only the fields needed by the Domain A economics functions.
/// Pure QASH does not carry per-validator divergence metrics in this struct;
/// those are Domain B admission artifacts and must not flow into Domain A.
///
/// Encoding note: EconomicsState appends 64 bytes (4 × u128 LE) to any
/// wire format that includes this state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochState {
    /// Current epoch index (monotone-increasing, overflow → EpochOverflow halt).
    pub epoch: u64,
    /// Active halt reason; HaltReason::None means the epoch is live.
    pub halt_reason: HaltReason,
    /// Economics sub-state: supply, issuance, and burn accounting.
    ///
    /// Included in canonical state root. Conservation invariant:
    ///   total_supply = issued_total − burned_fees_total − burned_slashes_total
    pub economics: EconomicsState,
}

impl EpochState {
    pub const fn genesis() -> Self {
        Self {
            epoch:       0,
            halt_reason: HaltReason::None,
            economics:   EconomicsState::zero(),
        }
    }

    pub fn is_halted(&self) -> bool {
        self.halt_reason != HaltReason::None
    }
}

// ---------------------------------------------------------------------------
// Canonical encoding (for state root computation)
// ---------------------------------------------------------------------------

/// Byte length of the canonical EpochState encoding.
/// epoch(8) + halt_reason(1) + pad(7) + EconomicsState(64) = 80 bytes.
pub const EPOCH_STATE_WIRE_LEN: usize = 80;

impl EpochState {
    /// Encode to canonical wire format (deterministic, little-endian).
    pub fn encode_canonical(&self) -> [u8; EPOCH_STATE_WIRE_LEN] {
        let mut out = [0u8; EPOCH_STATE_WIRE_LEN];
        out[0..8].copy_from_slice(&self.epoch.to_le_bytes());
        out[8] = self.halt_reason.to_u8();
        // bytes 9-15: reserved / padding (zero)
        // EconomicsState at bytes 16-79 (4 × u128 LE = 64 bytes)
        let e = &self.economics;
        out[16..32].copy_from_slice(&e.total_supply.to_le_bytes());
        out[32..48].copy_from_slice(&e.issued_total.to_le_bytes());
        out[48..64].copy_from_slice(&e.burned_fees_total.to_le_bytes());
        out[64..80].copy_from_slice(&e.burned_slashes_total.to_le_bytes());
        out
    }

    /// Decode from canonical wire format. Returns None on length mismatch.
    pub fn decode_canonical(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != EPOCH_STATE_WIRE_LEN {
            return None;
        }
        let epoch = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let halt_reason = match bytes[8] {
            0x00 => HaltReason::None,
            0x01 => HaltReason::LyapunovViolation,
            0x02 => HaltReason::ArithOverflow,
            0x03 => HaltReason::EpochOverflow,
            0x04 => HaltReason::DecodeInvalid,
            0x05 => HaltReason::RoundtripFailure,
            0x06 => HaltReason::HaltFlagSet,
            0x07 => HaltReason::PhiSafetyViolation,
            0x08 => HaltReason::IncompatibleVersion,
            _ => return None,
        };
        let total_supply         = u128::from_le_bytes(bytes[16..32].try_into().ok()?);
        let issued_total         = u128::from_le_bytes(bytes[32..48].try_into().ok()?);
        let burned_fees_total    = u128::from_le_bytes(bytes[48..64].try_into().ok()?);
        let burned_slashes_total = u128::from_le_bytes(bytes[64..80].try_into().ok()?);
        Some(Self {
            epoch,
            halt_reason,
            economics: crate::economics::EconomicsState {
                total_supply,
                issued_total,
                burned_fees_total,
                burned_slashes_total,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_state_is_all_zero() {
        let g = EpochState::genesis();
        assert_eq!(g.epoch, 0);
        assert!(!g.is_halted());
        assert_eq!(g.economics.total_supply, 0);
    }

    #[test]
    fn epoch_state_encode_decode_roundtrip() {
        let s = EpochState {
            epoch: 42,
            halt_reason: HaltReason::None,
            economics: EconomicsState {
                total_supply:         1_000_000,
                issued_total:         1_001_000,
                burned_fees_total:       500,
                burned_slashes_total:    500,
            },
        };
        let encoded = s.encode_canonical();
        let decoded = EpochState::decode_canonical(&encoded).unwrap();
        assert_eq!(decoded.epoch, 42);
        assert_eq!(decoded.halt_reason, HaltReason::None);
        assert_eq!(decoded.economics.total_supply, 1_000_000);
        assert_eq!(decoded.economics.issued_total, 1_001_000);
        assert_eq!(decoded.economics.burned_fees_total, 500);
        assert_eq!(decoded.economics.burned_slashes_total, 500);
    }

    #[test]
    fn halt_reason_roundtrips() {
        let variants = [
            HaltReason::None,
            HaltReason::LyapunovViolation,
            HaltReason::ArithOverflow,
            HaltReason::DecodeInvalid,
        ];
        for v in variants {
            let s = EpochState { epoch: 0, halt_reason: v, economics: EconomicsState::zero() };
            let enc = s.encode_canonical();
            let dec = EpochState::decode_canonical(&enc).unwrap();
            assert_eq!(dec.halt_reason, v);
        }
    }
}
