//! Privacy boundary enforcement for Domain B receipt and key handling.
//!
//! This module provides:
//! - Compile-time and runtime assertions that `PublicTranscript` contains no PII
//! - Erasure-compatible key shredding via `ZeroizeOnDrop`-backed `ReceiptKey`
//!
//! Framing: key shredding makes decryption of erased receipts computationally
//! infeasible. This is one component of a broader erasure-handling design.
//! Compliance with Art. 17 GDPR requires the full design plus legal assessment.
//! Claim "GDPR-aligned design with erasure-compatible receipt handling" —
//! NOT "GDPR compliant."

pub mod erasure;
pub mod public_transcript;

pub use erasure::{
    compute_erasure_evidence_root_pair, process_erasure_request, shred_key, verify_erasure_evidence_root_pair,
    ErasureError, ErasureRequest, ErasureState, KeyStore, ReceiptKey, ShredKeyEvidence,
};
pub use public_transcript::assert_no_pii_surface;
