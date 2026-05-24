// Fuzz target: advance_epoch — Domain A state transition.
//
// Uses the `arbitrary` crate for structured input generation so honggfuzz
// explores the space of valid EpochState + EpochInput pairs efficiently
// rather than fighting fixed-size struct layouts.
//
// Two fuzz modes per invocation (selected by a leading byte in the input):
//
//   Mode 0 — Clamped: metrics ∈ [0, SCALE], validator_count 1–4.
//     Exercises the normal transition path and Lyapunov halt.
//
//   Mode 1 — Halt-trigger: metrics unclamped (full i64 range), validator_count
//     up to MAX_VALIDATORS. Targets the ArithmeticOverflow halt path and verifies
//     that no unclamped value causes a panic outside the absorbing-halt contract.
//
// Invariants verified on every invocation:
//   1. No panic
//   2. If Ok: epoch incremented by 1
//   3. If Ok: state_root is non-zero
//   4. If Err: halt is absorbing — subsequent call returns same HaltReason
//   5. If Err: epoch and state_root are frozen (TH-6)
//
// Run: cargo hfuzz run transition_fuzz  (from fuzz/)

use honggfuzz::fuzz;
use arbitrary::Arbitrary;
use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

#[derive(Arbitrary, Debug)]
struct ClampedInput {
    divergence_raw: i32,
    conflict_raw: i32,
    slash_raw: u32,
    validator_count: u8,
    entropy_seed: [u8; 32],
    window_fill: u8,
}

#[derive(Arbitrary, Debug)]
struct ExtremeInput {
    // Full i64 range — unclamped. Targets ArithmeticOverflow halt path.
    divergence_raw: i64,
    conflict_raw: i64,
    slash_raw: i64,
    validator_count: u16, // up to MAX_VALIDATORS
    entropy_seed: [u8; 32],
    window_fill: u8,
    window_value: i64,
}

fn run_and_verify(mut state: EpochState, input: EpochInput, vc: u32) {
    let prior_epoch = state.epoch;
    let result = advance_epoch(&mut state, input.into_effect(), &[]);

    match result {
        Ok(_) => {
            assert_eq!(state.epoch, prior_epoch + 1);
            assert_ne!(state.state_root, [0u8; 32]);
        }
        Err(halt) => {
            assert_eq!(state.halt_reason, halt);
            let frozen_epoch = state.epoch;
            let frozen_root = state.state_root;

            let idle = EpochInput::new(vc);
            let r2 = advance_epoch(&mut state, idle.into_effect(), &[]);
            assert_eq!(r2, Err(halt));
            assert_eq!(state.epoch, frozen_epoch);
            assert_eq!(state.state_root, frozen_root);
        }
    }
}

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            if data.is_empty() { return; }
            let mode = data[0] & 1;
            let rest = &data[1..];
            let mut u = arbitrary::Unstructured::new(rest);

            if mode == 0 {
                // ---- Mode 0: clamped inputs — normal + Lyapunov halt path ----
                let fi = match ClampedInput::arbitrary(&mut u) {
                    Ok(v) => v,
                    Err(_) => return,
                };

                let vc = (fi.validator_count as u32 % 5).max(1);
                let scale = 1_000_000i128;
                let d = (fi.divergence_raw as i128).abs().min(scale);
                let c = (fi.conflict_raw as i128).abs().min(scale);
                let s = (fi.slash_raw as i128).min(scale);

                let mut state = EpochState {
                    epoch: 0,
                    halt_reason: HaltReason::None,
                    entropy_seed: fi.entropy_seed,
                    validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
                    validator_count: vc,
                    convergence_window: ConvergenceWindow::new(),
                    nonces: [0u64; MAX_VALIDATORS],
                    validator_ids: [[0u8; 48]; MAX_VALIDATORS],
                    cascade_health: 0,
                    causal_fingerprint: [0u8; 32],
                    state_root: [0u8; 32],
                    receipt_root: [0u8; 32],
                    efb_root: [0u8; 32],
                };

                let fill = (fi.window_fill as usize).min(3);
                for _ in 0..fill {
                    state.convergence_window.push(FixedPoint::from_raw(d));
                }

                let mut input = EpochInput::new(vc);
                for i in 0..vc as usize {
                    input.updates[i] = Some(ValidatorUpdate {
                        divergence_new: FixedPoint::from_raw(d),
                        conflict_new: FixedPoint::from_raw(c),
                        slash_accum_new: FixedPoint::from_raw(s),
                    });
                }

                run_and_verify(state, input, vc);

            } else {
                // ---- Mode 1: extreme unclamped inputs — ArithmeticOverflow halt path ----
                let fi = match ExtremeInput::arbitrary(&mut u) {
                    Ok(v) => v,
                    Err(_) => return,
                };

                // Allow full MAX_VALIDATORS range to maximise Φ_convergence pressure.
                let vc = ((fi.validator_count as u32) % (MAX_VALIDATORS as u32 + 1)).max(1);

                // Keep values non-negative so the transition doesn't reject on range checks,
                // but otherwise unclamped — allows values up to i64::MAX which can overflow
                // the weighted sum when aggregated across many validators.
                let d = (fi.divergence_raw as i128).abs();
                let c = (fi.conflict_raw as i128).abs();
                let s = (fi.slash_raw as i128).abs();

                let mut state = EpochState {
                    epoch: 0,
                    halt_reason: HaltReason::None,
                    entropy_seed: fi.entropy_seed,
                    validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
                    validator_count: vc,
                    convergence_window: ConvergenceWindow::new(),
                    nonces: [0u64; MAX_VALIDATORS],
                    validator_ids: [[0u8; 48]; MAX_VALIDATORS],
                    cascade_health: 0,
                    causal_fingerprint: [0u8; 32],
                    state_root: [0u8; 32],
                    receipt_root: [0u8; 32],
                    efb_root: [0u8; 32],
                };

                // Pre-fill window with a large value to prime the δ_window halt trigger.
                let wv = (fi.window_value as i128).abs();
                let fill = (fi.window_fill as usize).min(3);
                for _ in 0..fill {
                    state.convergence_window.push(FixedPoint::from_raw(wv));
                }

                let mut input = EpochInput::new(vc);
                for i in 0..vc as usize {
                    input.updates[i] = Some(ValidatorUpdate {
                        divergence_new: FixedPoint::from_raw(d),
                        conflict_new: FixedPoint::from_raw(c),
                        slash_accum_new: FixedPoint::from_raw(s),
                    });
                }

                run_and_verify(state, input, vc);
            }
        });
    }
}
