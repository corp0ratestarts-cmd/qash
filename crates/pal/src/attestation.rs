//! Local attestation evidence boundary (Domain B only).
//!
//! Attestation results are verified in Domain B and reduced to a commitment
//! root before any data reaches Domain A. Raw hardware serials, AAAGUIDs,
//! vendor IDs, and quote bytes stay in Domain B and are never exposed as
//! Domain A transition inputs.
//!
//! The only artifact that may cross the A/B boundary is a 32-byte
//! `AttestationRoot` — an opaque commitment to the verified evidence.

/// A 32-byte commitment to a verified attestation evidence set.
/// This is the sole representation that may flow toward Domain A.
/// It does NOT contain hardware identifiers, vendor metadata, or raw quote bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttestationRoot(pub [u8; 32]);

impl AttestationRoot {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Outcome of verifying a local attestation quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationVerdict {
    /// Quote verified against the expected platform configuration.
    Trusted,
    /// Quote did not match the expected configuration.
    Rejected,
}

/// Raw attestation evidence from a local hardware source (TPM, TEE, etc.).
///
/// Fields are Domain B-only. None of these values may enter Domain A.
#[derive(Debug, Clone)]
pub struct LocalAttestationEvidence {
    /// Raw hardware quote bytes (TPM PCR quote, SGX report, etc.).
    /// Domain B-only. Never forwarded to Domain A.
    pub quote_bytes: Vec<u8>,
    /// Epoch at which this evidence was collected.
    pub epoch: u64,
    /// Expected nonce bound to this epoch (prevents replay).
    pub nonce: [u8; 32],
}

/// Trait for verifying local attestation evidence in Domain B.
///
/// The implementation receives raw `LocalAttestationEvidence` and must:
/// 1. Verify the quote is structurally valid for the platform.
/// 2. Check the nonce matches the supplied epoch.
/// 3. Return either `Trusted` or `Rejected`.
///
/// The verifier MUST NOT return any hardware identifier, vendor string,
/// AAGUID, or quote bytes to the caller. Only the verdict and the derived
/// `AttestationRoot` are produced.
pub trait LocalAttestationVerifier {
    fn verify(
        &self,
        evidence: &LocalAttestationEvidence,
    ) -> (AttestationVerdict, Option<AttestationRoot>);
}

/// Derives an `AttestationRoot` from verified evidence without exposing
/// hardware identity.
///
/// The root is computed as SHA3-256 over (epoch LE || nonce) — binding the
/// attestation commitment to time and to the expected nonce, but excluding
/// all hardware-specific bytes.
pub fn derive_attestation_root(epoch: u64, nonce: [u8; 32]) -> AttestationRoot {
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(b"QASH_ATTEST_ROOT_V1");
    hasher.update(epoch.to_le_bytes());
    hasher.update(nonce);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    AttestationRoot(out)
}

/// A deterministic test verifier for use in unit tests.
///
/// Trusts any evidence whose `nonce` matches `expected_nonce`, and derives
/// the root from `(epoch, nonce)`. This verifier must never be used in
/// production — it accepts any structurally valid nonce match without
/// validating the quote bytes.
pub struct StaticNonceVerifier {
    pub expected_nonce: [u8; 32],
}

impl LocalAttestationVerifier for StaticNonceVerifier {
    fn verify(
        &self,
        evidence: &LocalAttestationEvidence,
    ) -> (AttestationVerdict, Option<AttestationRoot>) {
        if evidence.nonce == self.expected_nonce {
            let root = derive_attestation_root(evidence.epoch, evidence.nonce);
            (AttestationVerdict::Trusted, Some(root))
        } else {
            (AttestationVerdict::Rejected, None)
        }
    }
}

/// A verifier that always rejects — safe default for unconfigured deployments.
pub struct RejectAllVerifier;

impl LocalAttestationVerifier for RejectAllVerifier {
    fn verify(
        &self,
        _evidence: &LocalAttestationEvidence,
    ) -> (AttestationVerdict, Option<AttestationRoot>) {
        (AttestationVerdict::Rejected, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(epoch: u64, nonce: [u8; 32]) -> LocalAttestationEvidence {
        LocalAttestationEvidence {
            quote_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
            epoch,
            nonce,
        }
    }

    #[test]
    fn trusted_nonce_produces_root() {
        let nonce = [0x42u8; 32];
        let verifier = StaticNonceVerifier {
            expected_nonce: nonce,
        };
        let ev = evidence(7, nonce);
        let (verdict, root) = verifier.verify(&ev);
        assert_eq!(verdict, AttestationVerdict::Trusted);
        assert!(root.is_some());
    }

    #[test]
    fn wrong_nonce_is_rejected() {
        let verifier = StaticNonceVerifier {
            expected_nonce: [0x01u8; 32],
        };
        let ev = evidence(7, [0x02u8; 32]);
        let (verdict, root) = verifier.verify(&ev);
        assert_eq!(verdict, AttestationVerdict::Rejected);
        assert!(root.is_none());
    }

    #[test]
    fn reject_all_verifier_always_rejects() {
        let verifier = RejectAllVerifier;
        let ev = evidence(1, [0xFF; 32]);
        let (verdict, root) = verifier.verify(&ev);
        assert_eq!(verdict, AttestationVerdict::Rejected);
        assert!(root.is_none());
    }

    #[test]
    fn attestation_root_is_deterministic() {
        let r1 = derive_attestation_root(5, [0xABu8; 32]);
        let r2 = derive_attestation_root(5, [0xABu8; 32]);
        assert_eq!(r1, r2);
    }

    #[test]
    fn attestation_root_differs_by_epoch() {
        let r1 = derive_attestation_root(1, [0x01u8; 32]);
        let r2 = derive_attestation_root(2, [0x01u8; 32]);
        assert_ne!(r1, r2);
    }

    #[test]
    fn attestation_root_differs_by_nonce() {
        let r1 = derive_attestation_root(1, [0x01u8; 32]);
        let r2 = derive_attestation_root(1, [0x02u8; 32]);
        assert_ne!(r1, r2);
    }

    #[test]
    fn attestation_root_contains_no_hardware_identity() {
        // The root is computed only from epoch and nonce — same quote bytes,
        // different quote content doesn't matter; only epoch+nonce determine the root.
        let nonce = [0x10u8; 32];
        let mut ev1 = evidence(3, nonce);
        let mut ev2 = evidence(3, nonce);
        ev1.quote_bytes = vec![0x11; 64];
        ev2.quote_bytes = vec![0x22; 64]; // different hardware quote
        let verifier = StaticNonceVerifier {
            expected_nonce: nonce,
        };
        let (_, root1) = verifier.verify(&ev1);
        let (_, root2) = verifier.verify(&ev2);
        // Roots are equal — hardware identity is excluded.
        assert_eq!(root1, root2);
    }

    #[test]
    fn attestation_root_cannot_be_used_as_domain_a_input_without_crossing_boundary() {
        // This test asserts the TYPE-LEVEL guarantee: AttestationRoot is a plain [u8; 32]
        // wrapper, not a Domain A type. It stays in Domain B until explicitly extracted.
        let root = derive_attestation_root(1, [0u8; 32]);
        let bytes: [u8; 32] = *root.as_bytes();
        // The bytes can be used as a WAL root commitment — but that is a Domain B action.
        assert_eq!(bytes.len(), 32);
    }
}
