//! Intel TDX attestation backend (scaffold).
//!
//! Status: SCAFFOLD — not activated or claimed for v1.0.
//! Feature flag: `tdx`
//!
//! Production implementation path:
//!   - Use the `/dev/tdx_guest` Linux ioctl interface (kernel 5.19+) to
//!     obtain a TDX report: `TDX_CMD_GET_REPORT0` ioctl with a 64-byte
//!     REPORTDATA (fill lower 32 bytes with nonce, upper 32 bytes with 0).
//!   - Convert to a TDQUOTE via the Quoting Enclave (QE) service or the
//!     `tdx-attest` library.
//!   - Verify via Intel DCAP / PCS: validate the PCK certificate chain,
//!     check QE identity, verify the ECDSA-P256 quote signature.
//!   - Bind to QASH: include the validator identity in the REPORTDATA so the
//!     quote is specific to this validator instance.
//!
//! This stub returns `Err(NotAvailable)` until real TDX hardware and kernel
//! support are available.

#[cfg(feature = "tdx")]
use super::attestation_gate::{AttestationGate, AttestationGateError, AttestationQuote};

/// Intel TDX Trust Domain attestation backend.
///
/// Generates TDX quotes via the Linux TDX guest driver ioctl. Requires an
/// Intel 4th-Gen Xeon (Sapphire Rapids) or later with TDX enabled in firmware,
/// and Linux kernel 5.19+ with TDX module support.
///
/// Not active in v1.0; all methods return `Err(NotAvailable)`.
#[cfg(feature = "tdx")]
pub struct TdxAttestationGate {
    /// Supplementary validator identity to bind into the REPORTDATA (upper 32 bytes).
    _validator_identity: [u8; 32],
}

#[cfg(feature = "tdx")]
impl TdxAttestationGate {
    pub fn new(validator_identity: [u8; 32]) -> Self {
        Self {
            _validator_identity: validator_identity,
        }
    }
}

#[cfg(feature = "tdx")]
impl AttestationGate for TdxAttestationGate {
    fn generate_quote(&self, _nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }

    fn verify_quote(&self, _quote: &AttestationQuote) -> Result<(), AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }
}

#[cfg(all(test, feature = "tdx"))]
mod tests {
    use super::*;

    #[test]
    fn tdx_gate_fails_closed_without_hardware() {
        let gate = TdxAttestationGate::new([0u8; 32]);
        let nonce = [0u8; 32];
        assert_eq!(
            gate.generate_quote(&nonce),
            Err(AttestationGateError::NotAvailable)
        );
    }
}
