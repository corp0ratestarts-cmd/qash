//! ARM CCA (Confidential Compute Architecture) attestation backend (scaffold).
//!
//! Status: SCAFFOLD — not activated or claimed for v1.0.
//! Feature flag: `arm-cca`
//!
//! Production implementation path:
//!   - Use the ARM CCA Realm Management Monitor (RMM) SMC interface to generate
//!     a Realm Token: `RSI_ATTESTATION_TOKEN_INIT` / `RSI_ATTESTATION_TOKEN_CONTINUE`
//!     with the attestation challenge (nonce) passed as the RSI challenge.
//!   - Parse the CBOR-encoded CCA platform token (EAT / CoRIM-signed token).
//!   - Verify against the ARM CCA endorsement chain: Realm Token signature,
//!     CCA Platform Token, and the Veraison / Parsec attestation service.
//!   - Bind to QASH: include the validator identity in the challenge upper 32 bytes.
//!   - Requires ARMv9 CPU with CCA enabled and a compatible RMM (e.g., TF-RMM).
//!
//! This stub returns `Err(NotAvailable)` until real ARM CCA hardware is available.

#[cfg(feature = "arm-cca")]
use super::attestation_gate::{AttestationGate, AttestationGateError, AttestationQuote};

/// ARM CCA Realm attestation backend.
///
/// Generates CCA Realm Tokens via the RSI (Realm Services Interface) SMC calls.
/// Requires ARMv9-A hardware with CCA enabled (e.g., Arm Total Compute TC2 /
/// Morello or future production ARMv9 SoC) and a compatible Realm Management
/// Monitor (TF-RMM or equivalent).
///
/// Not active in v1.0; all methods return `Err(NotAvailable)`.
#[cfg(feature = "arm-cca")]
pub struct ArmCcaAttestationGate {
    /// Supplementary validator identity for the challenge upper 32 bytes.
    _validator_identity: [u8; 32],
}

#[cfg(feature = "arm-cca")]
impl ArmCcaAttestationGate {
    pub fn new(validator_identity: [u8; 32]) -> Self {
        Self {
            _validator_identity: validator_identity,
        }
    }
}

#[cfg(feature = "arm-cca")]
impl AttestationGate for ArmCcaAttestationGate {
    fn generate_quote(&self, _nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }

    fn verify_quote(&self, _quote: &AttestationQuote) -> Result<(), AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }
}

#[cfg(all(test, feature = "arm-cca"))]
mod tests {
    use super::*;

    #[test]
    fn arm_cca_gate_fails_closed_without_hardware() {
        let gate = ArmCcaAttestationGate::new([0u8; 32]);
        let nonce = [0u8; 32];
        assert_eq!(
            gate.generate_quote(&nonce),
            Err(AttestationGateError::NotAvailable)
        );
    }
}
