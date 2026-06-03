//! QASH Regulated Profile — Domain B scaffolding.
//!
//! Implements the Class IV observer scaffolding defined in §P4 of
//! `docs/spec/09_privacy_model.md` and §19.1 of `docs/spec/19_profile_taxonomy.md`.
//!
//! # Profile constraints (normative)
//!
//! - This module is Domain B ONLY. Nothing here may cross into Domain A.
//! - Class IV access requires a genesis-authorised disclosure key.
//! - Disclosure is epoch-scoped and non-retroactive (forward secrecy preserved).
//! - Every disclosure operation requires a valid `LawfulBasis`.
//! - This module MUST NOT appear in `corp0ratestarts-cmd/pure-qash`.
//!   The pure-qash absence guards explicitly reject `regulated`, `class_iv`,
//!   `disclosure_key`, and `lawful_basis` identifiers.

pub mod disclosure;
pub mod receipt;

pub use disclosure::{
    DisclosureDomain, DisclosureKey, DisclosureRequest, DisclosureRequestError,
    validate_disclosure_request,
};
pub use receipt::{RegulatedReceiptDecrypt, RegulatedDecryptError};

use zeroize::Zeroize;

// ── Observer class taxonomy ───────────────────────────────────────────────────

/// Observer classes for QASH (§P4a of `09_privacy_model.md`).
///
/// Class IV is only present in the Regulated Profile.
/// Pure QASH Core uses Class I–III only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserverClass {
    /// Class I — Public observer (unauthenticated).
    /// Sees: epoch, state_root, receipt_root, efb_root, halt_flag.
    ClassI,
    /// Class II — Authorized validator (Domain A + B, TEE/OEM-protected).
    /// Sees: own slot, aggregated divergence, blinded opcodes in own shard.
    ClassII,
    /// Class III — Receipt holder (scoped Domain B disclosure).
    /// Sees: own receipts with epoch viewing key.
    ClassIII,
    /// Class IV — Regulatory authority (genesis-authorised disclosure domain).
    /// Sees: disclosed receipts within epoch scope, with lawful basis.
    #[cfg(feature = "regulated")]
    ClassIV,
}

// ── Lawful-basis declarations ─────────────────────────────────────────────────

/// Lawful basis for a Class IV disclosure request (§P4a).
///
/// Regulated Profile only — not present in Pure QASH Core.
#[cfg(feature = "regulated")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LawfulBasis {
    /// GDPR Article 6(1)(c) — compliance with a legal obligation.
    GdprArt6LegalObligation,
    /// GDPR Article 6(1)(e) — public interest or official authority.
    GdprArt6PublicTask,
    /// GDPR Article 9(2)(g) — substantial public interest.
    GdprArt9SubstantialPublicInterest,
    /// National law equivalent (must supply jurisdiction code and citation).
    NationalLawEquivalent {
        /// ISO 3166-1 alpha-2 jurisdiction code (e.g. "US", "DE", "GB").
        jurisdiction: [u8; 2],
        /// Opaque citation bytes (law reference, statute number, etc.).
        citation_hash: [u8; 32],
    },
}

#[cfg(feature = "regulated")]
impl Zeroize for LawfulBasis {
    fn zeroize(&mut self) {
        if let LawfulBasis::NationalLawEquivalent { jurisdiction, citation_hash } = self {
            jurisdiction.zeroize();
            citation_hash.zeroize();
        }
    }
}

// ── Re-export for convenience ─────────────────────────────────────────────────

#[cfg(feature = "regulated")]
pub use LawfulBasis as ClassIVLawfulBasis;
