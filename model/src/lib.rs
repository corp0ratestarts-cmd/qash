//! qash-model — canonical reference execution layer.
//!
//! # Position in the QASH layer stack
//!
//! ```text
//! docs/spec/   → normative law       (what the protocol INTENDS)
//! proofs/      → formal theorems     (what is PROVED about it)
//! model/       → this crate          (what it COMPUTES, reference impl)
//! crates/      → production runtime  (what is DEPLOYED)
//! ```
//!
//! Every public function cites its spec section (§) and, where applicable,
//! the Coq theorem or definition it corresponds to (see `coq_correspondence`).
//!
//! # Coq analogue
//! The `step` function corresponds to the `step` definition used in
//! `proofs/contractivity/lyapunov_stability.v` (TH-3a: δ ≤ ε → no halt).
//! The `run` function corresponds to the iterated application of `step`
//! over a sequence of inputs (TH-6: halted state is terminal).
#![no_std]
extern crate alloc;

use alloc::vec::Vec;

pub use qash_consensus::fixed_point::FixedPoint;
pub use qash_consensus::lyapunov::{ConvergenceWindow, WINDOW_SIZE};
pub use qash_consensus::{
    advance_epoch, EpochInput, EpochState, HaltReason, LyapunovEval, ValidatorMetrics,
    ValidatorUpdate, MAX_VALIDATORS,
};

pub mod coq_correspondence;
pub mod scenario;

// ---------------------------------------------------------------------------
// StepOutput — explicit output type for one epoch transition
// ---------------------------------------------------------------------------

/// Everything an observer needs to trace one epoch of the QASH protocol.
///
/// §E1 (00_execution_model.md): The public observation transcript for epoch t
/// is { state_root_t, receipt_root_t, epoch_t, halt_flag_t }. `StepOutput`
/// is the full internal record that also carries Lyapunov metrics for
/// simulation and analysis.
///
/// Coq analogue: output of `step(s, i)` in lyapunov_stability.v; the
/// `v_convergence` and `delta_window` fields correspond directly to
/// the Coq `v_validator` sum and `delta_window` definition.
#[derive(Debug, Clone, Copy)]
pub struct StepOutput {
    /// Epoch index AFTER this step. §1: epoch is a u64 counter, monotone.
    pub epoch: u64,
    /// SHA3-256 commitment to the full state. §2: state_root = h_domain(StateRoot, Encode(S_t)).
    pub state_root: [u8; 32],
    /// V_convergence = Σ_i (α·D_i + β·C_i). Coq: v_validator sum.
    pub v_convergence: FixedPoint,
    /// δ_window = V_convergence - min(window). TH-3b: halt iff δ > ε = 20_000.
    pub delta_window: FixedPoint,
    /// True if this step triggered halt. Coq: TH-3b conclusion.
    pub halt_triggered: bool,
    /// The halt code. §A6: halt_reason is monotone (None → non-None, never back).
    pub halt_reason: HaltReason,
}

// ---------------------------------------------------------------------------
// genesis — create the initial EpochState
// ---------------------------------------------------------------------------

/// Create the initial protocol state (S_0).
///
/// §A0 (02_transition_axioms.md): Genesis state has halt_reason = None and
/// all validator metrics = 0. §1: validator_ids are stable and fixed at genesis.
///
/// `validator_count` must be ≤ MAX_VALIDATORS (1024).
///
/// `ids` is optional:
/// - `Some(slice)`: use the provided 48-byte identities (production use).
/// - `None`: auto-generate sequential ids where `id[i][0] = i + 1`
///   (simulation only; not suitable for deployment).
pub fn genesis(validator_count: u32, ids: Option<&[[u8; 48]]>) -> EpochState {
    assert!(
        validator_count as usize <= MAX_VALIDATORS,
        "validator_count ({}) exceeds MAX_VALIDATORS ({})",
        validator_count,
        MAX_VALIDATORS
    );
    let mut state = EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        cascade_health: 0,
        state_root: [0u8; 32],
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
        causal_fingerprint: [0u8; 32],
    };
    match ids {
        Some(explicit) => {
            for (i, id) in explicit.iter().enumerate().take(validator_count as usize) {
                state.validator_ids[i] = *id;
            }
        }
        None => {
            for i in 0..validator_count as usize {
                state.validator_ids[i][0] = (i as u8).wrapping_add(1);
            }
        }
    }
    state
}

// ---------------------------------------------------------------------------
// step — one epoch transition with explicit output capture
// ---------------------------------------------------------------------------

/// Execute one epoch transition and return the observation record.
///
/// This is T(S_t, I_t) from §E1 (00_execution_model.md) with explicit output
/// capture. The state is mutated in-place (§A9: atomic commit).
///
/// On halt (§A6):
/// - The state's `halt_reason` is set by `advance_epoch`.
/// - Subsequent calls to `step` on a halted state are absorbed: the same
///   `StepOutput` is returned with `halt_triggered = true`.
///
/// Coq analogue: `step` in lyapunov_stability.v. TH-3a guarantees no halt
/// when δ_window ≤ ε; TH-3b identifies the halt condition; TH-6 proves the
/// halted state is terminal.
///
/// No transactions are submitted (raw_txs = []). Pass transactions directly
/// to `advance_epoch` if needed.
pub fn step(state: &mut EpochState, input: &EpochInput) -> StepOutput {
    // §A6 halt monotonicity: halted state is absorbing — return immediately.
    if state.is_halted() {
        return StepOutput {
            epoch: state.epoch,
            state_root: state.state_root,
            v_convergence: FixedPoint::ZERO,
            delta_window: FixedPoint::ZERO,
            halt_triggered: true,
            halt_reason: state.halt_reason,
        };
    }

    match advance_epoch(state, input, &[]) {
        Ok(result) => StepOutput {
            epoch: state.epoch,
            state_root: result.state_root,
            v_convergence: result.lyapunov.v_convergence,
            delta_window: result.lyapunov.delta_window,
            halt_triggered: false,
            halt_reason: HaltReason::None,
        },
        Err(reason) => StepOutput {
            epoch: state.epoch,
            state_root: state.state_root,
            v_convergence: FixedPoint::ZERO,
            delta_window: FixedPoint::ZERO,
            halt_triggered: true,
            halt_reason: reason,
        },
    }
}

// ---------------------------------------------------------------------------
// run — multi-epoch executor
// ---------------------------------------------------------------------------

/// Run a sequence of epoch inputs against a state, collecting one `StepOutput`
/// per epoch.
///
/// Terminates at the first halt (§A6: absorbing) and returns all outputs up to
/// and including the halt epoch. If no inputs cause a halt, all outputs are
/// returned.
///
/// §A1 (determinism): `run` is deterministic — identical `(state, inputs)` always
/// produces identical `Vec<StepOutput>`. Any nondeterminism in `advance_epoch`
/// would be a Domain A violation (AX-1/AX-2 axioms).
///
/// §A6 (halt monotonicity): `run` always terminates because:
/// 1. Halted states produce no further state mutations.
/// 2. The input slice is finite.
pub fn run(state: &mut EpochState, inputs: &[EpochInput]) -> Vec<StepOutput> {
    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        let o = step(state, input);
        let halted = o.halt_triggered;
        out.push(o);
        if halted {
            break;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    fn idle_input(vc: u32) -> EpochInput {
        EpochInput::new(vc)
    }

    /// §A1: step is deterministic — same state + input → same output.
    #[test]
    fn step_is_deterministic() {
        let input = idle_input(4);
        let mut s1 = genesis(4, None);
        let mut s2 = genesis(4, None);
        let o1 = step(&mut s1, &input);
        let o2 = step(&mut s2, &input);
        assert_eq!(o1.epoch, o2.epoch);
        assert_eq!(o1.state_root, o2.state_root);
        assert_eq!(o1.halt_reason as u8, o2.halt_reason as u8);
    }

    /// §A6: step on a halted state returns the same halt_reason (absorbing).
    #[test]
    fn step_absorbs_on_halt() {
        let mut state = genesis(4, None);
        // Force a halt via update_count mismatch.
        let bad_input = EpochInput::new(99);
        let o1 = step(&mut state, &bad_input);
        assert!(o1.halt_triggered);
        let reason = o1.halt_reason;
        // Now any further step returns the same halt.
        let o2 = step(&mut state, &idle_input(4));
        assert!(o2.halt_triggered);
        assert_eq!(o2.halt_reason as u8, reason as u8);
    }

    /// run() stops at first halt and does not process further inputs.
    #[test]
    fn run_stops_at_halt() {
        let mut state = genesis(4, None);
        // First input is bad (halt), rest would be valid.
        let bad = EpochInput::new(99);
        let inputs = [bad, idle_input(4), idle_input(4)];
        let out = run(&mut state, &inputs);
        // Only 1 output (the halt), not 3.
        assert_eq!(out.len(), 1);
        assert!(out[0].halt_triggered);
    }

    /// genesis() with None ids auto-assigns sequential identities.
    #[test]
    fn genesis_auto_ids_sequential() {
        let state = genesis(4, None);
        for i in 0..4usize {
            assert_eq!(
                state.validator_ids[i][0],
                (i as u8) + 1,
                "id[{}][0] must be {}",
                i,
                i + 1
            );
            // All other bytes must be zero.
            assert_eq!(&state.validator_ids[i][1..], &[0u8; 47][..]);
        }
    }

    /// step() advances epoch by exactly 1 on each successful call.
    #[test]
    fn step_advances_epoch() {
        let mut state = genesis(4, None);
        let input = idle_input(4);
        for expected in 1u64..=5 {
            let o = step(&mut state, &input);
            assert!(!o.halt_triggered);
            assert_eq!(o.epoch, expected);
        }
    }

    /// Idle inputs keep V_convergence = 0 (all metrics zero).
    #[test]
    fn idle_trace_has_zero_v_convergence() {
        let mut state = genesis(4, None);
        let inputs: Vec<EpochInput> = (0..5).map(|_| idle_input(4)).collect();
        let out = run(&mut state, &inputs);
        for o in &out {
            assert!(!o.halt_triggered);
            assert_eq!(o.v_convergence.raw(), 0, "idle epoch must have V=0");
        }
    }

    /// LyapunovViolation halt is triggered after window fills and a spike occurs.
    /// TH-3b: halt iff δ_window > ε = 20_000.
    #[test]
    fn near_halt_scenario_triggers_lyapunov_halt() {
        let (mut state, inputs) = scenario::near_halt(4);
        let out = run(&mut state, &inputs);
        let last = out.last().expect("must have at least one output");
        assert!(last.halt_triggered, "near_halt must trigger halt");
        assert_eq!(last.halt_reason as u8, HaltReason::LyapunovViolation as u8);
    }
}
