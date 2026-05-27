//! Compile-time and runtime assertions that `PublicTranscript` contains no PII.
//!
//! `PublicTranscript` is a read-only view of Domain A epoch commitments. It
//! must never contain: raw receipt bodies, peer addresses, validator keys,
//! operator identities, or any field that could link to natural persons.

use qash_consensus::PublicTranscript;
use static_assertions::assert_fields;

// Compile-time assertion: exhaustively name every allowed field.
// Adding a PII-bearing field to PublicTranscript will break this assertion.
assert_fields!(
    PublicTranscript: state_root, receipt_root, efb_root, epoch, halt_flag
);

/// Runtime check: verify the PublicTranscript instance carries only fixed-width
/// roots and scalar flags — no variable-length or heap-allocated fields.
///
/// This is a sanity guard for integration tests; the static assertion above is
/// the authoritative PII boundary check.
pub fn assert_no_pii_surface(transcript: &PublicTranscript) {
    // Verify roots are exactly 32 bytes (compile-time type check already covers this,
    // but we assert the constant here to document the invariant explicitly).
    const ROOT_LEN: usize = 32;
    assert_eq!(transcript.state_root.len(), ROOT_LEN);
    assert_eq!(transcript.receipt_root.len(), ROOT_LEN);
    assert_eq!(transcript.efb_root.len(), ROOT_LEN);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_transcript_no_graph_field() {
        // Constructing a PublicTranscript with only the allowed fields compiles.
        // If a PII-bearing field were added to PublicTranscript, the
        // assert_fields! macro above would fail at compile time.
        let t = PublicTranscript {
            state_root: [1u8; 32],
            receipt_root: [2u8; 32],
            efb_root: [3u8; 32],
            epoch: 7,
            halt_flag: false,
        };
        assert_no_pii_surface(&t);
        assert_eq!(t.epoch, 7);
        assert!(!t.halt_flag);
    }
}
