//! Domain B attestation gate interfaces.
//!
//! Hardware attestation quotes and local operator evidence are Domain B
//! material. They must never flow into Domain A state-transition inputs.

use sha3::{Digest, Sha3_256};

const LOCAL_EVIDENCE_QUOTE_MAGIC: &[u8] = b"QASH-LOCAL-EVIDENCE-QUOTE-v1\0";
const LOCAL_EVIDENCE_QUOTE_LEN: usize = 29 + 32 + 32 + 32;
const LOCAL_EVIDENCE_DIGEST_DOMAIN: &[u8] = b"QASH-DOMAIN-B-LOCAL-EVIDENCE-QUOTE\0";

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

/// Local deterministic evidence gate for hosted tests and non-TEE operators.
///
/// This backend is intentionally not a hardware attestation implementation. It
/// packages a caller-supplied platform measurement with a nonce and a digest so
/// Domain B callers can exercise quote generation and verification paths while
/// failing closed on malformed or mismatched evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalEvidenceAttestationGate {
    platform_measurement: [u8; 32],
}

impl LocalEvidenceAttestationGate {
    pub const fn new(platform_measurement: [u8; 32]) -> Self {
        Self {
            platform_measurement,
        }
    }

    pub const fn platform_measurement(&self) -> [u8; 32] {
        self.platform_measurement
    }
}

impl Default for LocalEvidenceAttestationGate {
    fn default() -> Self {
        let mut hasher = Sha3_256::new();
        hasher.update(LOCAL_EVIDENCE_DIGEST_DOMAIN);
        hasher.update(b"default-local-evidence-measurement");
        Self::new(hasher.finalize().into())
    }
}

/// Domain B attestation gate trait.
///
/// Implementors bridge to platform attestation hardware (TPM 2.0, Intel TDX,
/// AMD SEV-SNP, ARM CCA). Never feed quote bytes into Domain A computations.
pub trait AttestationGate {
    fn generate_quote(&self, nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError>;
    fn verify_quote(&self, quote: &AttestationQuote) -> Result<(), AttestationGateError>;
}

impl AttestationGate for LocalEvidenceAttestationGate {
    fn generate_quote(&self, nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError> {
        let digest = local_evidence_digest(nonce, &self.platform_measurement);
        let mut bytes = Vec::with_capacity(LOCAL_EVIDENCE_QUOTE_LEN);
        bytes.extend_from_slice(LOCAL_EVIDENCE_QUOTE_MAGIC);
        bytes.extend_from_slice(nonce);
        bytes.extend_from_slice(&self.platform_measurement);
        bytes.extend_from_slice(&digest);
        Ok(AttestationQuote { bytes })
    }

    fn verify_quote(&self, quote: &AttestationQuote) -> Result<(), AttestationGateError> {
        let parsed = parse_local_evidence_quote(quote)?;
        if parsed.platform_measurement != self.platform_measurement {
            return Err(AttestationGateError::VerificationFailed);
        }
        let expected = local_evidence_digest(&parsed.nonce, &parsed.platform_measurement);
        if parsed.digest != expected {
            return Err(AttestationGateError::VerificationFailed);
        }
        Ok(())
    }
}

struct ParsedLocalEvidenceQuote {
    nonce: [u8; 32],
    platform_measurement: [u8; 32],
    digest: [u8; 32],
}

fn parse_local_evidence_quote(
    quote: &AttestationQuote,
) -> Result<ParsedLocalEvidenceQuote, AttestationGateError> {
    if quote.bytes.len() != LOCAL_EVIDENCE_QUOTE_LEN
        || !quote.bytes.starts_with(LOCAL_EVIDENCE_QUOTE_MAGIC)
    {
        return Err(AttestationGateError::VerificationFailed);
    }

    let mut cursor = LOCAL_EVIDENCE_QUOTE_MAGIC.len();
    let nonce = read_array::<32>(&quote.bytes, &mut cursor)?;
    let platform_measurement = read_array::<32>(&quote.bytes, &mut cursor)?;
    let digest = read_array::<32>(&quote.bytes, &mut cursor)?;
    Ok(ParsedLocalEvidenceQuote {
        nonce,
        platform_measurement,
        digest,
    })
}

fn read_array<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], AttestationGateError> {
    let end = cursor
        .checked_add(N)
        .ok_or(AttestationGateError::VerificationFailed)?;
    let slice = bytes
        .get(*cursor..end)
        .ok_or(AttestationGateError::VerificationFailed)?;
    let array = slice
        .try_into()
        .map_err(|_| AttestationGateError::VerificationFailed)?;
    *cursor = end;
    Ok(array)
}

fn local_evidence_digest(nonce: &[u8; 32], platform_measurement: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(LOCAL_EVIDENCE_DIGEST_DOMAIN);
    hasher.update(nonce);
    hasher.update(platform_measurement);
    hasher.finalize().into()
}

/// Gate that always reports platform attestation as unavailable.
pub struct UnimplementedAttestationGate;

impl AttestationGate for UnimplementedAttestationGate {
    fn generate_quote(&self, _nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }

    fn verify_quote(&self, _quote: &AttestationQuote) -> Result<(), AttestationGateError> {
        Err(AttestationGateError::NotAvailable)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttestationGate, AttestationGateError, AttestationQuote, LocalEvidenceAttestationGate,
        UnimplementedAttestationGate, LOCAL_EVIDENCE_QUOTE_LEN,
    };

    fn nonce() -> [u8; 32] {
        [7u8; 32]
    }

    fn measurement(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn local_evidence_quote_roundtrips() {
        let gate = LocalEvidenceAttestationGate::new(measurement(11));
        let quote = gate.generate_quote(&nonce()).expect("quote generation");

        assert_eq!(quote.bytes.len(), LOCAL_EVIDENCE_QUOTE_LEN);
        gate.verify_quote(&quote).expect("quote verifies");
    }

    #[test]
    fn local_evidence_quote_binds_nonce() {
        let gate = LocalEvidenceAttestationGate::new(measurement(11));
        let first = gate.generate_quote(&nonce()).expect("quote generation");
        let second = gate.generate_quote(&[8u8; 32]).expect("quote generation");

        assert_ne!(first, second);
    }

    #[test]
    fn local_evidence_quote_rejects_tampering() {
        let gate = LocalEvidenceAttestationGate::new(measurement(11));
        let mut quote = gate.generate_quote(&nonce()).expect("quote generation");
        let last = quote.bytes.len() - 1;
        quote.bytes[last] ^= 1;

        assert_eq!(
            gate.verify_quote(&quote),
            Err(AttestationGateError::VerificationFailed)
        );
    }

    #[test]
    fn local_evidence_quote_rejects_wrong_measurement_gate() {
        let gate = LocalEvidenceAttestationGate::new(measurement(11));
        let other_gate = LocalEvidenceAttestationGate::new(measurement(12));
        let quote = gate.generate_quote(&nonce()).expect("quote generation");

        assert_eq!(
            other_gate.verify_quote(&quote),
            Err(AttestationGateError::VerificationFailed)
        );
    }

    #[test]
    fn local_evidence_quote_rejects_bad_shape() {
        let gate = LocalEvidenceAttestationGate::new(measurement(11));

        assert_eq!(
            gate.verify_quote(&AttestationQuote { bytes: Vec::new() }),
            Err(AttestationGateError::VerificationFailed)
        );
    }

    #[test]
    fn unimplemented_gate_fails_closed() {
        let gate = UnimplementedAttestationGate;

        assert_eq!(
            gate.generate_quote(&nonce()),
            Err(AttestationGateError::NotAvailable)
        );
        assert_eq!(
            gate.verify_quote(&AttestationQuote { bytes: Vec::new() }),
            Err(AttestationGateError::NotAvailable)
        );
    }
}
