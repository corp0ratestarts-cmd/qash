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

/// Software-hash-Merkle attestation (`software_hash_merkle` genesis mode).
///
/// Builds a two-leaf SHA3-256 Merkle tree over fixed platform measurements and
/// binds it to a caller-supplied nonce. Verification is purely computational —
/// no hardware required.
///
/// Wire format: `version(1) || nonce(32) || root(32) || quote_body(32)` = 97 bytes.
///   leaf_i     = SHA3-256("QASH/attest/leaf/v1" || u8(i) || measurements[i])
///   root       = SHA3-256("QASH/attest/root/v1" || leaf_0 || leaf_1)
///   quote_body = SHA3-256("QASH/attest/quote/v1" || nonce || root)
pub struct SoftwareHashMerkleAttestation {
    platform_identity: [u8; 32],
}

impl SoftwareHashMerkleAttestation {
    pub fn new() -> Self {
        Self {
            platform_identity: software_merkle_identity_hash(),
        }
    }

    pub fn with_identity(identity: [u8; 32]) -> Self {
        Self {
            platform_identity: identity,
        }
    }

    fn compute_root(&self) -> [u8; 32] {
        let genesis = qash_consensus::params::consensus_params_hash();

        let leaf0: [u8; 32] = {
            let mut h = Sha3_256::new();
            h.update(b"QASH/attest/leaf/v1");
            h.update([0x00]);
            h.update(genesis);
            h.finalize().into()
        };
        let leaf1: [u8; 32] = {
            let mut h = Sha3_256::new();
            h.update(b"QASH/attest/leaf/v1");
            h.update([0x01]);
            h.update(self.platform_identity);
            h.finalize().into()
        };
        let mut h = Sha3_256::new();
        h.update(b"QASH/attest/root/v1");
        h.update(leaf0);
        h.update(leaf1);
        h.finalize().into()
    }

    fn compute_quote_body(nonce: &[u8; 32], root: &[u8; 32]) -> [u8; 32] {
        let mut h = Sha3_256::new();
        h.update(b"QASH/attest/quote/v1");
        h.update(nonce);
        h.update(root);
        h.finalize().into()
    }
}

impl Default for SoftwareHashMerkleAttestation {
    fn default() -> Self {
        Self::new()
    }
}

impl AttestationGate for SoftwareHashMerkleAttestation {
    fn generate_quote(&self, nonce: &[u8; 32]) -> Result<AttestationQuote, AttestationGateError> {
        let root = self.compute_root();
        let body = Self::compute_quote_body(nonce, &root);
        let mut bytes = Vec::with_capacity(97);
        bytes.push(0x01u8);
        bytes.extend_from_slice(nonce);
        bytes.extend_from_slice(&root);
        bytes.extend_from_slice(&body);
        Ok(AttestationQuote { bytes })
    }

    fn verify_quote(&self, quote: &AttestationQuote) -> Result<(), AttestationGateError> {
        if quote.bytes.len() != 97 {
            return Err(AttestationGateError::VerificationFailed);
        }
        let ver = quote
            .bytes
            .first()
            .ok_or(AttestationGateError::VerificationFailed)?;
        if *ver != 0x01 {
            return Err(AttestationGateError::VerificationFailed);
        }
        let nonce: &[u8; 32] = quote
            .bytes
            .get(1..33)
            .ok_or(AttestationGateError::VerificationFailed)?
            .try_into()
            .map_err(|_| AttestationGateError::VerificationFailed)?;
        let claimed_root: &[u8; 32] = quote
            .bytes
            .get(33..65)
            .ok_or(AttestationGateError::VerificationFailed)?
            .try_into()
            .map_err(|_| AttestationGateError::VerificationFailed)?;
        let claimed_body: &[u8; 32] = quote
            .bytes
            .get(65..97)
            .ok_or(AttestationGateError::VerificationFailed)?
            .try_into()
            .map_err(|_| AttestationGateError::VerificationFailed)?;

        let expected_root = self.compute_root();
        if expected_root != *claimed_root {
            return Err(AttestationGateError::VerificationFailed);
        }
        let expected_body = Self::compute_quote_body(nonce, claimed_root);
        if expected_body != *claimed_body {
            return Err(AttestationGateError::VerificationFailed);
        }
        Ok(())
    }
}

fn software_merkle_identity_hash() -> [u8; 32] {
    let mut h = Sha3_256::new();
    h.update(b"QASH/platform/software-hash-merkle/v1");
    h.finalize().into()
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
    use sha3::{Digest, Sha3_256};

    fn nonce() -> [u8; 32] {
        test_hash(b"nonce")
    }

    fn measurement(byte: u8) -> [u8; 32] {
        test_hash(&[b"measurement".as_slice(), &[byte]].concat())
    }

    fn test_hash(label: &[u8]) -> [u8; 32] {
        let mut h = Sha3_256::new();
        h.update(b"QASH/test/attestation/v1");
        h.update(label);
        h.finalize().into()
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

    // --- SoftwareHashMerkleAttestation tests ---

    #[test]
    fn merkle_roundtrip_generate_and_verify() {
        use super::SoftwareHashMerkleAttestation;
        let gate = SoftwareHashMerkleAttestation::new();
        let quote = gate.generate_quote(&nonce()).unwrap();
        gate.verify_quote(&quote).unwrap();
    }

    #[test]
    fn merkle_quote_is_97_bytes_with_version_1() {
        use super::SoftwareHashMerkleAttestation;
        let gate = SoftwareHashMerkleAttestation::new();
        let quote = gate.generate_quote(&nonce()).unwrap();
        assert_eq!(quote.bytes.len(), 97);
        assert_eq!(quote.bytes[0], 0x01);
    }

    #[test]
    fn merkle_different_nonces_same_root() {
        use super::SoftwareHashMerkleAttestation;
        let gate = SoftwareHashMerkleAttestation::new();
        let q1 = gate.generate_quote(&test_hash(b"nonce-a")).unwrap();
        let q2 = gate.generate_quote(&test_hash(b"nonce-b")).unwrap();
        assert_ne!(q1, q2);
        assert_eq!(&q1.bytes[33..65], &q2.bytes[33..65]);
    }

    #[test]
    fn merkle_tampered_root_fails() {
        use super::SoftwareHashMerkleAttestation;
        let gate = SoftwareHashMerkleAttestation::new();
        let mut quote = gate.generate_quote(&nonce()).unwrap();
        quote.bytes[33] ^= 0xff;
        assert_eq!(
            gate.verify_quote(&quote),
            Err(AttestationGateError::VerificationFailed)
        );
    }

    #[test]
    fn merkle_tampered_body_fails() {
        use super::SoftwareHashMerkleAttestation;
        let gate = SoftwareHashMerkleAttestation::new();
        let mut quote = gate.generate_quote(&nonce()).unwrap();
        quote.bytes[65] ^= 0xff;
        assert_eq!(
            gate.verify_quote(&quote),
            Err(AttestationGateError::VerificationFailed)
        );
    }

    #[test]
    fn merkle_wrong_identity_fails() {
        use super::SoftwareHashMerkleAttestation;
        let gate_a = SoftwareHashMerkleAttestation::with_identity(test_hash(b"identity-a"));
        let gate_b = SoftwareHashMerkleAttestation::with_identity(test_hash(b"identity-b"));
        let quote = gate_a.generate_quote(&nonce()).unwrap();
        assert_eq!(
            gate_b.verify_quote(&quote),
            Err(AttestationGateError::VerificationFailed)
        );
    }

    #[test]
    fn merkle_truncated_quote_fails() {
        use super::SoftwareHashMerkleAttestation;
        let gate = SoftwareHashMerkleAttestation::new();
        let mut quote = gate.generate_quote(&nonce()).unwrap();
        quote.bytes.truncate(50);
        assert_eq!(
            gate.verify_quote(&quote),
            Err(AttestationGateError::VerificationFailed)
        );
    }

    #[test]
    fn merkle_extended_quote_fails() {
        use super::SoftwareHashMerkleAttestation;
        let gate = SoftwareHashMerkleAttestation::new();
        let mut quote = gate.generate_quote(&nonce()).unwrap();
        quote.bytes.push(0x00);
        assert_eq!(
            gate.verify_quote(&quote),
            Err(AttestationGateError::VerificationFailed)
        );
    }
}
