//! Class IV disclosure key management — epoch-scoped, genesis-authorised.
//!
//! Implements the disclosure domain model from §P4a and §P7 of
//! `docs/spec/09_privacy_model.md`.
//!
//! # Security properties (normative)
//!
//! - `DisclosureKey` is Domain B only; it never crosses into Domain A.
//! - Disclosure is non-retroactive: a key authorised at epoch T cannot decrypt
//!   receipts from epoch T−k, even with full regulatory cooperation.
//! - `DisclosureKey` implements `ZeroizeOnDrop` — key material is wiped on drop.
//! - Epoch-scoped: decryption fails with `EpochOutOfRange` outside the key's
//!   declared `(activation_epoch, expiry_epoch)` window.
//! - Requires a valid `LawfulBasis` from the caller for every disclosure operation.

use sha3::{Digest, Sha3_256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::LawfulBasis;

// ── Disclosure domain configuration ──────────────────────────────────────────

/// Genesis-authorised disclosure domain configuration.
///
/// One `DisclosureDomain` per jurisdiction / regulatory scope. Multiple domains
/// may coexist in a single deployment; each has its own epoch-scoped key material.
///
/// Populated from `[disclosure_domain]` in `GENESIS_CONSTANTS.toml` at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisclosureDomain {
    /// Human-readable domain identifier (e.g. "EU-AML" or "US-OFAC").
    pub domain_id: [u8; 32],
    /// ISO 3166-1 alpha-2 jurisdiction code.
    pub jurisdiction: [u8; 2],
    /// Epoch at which this domain's disclosure key first becomes valid.
    /// Receipts from epochs < activation_epoch cannot be disclosed.
    pub activation_epoch: u64,
    /// Epoch at which this domain's disclosure key expires.
    /// Receipts from epochs >= expiry_epoch cannot be disclosed.
    pub expiry_epoch: u64,
    /// Genesis-committed public commitment to the disclosure key.
    /// The actual private key material is held in `DisclosureKey`.
    pub key_commitment: [u8; 32],
}

impl DisclosureDomain {
    /// Returns true if `epoch` falls within this domain's authorised range.
    pub fn epoch_is_in_scope(&self, epoch: u64) -> bool {
        epoch >= self.activation_epoch && epoch < self.expiry_epoch
    }
}

// ── Disclosure key (private, Domain B only) ───────────────────────────────────

/// Genesis-authorised Class IV disclosure key.
///
/// Private key material for epoch-scoped receipt disclosure.
/// Must NEVER leave Domain B. Never serialized, never logged.
/// Zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct DisclosureKey {
    /// Raw 32-byte key material derived at genesis.
    key_material: [u8; 32],
    /// Epoch range this key is valid for.
    pub activation_epoch: u64,
    pub expiry_epoch: u64,
    /// Domain this key belongs to (identifies the `DisclosureDomain`).
    pub domain_id: [u8; 32],
}

impl DisclosureKey {
    /// Construct a `DisclosureKey` from genesis-provided material.
    ///
    /// The caller is responsible for ensuring the key was loaded from a
    /// genesis-authorised source (HSM, secure enclave, or genesis ceremony).
    pub fn from_genesis_material(
        key_material: [u8; 32],
        activation_epoch: u64,
        expiry_epoch: u64,
        domain_id: [u8; 32],
    ) -> Self {
        Self { key_material, activation_epoch, expiry_epoch, domain_id }
    }

    /// Derive an epoch-scoped decryption key for `epoch` from this disclosure key.
    ///
    /// Returns `Err(DisclosureRequestError::EpochOutOfRange)` if `epoch` is outside
    /// `[activation_epoch, expiry_epoch)`.
    pub fn derive_epoch_key(
        &self,
        epoch: u64,
    ) -> Result<EpochDisclosureKey, DisclosureRequestError> {
        if epoch < self.activation_epoch || epoch >= self.expiry_epoch {
            return Err(DisclosureRequestError::EpochOutOfRange {
                epoch,
                activation: self.activation_epoch,
                expiry: self.expiry_epoch,
            });
        }
        let mut h = Sha3_256::new();
        h.update(b"QASH-CLASS-IV-EPOCH-KEY-V1\x00");
        h.update(self.domain_id);
        h.update(epoch.to_be_bytes());
        h.update(self.key_material);
        let derived: [u8; 32] = h.finalize().into();
        Ok(EpochDisclosureKey { key: derived, epoch, domain_id: self.domain_id })
    }

    /// Compute the public commitment to this key (for genesis verification).
    ///
    /// Must match `DisclosureDomain::key_commitment` from genesis constants.
    pub fn compute_commitment(&self) -> [u8; 32] {
        let mut h = Sha3_256::new();
        h.update(b"QASH-CLASS-IV-KEY-COMMITMENT-V1\x00");
        h.update(self.domain_id);
        h.update(self.key_material);
        h.finalize().into()
    }
}

/// Epoch-scoped disclosure key — single-epoch, zeroized on drop.
///
/// Does not implement Debug to prevent accidental key material logging.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct EpochDisclosureKey {
    pub(crate) key: [u8; 32],
    pub epoch: u64,
    pub domain_id: [u8; 32],
}

// ── Disclosure request ────────────────────────────────────────────────────────

/// A validated Class IV disclosure request.
///
/// Must be presented together with a `DisclosureKey` for any receipt decryption.
/// The combination of lawful basis + epoch scope enforces the two-factor
/// authorization model from §P4a.
pub struct DisclosureRequest {
    /// The lawful basis authorising this disclosure.
    pub lawful_basis: LawfulBasis,
    /// The requesting authority's identity (opaque 32-byte hash of credentials).
    pub requester_id: [u8; 32],
    /// The epoch range covered by this request.
    pub epoch_start: u64,
    pub epoch_end: u64,
    /// Opaque case reference (court order number hash, etc.).
    pub case_reference: [u8; 32],
}

/// Errors produced by disclosure request validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisclosureRequestError {
    /// Requested epoch is outside the disclosure key's authorised range.
    EpochOutOfRange {
        epoch: u64,
        activation: u64,
        expiry: u64,
    },
    /// The disclosure request's epoch range does not match the key's range.
    RequestExceedsKeyScope,
    /// The requester identity is zero (likely uninitialized).
    RequesterIdentityBlank,
}

/// Validate a `DisclosureRequest` against a `DisclosureDomain` and `DisclosureKey`.
///
/// Returns `Ok(())` if the request is valid and the key covers the requested range.
/// The caller must still perform the actual decryption via `RegulatedReceiptDecrypt`.
pub fn validate_disclosure_request(
    request: &DisclosureRequest,
    domain: &DisclosureDomain,
    key: &DisclosureKey,
) -> Result<(), DisclosureRequestError> {
    if request.requester_id == [0u8; 32] {
        return Err(DisclosureRequestError::RequesterIdentityBlank);
    }
    // Key scope must cover the full requested epoch range.
    if request.epoch_start < key.activation_epoch || request.epoch_end > key.expiry_epoch {
        return Err(DisclosureRequestError::RequestExceedsKeyScope);
    }
    // Domain must cover the full requested epoch range.
    if !domain.epoch_is_in_scope(request.epoch_start)
        || !domain.epoch_is_in_scope(request.epoch_end.saturating_sub(1))
    {
        return Err(DisclosureRequestError::RequestExceedsKeyScope);
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_domain() -> DisclosureDomain {
        DisclosureDomain {
            domain_id: *b"QASH-TEST-DOMAIN-EU-AML-0000000\x00",
            jurisdiction: *b"DE",
            activation_epoch: 100,
            expiry_epoch: 200,
            key_commitment: [0u8; 32],
        }
    }

    fn test_key() -> DisclosureKey {
        DisclosureKey::from_genesis_material(
            [0x42u8; 32],
            100,
            200,
            *b"QASH-TEST-DOMAIN-EU-AML-0000000\x00",
        )
    }

    #[test]
    fn epoch_in_scope_passes() {
        let domain = test_domain();
        assert!(domain.epoch_is_in_scope(100));
        assert!(domain.epoch_is_in_scope(150));
        assert!(domain.epoch_is_in_scope(199));
    }

    #[test]
    fn epoch_out_of_scope_rejected() {
        let domain = test_domain();
        assert!(!domain.epoch_is_in_scope(99));
        assert!(!domain.epoch_is_in_scope(200));
        assert!(!domain.epoch_is_in_scope(300));
    }

    #[test]
    fn derive_epoch_key_in_range() {
        let key = test_key();
        let ek = key.derive_epoch_key(150).unwrap();
        assert_eq!(ek.epoch, 150);
    }

    #[test]
    fn derive_epoch_key_out_of_range() {
        let key = test_key();
        assert!(matches!(
            key.derive_epoch_key(99),
            Err(DisclosureRequestError::EpochOutOfRange { epoch: 99, .. })
        ));
        assert!(matches!(
            key.derive_epoch_key(200),
            Err(DisclosureRequestError::EpochOutOfRange { epoch: 200, .. })
        ));
    }

    #[test]
    fn different_epochs_produce_different_keys() {
        let key = test_key();
        let k1 = key.derive_epoch_key(100).unwrap().key;
        let k2 = key.derive_epoch_key(101).unwrap().key;
        assert_ne!(k1, k2, "epoch keys must be distinct");
    }

    #[test]
    fn validate_request_passes() {
        let domain = test_domain();
        let key = test_key();
        let req = DisclosureRequest {
            lawful_basis: LawfulBasis::GdprArt6LegalObligation,
            requester_id: [1u8; 32],
            epoch_start: 100,
            epoch_end: 150,
            case_reference: [2u8; 32],
        };
        assert!(validate_disclosure_request(&req, &domain, &key).is_ok());
    }

    #[test]
    fn validate_request_blank_requester_rejected() {
        let domain = test_domain();
        let key = test_key();
        let req = DisclosureRequest {
            lawful_basis: LawfulBasis::GdprArt6LegalObligation,
            requester_id: [0u8; 32],
            epoch_start: 100,
            epoch_end: 150,
            case_reference: [2u8; 32],
        };
        assert_eq!(
            validate_disclosure_request(&req, &domain, &key).unwrap_err(),
            DisclosureRequestError::RequesterIdentityBlank
        );
    }

    #[test]
    fn validate_request_out_of_range_rejected() {
        let domain = test_domain();
        let key = test_key();
        let req = DisclosureRequest {
            lawful_basis: LawfulBasis::GdprArt6LegalObligation,
            requester_id: [1u8; 32],
            epoch_start: 50,
            epoch_end: 150,
            case_reference: [2u8; 32],
        };
        assert_eq!(
            validate_disclosure_request(&req, &domain, &key).unwrap_err(),
            DisclosureRequestError::RequestExceedsKeyScope
        );
    }

    #[test]
    fn key_commitment_is_deterministic() {
        let key = test_key();
        let c1 = key.compute_commitment();
        let c2 = key.compute_commitment();
        assert_eq!(c1, c2);
    }

    #[test]
    fn different_keys_have_different_commitments() {
        let k1 = DisclosureKey::from_genesis_material(
            [0x11u8; 32], 100, 200,
            *b"QASH-TEST-DOMAIN-EU-AML-0000000\x00",
        );
        let k2 = DisclosureKey::from_genesis_material(
            [0x22u8; 32], 100, 200,
            *b"QASH-TEST-DOMAIN-EU-AML-0000000\x00",
        );
        assert_ne!(k1.compute_commitment(), k2.compute_commitment());
    }
}
