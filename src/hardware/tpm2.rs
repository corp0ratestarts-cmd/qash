//! TPM 2.0 attestation backend (scaffold).
//!
//! Status: SCAFFOLD — not activated or claimed for v1.0.
//! Feature flag: `tpm2`
//!
//! Production implementation path:
//!   - Use `tss-esapi` crate for TPM 2.0 ESAPI access.
//!   - Call `Esys_Quote()` to generate a TPM2_Quote over the nonce,
//!     binding the TPM PCR bank that covers the validator binary.
//!   - Parse the TPMS_ATTEST / TPMT_SIGNATURE structure for verification.
//!   - Seal the epoch signing key to PCR policy so it is inaccessible outside
//!     the expected software state (PCR0..7 + QASH-specific PCR).
//!
//! This stub returns `Err(NotAvailable)` until real hardware and tss-esapi
//! are wired up. Adding `tss-esapi` as a `[dependencies]` entry gated on
//! `tpm2` feature is the first step.

#[cfg(feature = "tpm2")]
use super::attestation_gate::{AttestationGate, AttestationGateError, AttestationQuote};

/// TPM 2.0 attestation backend.
///
/// Generates TPM quotes via the TPM ESAPI. Requires a `/dev/tpm0` or
/// `/dev/tpmrm0` device and a running TPM resource manager (tpm2-abrmd or
/// in-kernel RM on Linux 4.12+).
///
/// Not active in v1.0; all methods return `Err(NotAvailable)`.
#[cfg(feature = "tpm2")]
pub struct Tpm2AttestationGate {
    /// PCR selection for the quote (e.g., SHA-256 bank, PCRs 0-7).
    /// Stored for when the real ESAPI implementation is wired up.
    _pcr_selection: Vec<u8>,
}

#[cfg(feature = "tpm2")]
impl Tpm2AttestationGate {
    /// Create a new TPM 2.0 gate with the given PCR selection.
    ///
    /// `pcr_selection` is an opaque byte blob encoding the TPML_PCR_SELECTION.
    /// The format matches the `tss-esapi` TPML_PCR_SELECTION wire encoding.
    pub fn new(pcr_selection: Vec<u8>) -> Self {
        Self {
            _pcr_selection: pcr_selection,
        }
    }

    /// Create a gate selecting the SHA-256 bank, PCRs 0–7 (typical boot chain).
    pub fn default_boot_pcrs() -> Self {
        Self::new(vec![])
    }
}

#[cfg(feature = "tpm2")]
impl AttestationGate for Tpm2AttestationGate {
    fn generate_quote(&self, _nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }

    fn verify_quote(&self, _quote: &AttestationQuote) -> Result<(), AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }
}

#[cfg(all(test, feature = "tpm2"))]
mod tests {
    use super::*;

    #[test]
    fn tpm2_gate_fails_closed_without_hardware() {
        let gate = Tpm2AttestationGate::default_boot_pcrs();
        let nonce = [0u8; 32];
        assert_eq!(
            gate.generate_quote(&nonce),
            Err(AttestationGateError::NotAvailable)
        );
    }
}
