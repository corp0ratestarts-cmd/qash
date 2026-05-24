//! Zero-persistence Domain B admission primitives.
//!
//! This module is the production-side counterpart to the hosted replay scaffold.
//! Raw envelope bytes are owned by [`EphemeralEnvelope`], inspected through
//! borrowed views, converted to fixed-width commitments, and zeroized on drop.

use core::marker::PhantomData;

/// Fixed-width effect admitted across the Domain B -> Domain A boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedEffectCommitment {
    pub effect_root: [u8; 32],
    pub receipt_root: [u8; 32],
    pub epoch: u64,
}

/// Error class that may be logged without carrying raw payload material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionErrorClass {
    DecodeInvalid,
    SchemaInvalid,
    BoundsInvalid,
    AuthInvalid,
}

/// Redacted admission error. It intentionally carries no raw bytes, peer
/// addresses, receipt bodies, or payload-derived strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionError {
    pub class: AdmissionErrorClass,
}

/// Owned raw admission slot.
///
/// This type intentionally does not implement `Clone`, `Debug`, `Display`, or
/// serialization traits. The pointer marker makes the type `!Send` and `!Sync`
/// so raw envelope ownership cannot be casually moved to background workers.
pub struct EphemeralEnvelope<const N: usize> {
    data: [u8; N],
    len: usize,
    _no_send_sync: PhantomData<*mut ()>,
}

impl<const N: usize> EphemeralEnvelope<N> {
    pub fn new(input: &[u8]) -> Result<Self, AdmissionError> {
        if input.len() > N {
            return Err(AdmissionError {
                class: AdmissionErrorClass::BoundsInvalid,
            });
        }
        let mut data = [0u8; N];
        data[..input.len()].copy_from_slice(input);
        Ok(Self {
            data,
            len: input.len(),
            _no_send_sync: PhantomData,
        })
    }

    fn as_view(&self) -> EnvelopeView<'_> {
        EnvelopeView {
            bytes: &self.data[..self.len],
        }
    }
}

impl<const N: usize> Drop for EphemeralEnvelope<N> {
    fn drop(&mut self) {
        for byte in &mut self.data {
            unsafe { core::ptr::write_volatile(byte, 0) };
        }
        self.len = 0;
    }
}

/// Borrowed parser view over raw envelope bytes.
#[derive(Clone, Copy)]
struct EnvelopeView<'a> {
    bytes: &'a [u8],
}

/// Consume an owned ephemeral envelope and return only scalar commitments.
///
/// The slot is consumed by value so the raw buffer is dropped and zeroized as
/// this function unwinds. No owned payload copy is returned.
pub fn process_envelope<const N: usize>(
    slot: EphemeralEnvelope<N>,
) -> Result<ValidatedEffectCommitment, AdmissionError> {
    let view = parse_in_place(slot.as_view())?;
    validate_effect_view(view)
}

fn parse_in_place(view: EnvelopeView<'_>) -> Result<EnvelopeView<'_>, AdmissionError> {
    if view.bytes.len() < 72 {
        return Err(AdmissionError {
            class: AdmissionErrorClass::DecodeInvalid,
        });
    }
    Ok(view)
}

fn validate_effect_view(view: EnvelopeView<'_>) -> Result<ValidatedEffectCommitment, AdmissionError> {
    if view.bytes[0] != 1 {
        return Err(AdmissionError {
            class: AdmissionErrorClass::SchemaInvalid,
        });
    }

    let mut pos = 8;
    let epoch = read_u64(view.bytes, &mut pos)?;
    let mut effect_root = [0u8; 32];
    effect_root.copy_from_slice(&view.bytes[pos..pos + 32]);
    pos += 32;
    let mut receipt_root = [0u8; 32];
    receipt_root.copy_from_slice(&view.bytes[pos..pos + 32]);

    Ok(ValidatedEffectCommitment {
        effect_root,
        receipt_root,
        epoch,
    })
}

fn read_u64(bytes: &[u8], pos: &mut usize) -> Result<u64, AdmissionError> {
    if *pos + 8 > bytes.len() {
        return Err(AdmissionError {
            class: AdmissionErrorClass::DecodeInvalid,
        });
    }
    let mut out = [0u8; 8];
    out.copy_from_slice(&bytes[*pos..*pos + 8]);
    *pos += 8;
    Ok(u64::from_le_bytes(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_envelope(epoch: u64) -> [u8; 72] {
        let mut bytes = [0u8; 72];
        bytes[0] = 1;
        bytes[8..16].copy_from_slice(&epoch.to_le_bytes());
        bytes[16..48].copy_from_slice(&[7u8; 32]);
        bytes[48..72].copy_from_slice(&[9u8; 24]);
        bytes
    }

    #[test]
    fn process_envelope_returns_only_commitment() {
        let bytes = valid_envelope(42);
        let slot = EphemeralEnvelope::<128>::new(&bytes).unwrap();
        let commitment = process_envelope(slot).unwrap();
        assert_eq!(commitment.epoch, 42);
        assert_eq!(commitment.effect_root, [7u8; 32]);
    }

    #[test]
    fn process_envelope_rejects_bad_schema_without_payload_error() {
        let mut bytes = valid_envelope(1);
        bytes[0] = 0;
        let slot = EphemeralEnvelope::<128>::new(&bytes).unwrap();
        let err = process_envelope(slot).unwrap_err();
        assert_eq!(err.class, AdmissionErrorClass::SchemaInvalid);
    }
}
