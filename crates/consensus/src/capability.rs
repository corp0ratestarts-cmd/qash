//! Domain-crossing capability tokens and validation.
//!
//! # Purpose
//!
//! Domain A (`qash-consensus`) is fully deterministic and may never receive
//! non-deterministic values directly from Domain B (PAL / OS / network). Every
//! approved crossing point must be wrapped in a [`CapToken<T>`] and annotated
//! with a [`Capability`] variant. This module defines:
//!
//! - [`Capability`] — enumeration of approved Domain B → Domain A crossing types.
//! - [`validate_capability`] — checks that a raw capability code maps to a known
//!   approved variant (guards against bit-flip / serialisation errors).
//!
//! # Relationship to `CapToken<T>`
//!
//! [`CapToken<T>`][crate::domain::CapToken] wraps the *value* crossing the boundary.
//! [`Capability`] documents the *reason* for the crossing. In the PAL layer:
//!
//! ```text
//! // Domain B:
//! let cap  = Capability::EntropyIngress;
//! let token: CapToken<[u8; 32]> = CapToken::new(os_entropy());
//! domain_a_admission(cap, token);
//!
//! // Domain A admission point:
//! pub fn domain_a_admission(cap: Capability, token: CapToken<[u8; 32]>) {
//!     validate_capability(cap as u8).expect("unknown capability code");
//!     let raw = token.into_inner();
//!     // ... safe to use raw inside Domain A
//! }
//! ```

/// Approved categories of Domain B → Domain A boundary crossings.
///
/// Each variant represents one type of external input that a PAL admission
/// function is allowed to forward into Domain A consensus logic. The code
/// value (`u8`) is stable across versions; do not renumber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Capability {
    /// OS-sourced entropy for genesis seed initialisation.
    ///
    /// Only permitted at chain genesis; never permitted mid-epoch.
    /// The caller must certify this is the first and only entropy ingress
    /// for this chain instance.
    EntropyIngress = 0x01,

    /// Wall-clock epoch scheduling signal from Domain B scheduler.
    ///
    /// The timestamp must be sanitised to a `u64` epoch index (monotone,
    /// within `[genesis_epoch, genesis_epoch + MAX_EPOCHS]`) before wrapping.
    EpochSchedule = 0x02,

    /// Raw envelope bytes arriving from the network layer.
    ///
    /// Must pass `validate_envelope_epoch` before the inner bytes are
    /// forwarded to `advance_epoch`. The PAL is responsible for the
    /// admission check; Domain A trusts the pre-validated bytes.
    NetworkEnvelope = 0x03,

    /// External halt directive from Domain B governance (emergency stop).
    ///
    /// Sets `HaltReason::HaltFlagSet` on the active epoch state.
    /// Irreversible; no transition is possible after this capability fires.
    ExternalHalt = 0x04,
}

/// Validate that a raw code byte maps to a known [`Capability`] variant.
///
/// Returns `Ok(cap)` for any approved capability code, `Err(CapabilityError::UnknownCode)` for an
/// unknown byte. Used at PAL admission points to guard against serialisation
/// errors or unexpected capability extensions.
pub fn validate_capability(raw_code: u8) -> Result<Capability, CapabilityError> {
    match raw_code {
        0x01 => Ok(Capability::EntropyIngress),
        0x02 => Ok(Capability::EpochSchedule),
        0x03 => Ok(Capability::NetworkEnvelope),
        0x04 => Ok(Capability::ExternalHalt),
        _ => Err(CapabilityError::UnknownCode),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityError {
    UnknownCode,
}

// ---------------------------------------------------------------------------
// Effect-Capability Token (Stage 6a / 1-A)
// ---------------------------------------------------------------------------

/// A validated effect bundle ready to cross the Domain B → Domain A boundary.
///
/// `ValidatedEffect` is a type alias for [`crate::transition::EpochInput`].
/// Using the name `ValidatedEffect` at the `EffectToken` boundary documents that
/// Domain B has completed all pre-validation checks (signature verification,
/// nonce anti-replay, admission gate) before wrapping a reference to the input.
///
/// `advance_epoch` accepts `EffectToken<&ValidatedEffect>` — the token wraps a
/// reference (8 bytes), not an owned copy. This avoids a ~64 KB stack copy of the
/// `updates` array on every epoch transition. The token is still move-only, which
/// proves Domain B holds the pre-validation obligation and cannot accidentally
/// double-unwrap the same crossing-point value.
///
/// Proof obligation: `proofs/capability/cap_token_schema.v` (CT-1 through CT-3).
pub type ValidatedEffect = crate::transition::EpochInput;

/// A Domain B → Domain A capability token wrapping a value of type `T`.
///
/// Wrapping a value in `EffectToken<T>` asserts that Domain B has completed
/// all pre-validation obligations for that value before forwarding it into
/// Domain A. Domain A functions that accept `EffectToken<T>` must never
/// short-circuit the unwrap — the unwrap is the audit point.
///
/// # Invariants (enforced by construction)
///
/// 1. An `EffectToken` can only be constructed via `EffectToken::new` —
///    there is no `Default`, no `Copy`, and no `Clone` implementation.
/// 2. The inner value is consumed on first `.into_inner()` call (move semantics
///    prevent double-unwrap at the boundary).
pub struct EffectToken<T>(T);

impl<T> EffectToken<T> {
    /// Wrap a Domain B value, asserting all pre-validation obligations are met.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Consume the token and extract the inner value for Domain A use.
    ///
    /// This call is the sole admitted crossing point for the wrapped value.
    /// Domain A must not store the result beyond the current transition call.
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_roundtrip() {
        let all = [
            Capability::EntropyIngress,
            Capability::EpochSchedule,
            Capability::NetworkEnvelope,
            Capability::ExternalHalt,
        ];
        for cap in &all {
            let code = *cap as u8;
            assert_eq!(validate_capability(code), Ok(*cap));
        }
    }

    #[test]
    fn unknown_codes_rejected() {
        for code in [0x00u8, 0x05, 0xFF] {
            assert_eq!(validate_capability(code), Err(CapabilityError::UnknownCode));
        }
    }
}
