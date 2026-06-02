//! AMD SEV-SNP attestation backend (scaffold).
//!
//! Status: SCAFFOLD — not activated or claimed for v1.0.
//! Feature flag: `sev-snp`
//!
//! Production implementation path:
//!   - Use the `/dev/sev-guest` Linux ioctl interface (kernel 5.19+):
//!     `SNP_GET_REPORT` ioctl with a 64-byte USER_DATA field carrying the nonce.
//!   - Parse the `snp_report_t` response: validate REPORT_DATA matches nonce,
//!     check POLICY bits, verify the VCEK certificate chain against AMD KDS.
//!   - Optionally extend with the `sev` crate for safe Rust bindings.
//!   - Bind to QASH: include the validator identity in USER_DATA[32..64].
//!
//! This stub returns `Err(NotAvailable)` until real SEV-SNP hardware and kernel
//! support are available.

#[cfg(feature = "sev-snp")]
use super::attestation_gate::{AttestationGate, AttestationGateError, AttestationQuote};

/// AMD SEV-SNP attestation backend.
///
/// Generates SEV-SNP attestation reports via the Linux sev-guest driver.
/// Requires AMD EPYC 3rd-Gen (Milan) or later with SEV-SNP enabled in firmware
/// and Linux kernel 5.19+ with `CONFIG_SEV_GUEST`.
///
/// Not active in v1.0; all methods return `Err(NotAvailable)`.
#[cfg(feature = "sev-snp")]
pub struct SevSnpAttestationGate {
    /// Supplementary validator identity for USER_DATA[32..64].
    _validator_identity: [u8; 32],
}

#[cfg(feature = "sev-snp")]
impl SevSnpAttestationGate {
    pub fn new(validator_identity: [u8; 32]) -> Self {
        Self {
            _validator_identity: validator_identity,
        }
    }
}

#[cfg(feature = "sev-snp")]
impl AttestationGate for SevSnpAttestationGate {
    fn generate_quote(&self, _nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }

    fn verify_quote(&self, _quote: &AttestationQuote) -> Result<(), AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }
}

#[cfg(all(test, feature = "sev-snp"))]
mod tests {
    use super::*;

    #[test]
    fn sev_snp_gate_fails_closed_without_hardware() {
        let gate = SevSnpAttestationGate::new([0u8; 32]);
        let nonce = [0u8; 32];
        assert_eq!(
            gate.generate_quote(&nonce),
            Err(AttestationGateError::NotAvailable)
        );
    }
}
