//! Capability tokens for Domain B -> Domain A boundary hardening.
//!
//! A `CapToken<T>` is an authenticated wrapper produced only by validators at
//! the PAL boundary. Domain A-facing adapters can require tokenized inputs
//! instead of raw host data.

use qash_consensus::CASCADE_DEPTH;

/// Opaque capability wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapToken<T>(T);

impl<T> CapToken<T> {
    /// Consume and reveal the validated payload.
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Borrow the validated payload.
    pub fn as_ref(&self) -> &T {
        &self.0
    }
}

/// Validated envelope-admission effect schema carried across the PAL boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedEffect {
    pub epoch: u64,
    pub validator_id: u32,
    pub cascade_health: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapTokenParams {
    pub max_validators: u32,
    pub cascade_depth: u32,
    pub max_payload_bytes: usize,
}

impl Default for CapTokenParams {
    fn default() -> Self {
        Self {
            max_validators: 1024,
            cascade_depth: CASCADE_DEPTH,
            max_payload_bytes: 4096,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapTokenError {
    EpochZero,
    ValidatorOutOfBounds,
    CascadeHealthOutOfRange,
    PayloadTooLarge,
}

/// Validate a raw Domain-B effect and seal it into a capability token.
pub fn validate_effect_token(
    params: &CapTokenParams,
    raw_epoch: u64,
    raw_validator_id: u32,
    raw_cascade_health: u32,
    raw_payload: &[u8],
) -> Result<CapToken<ValidatedEffect>, CapTokenError> {
    if raw_epoch == 0 {
        return Err(CapTokenError::EpochZero);
    }
    if raw_validator_id >= params.max_validators {
        return Err(CapTokenError::ValidatorOutOfBounds);
    }
    if raw_cascade_health > params.cascade_depth {
        return Err(CapTokenError::CascadeHealthOutOfRange);
    }
    if raw_payload.len() > params.max_payload_bytes {
        return Err(CapTokenError::PayloadTooLarge);
    }

    Ok(CapToken(ValidatedEffect {
        epoch: raw_epoch,
        validator_id: raw_validator_id,
        cascade_health: raw_cascade_health,
        payload: raw_payload.to_vec(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_seals_effect() {
        let params = CapTokenParams::default();
        let tok = validate_effect_token(&params, 1, 7, 3, b"abc").expect("must validate");
        let eff = tok.into_inner();
        assert_eq!(eff.epoch, 1);
        assert_eq!(eff.validator_id, 7);
        assert_eq!(eff.cascade_health, 3);
        assert_eq!(eff.payload, b"abc");
    }

    #[test]
    fn rejects_out_of_range_fields() {
        let params = CapTokenParams::default();
        assert_eq!(
            validate_effect_token(&params, 0, 0, 0, b"x"),
            Err(CapTokenError::EpochZero)
        );
        assert_eq!(
            validate_effect_token(&params, 1, params.max_validators, 0, b"x"),
            Err(CapTokenError::ValidatorOutOfBounds)
        );
        assert_eq!(
            validate_effect_token(&params, 1, 1, params.cascade_depth + 1, b"x"),
            Err(CapTokenError::CascadeHealthOutOfRange)
        );
        let big = vec![0u8; params.max_payload_bytes + 1];
        assert_eq!(
            validate_effect_token(&params, 1, 1, 0, &big),
            Err(CapTokenError::PayloadTooLarge)
        );
    }
}
