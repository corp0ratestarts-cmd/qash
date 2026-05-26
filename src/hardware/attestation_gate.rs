//! Hardware attestation gate — Domain B Phase 2 placeholder.
//!
//! This module will wire a TPM 2.0 / TEE attestation chain into the PAL
//! boundary. Attestation quotes are Domain B material; they must never flow
//! into Domain A state-transition inputs.

/// Error type for attestation gate operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationGateError {
    /// Hardware attestation not available on this platform.
    NotAvailable,
    /// Quote generation failed.
    QuoteFailed(String),
    /// Quote verification failed.
    VerificationFailed,
}

/// A raw platform attestation quote (Domain B only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationQuote {
    pub bytes: Vec<u8>,
}

/// Domain B attestation gate trait.
///
/// Implementors bridge to platform attestation hardware (TPM 2.0, Intel TDX,
/// AMD SEV-SNP, ARM CCA). Never feed quote bytes into Domain A computations.
pub trait AttestationGate {
    fn generate_quote(&self, nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError>;
    fn verify_quote(&self, quote: &AttestationQuote) -> Result<(), AttestationGateError>;
}

/// Stub that always returns `NotAvailable`. Replace with a real backend in Phase 2.
pub struct UnimplementedAttestationGate;

impl AttestationGate for UnimplementedAttestationGate {
    fn generate_quote(&self, _nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }

    fn verify_quote(&self, _quote: &AttestationQuote) -> Result<(), AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }
}
