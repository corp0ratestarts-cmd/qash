//! Public observation transcript — the only Domain A artifacts
//! visible to external passive observers.
//!
//! `RawTx`, `Tx` envelopes, `ValidatorMetrics`, and `ConvergenceWindow`
//! are intentionally absent: they are Domain B admission artifacts and
//! MUST NOT appear in `PublicTranscript`.
//!
//! Constructed ONLY by epoch advancement after Lyapunov validation and cascade
//! or EFB verification.  Callers in Domain A MUST NOT fabricate or modify
//! `PublicTranscript` instances outside the authorized construction path.
//! This struct is a read-only view of the epoch's public commitment surface.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicTranscript {
    pub state_root: [u8; 32],
    pub receipt_root: [u8; 32],
    pub efb_root: [u8; 32],
    pub epoch: u64,
    pub halt_flag: bool,
}
