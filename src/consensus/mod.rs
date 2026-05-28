//! Hosted consensus runner — Domain B orchestration layer.
//!
//! `Module` wraps `qash_model` (the canonical reference execution layer) and
//! `SoftwareHashMerkleAttestation` to provide a single, self-contained API for
//! the hosted binary. All consensus computation is delegated to Domain A
//! (`qash_consensus` / `qash_model`); this module only manages lifecycle,
//! provides read access to state, and bridges to Domain B attestation.
//!
//! Domain B boundary: attestation quotes produced here must never flow into
//! Domain A state transitions.

pub mod compat;

use qash_model::{EpochInput, EpochState, HaltReason, StepOutput};

use crate::hardware::{
    AttestationGate, AttestationGateError, AttestationQuote, SoftwareHashMerkleAttestation,
};

/// Hosted consensus runner.
///
/// Owns the protocol's `EpochState` and exposes a step-by-step interface
/// for advancing the consensus protocol. Attestation quotes are available
/// on demand via the embedded `SoftwareHashMerkleAttestation` gate.
pub struct Module {
    state: EpochState,
    attestation: SoftwareHashMerkleAttestation,
    step_count: u64,
}

impl Module {
    /// Initialise the hosted consensus runner with `validator_count` genesis validators.
    ///
    /// Validator identities are auto-generated (simulation mode). For production,
    /// use `Module::with_ids`.
    pub fn new(validator_count: u32) -> Self {
        Self {
            state: qash_model::genesis(validator_count, None),
            attestation: SoftwareHashMerkleAttestation::new(),
            step_count: 0,
        }
    }

    /// Initialise with explicit 48-byte validator identities.
    pub fn with_ids(ids: &[[u8; 48]]) -> Self {
        Self {
            state: qash_model::genesis(ids.len() as u32, Some(ids)),
            attestation: SoftwareHashMerkleAttestation::new(),
            step_count: 0,
        }
    }

    /// Execute one epoch transition and return the observation record.
    ///
    /// If the runner is already halted, returns the halted `StepOutput`
    /// without modifying state (halt monotonicity — §A6).
    pub fn advance(&mut self, input: &EpochInput) -> StepOutput {
        let out = qash_model::step(&mut self.state, input);
        self.step_count = self.step_count.saturating_add(1);
        out
    }

    /// Read-only view of the current epoch state.
    pub fn state(&self) -> &EpochState {
        &self.state
    }

    /// Current epoch index.
    pub fn epoch(&self) -> u64 {
        self.state.epoch
    }

    /// Number of epoch steps executed since creation (saturates at u64::MAX).
    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    /// True if the protocol has entered an absorbing halt (§A6).
    pub fn is_halted(&self) -> bool {
        self.state.is_halted()
    }

    /// The halt reason, or `HaltReason::None` if the protocol is running.
    pub fn halt_reason(&self) -> HaltReason {
        self.state.halt_reason
    }

    /// SHA3-256 commitment to the current full state.
    pub fn state_root(&self) -> [u8; 32] {
        self.state.state_root
    }

    /// Produce a Domain B attestation quote bound to `nonce`.
    ///
    /// The quote commits to the genesis parameter hash and the software
    /// platform identity. It does NOT commit to the current epoch state —
    /// use `state_root()` for that. Never feed this quote into Domain A
    /// state transitions.
    pub fn attestation_quote(
        &self,
        nonce: &[u8; 32],
    ) -> Result<AttestationQuote, AttestationGateError> {
        self.attestation.generate_quote(nonce)
    }

    /// Verify a previously generated attestation quote.
    pub fn verify_attestation(&self, quote: &AttestationQuote) -> Result<(), AttestationGateError> {
        self.attestation.verify_quote(quote)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qash_consensus::EpochInput;

    fn idle_input(validator_count: u32) -> EpochInput {
        EpochInput::new(validator_count)
    }

    #[test]
    fn new_module_starts_at_epoch_zero_not_halted() {
        let m = Module::new(4);
        assert_eq!(m.epoch(), 0);
        assert!(!m.is_halted());
        assert_eq!(m.halt_reason(), HaltReason::None);
        assert_eq!(m.step_count(), 0);
    }

    #[test]
    fn advance_increments_epoch_and_step_count() {
        let mut m = Module::new(4);
        let out = m.advance(&idle_input(4));
        assert_eq!(out.epoch, 1);
        assert_eq!(m.epoch(), 1);
        assert_eq!(m.step_count(), 1);
        assert!(!m.is_halted());
    }

    #[test]
    fn state_root_changes_across_epochs() {
        let mut m = Module::new(4);
        let root0 = m.state_root();
        m.advance(&idle_input(4));
        let root1 = m.state_root();
        // Non-trivial advance should change state root
        assert_ne!(root0, root1);
    }

    #[test]
    fn with_ids_uses_provided_identities() {
        let ids: Vec<[u8; 48]> = (1u8..=4).map(|b| [b; 48]).collect();
        let m = Module::with_ids(&ids);
        assert_eq!(m.state().validator_count, 4);
        assert_eq!(&m.state().validator_ids[0], &[1u8; 48]);
        assert_eq!(&m.state().validator_ids[3], &[4u8; 48]);
    }

    #[test]
    fn attestation_roundtrip() {
        let m = Module::new(4);
        let nonce = [0x42u8; 32];
        let quote = m.attestation_quote(&nonce).unwrap();
        m.verify_attestation(&quote).unwrap();
    }

    #[test]
    fn attestation_quote_is_deterministic_for_same_nonce() {
        let m = Module::new(4);
        let nonce = [0u8; 32];
        let q1 = m.attestation_quote(&nonce).unwrap();
        let q2 = m.attestation_quote(&nonce).unwrap();
        assert_eq!(q1, q2);
    }

    #[test]
    fn different_nonces_produce_different_quotes() {
        let m = Module::new(4);
        let q1 = m.attestation_quote(&[0u8; 32]).unwrap();
        let q2 = m.attestation_quote(&[1u8; 32]).unwrap();
        assert_ne!(q1, q2);
    }

    #[test]
    fn halted_module_returns_absorbing_state() {
        let mut m = Module::new(4);
        // Run until halted or 1000 epochs (safety bound)
        for _ in 0..1000 {
            if m.is_halted() {
                break;
            }
            m.advance(&idle_input(4));
        }
        if m.is_halted() {
            // Further advances must not change epoch or root
            let root_before = m.state_root();
            let epoch_before = m.epoch();
            let out = m.advance(&idle_input(4));
            assert_eq!(out.epoch, epoch_before);
            assert_eq!(m.state_root(), root_before);
            assert!(out.halt_triggered);
        }
        // If it never halted in 1000 epochs, that's also valid (steady state)
    }
}
