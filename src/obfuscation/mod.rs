//! Obfuscation cascade layer — Domain B transport material.
//!
//! This module applies the canonical QASH cascade from `qash-consensus` with a
//! caller-supplied Domain B context. The cascade output can be used to wrap or
//! route transport material, but it must not be treated as a Domain A state
//! transition or as a production privacy proof.

const OUTPUT_BYTES: usize = 64;

/// Error type for obfuscation cascade operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObfuscationError {
    /// Cascade not yet implemented.
    NotImplemented,
    /// Input length is not a multiple of the expected block size.
    InvalidInputLength,
    /// Domain separator is empty.
    InvalidDomain,
    /// Cascade output bytes do not match the expected canonical output.
    VerificationFailed,
}

/// Obfuscation cascade output (Domain B only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CascadeOutput {
    pub bytes: Vec<u8>,
}

impl CascadeOutput {
    /// Convert the heap-backed output into the canonical fixed-size form.
    pub fn try_into_array(self) -> Result<[u8; OUTPUT_BYTES], ObfuscationError> {
        self.bytes
            .try_into()
            .map_err(|_| ObfuscationError::InvalidInputLength)
    }

    /// Borrow the output as the canonical fixed-size byte array.
    pub fn as_array(&self) -> Result<[u8; OUTPUT_BYTES], ObfuscationError> {
        self.bytes
            .as_slice()
            .try_into()
            .map_err(|_| ObfuscationError::InvalidInputLength)
    }
}

/// Domain B obfuscation cascade trait.
///
/// Implementors must be deterministic and must not observe clocks, entropy, or
/// host-specific state.
pub trait ObfuscationCascade {
    /// Apply the Domain B cascade.
    ///
    /// # Errors
    ///
    /// Returns an error when the domain is invalid or the implementation is not
    /// available.
    fn apply(&self, input: &[u8], domain: &[u8]) -> Result<CascadeOutput, ObfuscationError>;

    /// Recompute and compare a canonical cascade output.
    fn verify(
        &self,
        input: &[u8],
        domain: &[u8],
        expected: &CascadeOutput,
    ) -> Result<(), ObfuscationError> {
        let computed = self.apply(input, domain)?;
        if computed.bytes == expected.bytes && expected.bytes.len() == OUTPUT_BYTES {
            Ok(())
        } else {
            Err(ObfuscationError::VerificationFailed)
        }
    }
}

/// Deterministic Domain B obfuscation cascade.
///
/// The implementation domain-separates by using `domain` as the cascade context
/// key and `input` as the cascade message. It returns the canonical 64-byte
/// cascade output without admitting anything into Domain A.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeterministicCascade;

impl ObfuscationCascade for DeterministicCascade {
    fn apply(&self, input: &[u8], domain: &[u8]) -> Result<CascadeOutput, ObfuscationError> {
        if domain.is_empty() {
            return Err(ObfuscationError::InvalidDomain);
        }
        let out = crate::crypto::cascade::h_cascade_keyed(domain, input);
        Ok(CascadeOutput {
            bytes: out.to_vec(),
        })
    }
}

/// Stub that always returns `NotImplemented`. Replace in Phase 2.
pub struct UnimplementedCascade;

impl ObfuscationCascade for UnimplementedCascade {
    fn apply(&self, _input: &[u8], _domain: &[u8]) -> Result<CascadeOutput, ObfuscationError> {
        Err(ObfuscationError::NotImplemented)
    }
}

pub fn output_bytes() -> usize {
    OUTPUT_BYTES
}

#[cfg(test)]
mod tests {
    use super::{
        output_bytes, CascadeOutput, DeterministicCascade, ObfuscationCascade, ObfuscationError,
    };

    #[test]
    fn deterministic_cascade_returns_fixed_size_output() {
        let cascade = DeterministicCascade;
        let out = cascade
            .apply(
                b"offline clone transport material",
                b"QASH-TEST-OBFUSCATION",
            )
            .expect("valid cascade input");

        assert_eq!(out.bytes.len(), output_bytes());
        assert_ne!(out.bytes, vec![0u8; output_bytes()]);
        assert_eq!(
            out.as_array().expect("canonical output").len(),
            output_bytes()
        );
    }

    #[test]
    fn deterministic_cascade_is_stable_for_same_domain_and_input() {
        let cascade = DeterministicCascade;
        let first = cascade.apply(b"payload", b"domain-a").unwrap();
        let second = cascade.apply(b"payload", b"domain-a").unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn deterministic_cascade_binds_domain_and_input() {
        let cascade = DeterministicCascade;
        let baseline = cascade.apply(b"payload", b"domain-a").unwrap();

        assert_ne!(baseline, cascade.apply(b"payload", b"domain-b").unwrap());
        assert_ne!(baseline, cascade.apply(b"payload-2", b"domain-a").unwrap());
    }

    #[test]
    fn deterministic_cascade_rejects_empty_domain() {
        let cascade = DeterministicCascade;

        assert_eq!(
            cascade.apply(b"payload", b""),
            Err(ObfuscationError::InvalidDomain)
        );
    }

    #[test]
    fn cascade_output_rejects_noncanonical_length() {
        let output = CascadeOutput {
            bytes: vec![0u8; output_bytes() - 1],
        };

        assert_eq!(output.as_array(), Err(ObfuscationError::InvalidInputLength));
    }

    #[test]
    fn deterministic_cascade_verifies_expected_output() {
        let cascade = DeterministicCascade;
        let expected = cascade.apply(b"payload", b"domain-a").unwrap();

        cascade
            .verify(b"payload", b"domain-a", &expected)
            .expect("expected output verifies");
    }

    #[test]
    fn deterministic_cascade_verification_rejects_tampering() {
        let cascade = DeterministicCascade;
        let mut expected = cascade.apply(b"payload", b"domain-a").unwrap();
        expected.bytes[0] ^= 1;

        assert_eq!(
            cascade.verify(b"payload", b"domain-a", &expected),
            Err(ObfuscationError::VerificationFailed)
        );
    }

    #[test]
    fn deterministic_cascade_verification_rejects_truncation() {
        let cascade = DeterministicCascade;
        let mut expected = cascade.apply(b"payload", b"domain-a").unwrap();
        expected.bytes.pop();

        assert_eq!(
            cascade.verify(b"payload", b"domain-a", &expected),
            Err(ObfuscationError::VerificationFailed)
        );
    }
}
