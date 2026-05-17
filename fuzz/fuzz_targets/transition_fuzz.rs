// Fuzz target: advance_epoch — Domain A state transition.
//
// Uses the `arbitrary` crate for structured input generation so honggfuzz
// explores the space of valid EpochState + EpochInput pairs efficiently
// rather than fighting fixed-size struct layouts.
//
// Invariants verified on every valid (non-rejected) input:
//   1. No panic
//   2. If Ok: epoch incremented by 1
//   3. If Ok: state_root is non-zero
//   4. If halted after: subsequent call returns the same HaltReason
//   5. If halted after: epoch and state_root are frozen (TH-6)
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
struct FuzzInput {
    divergence_raw: i32,
    conflict_raw: i32,
    slash_raw: u32,
    validator_count: u8,
    entropy_seed: [u8; 32],
    window_fill: u8,
}

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            let mut u = arbitrary::Unstructured::new(data);
            let fi = match FuzzInput::arbitrary(&mut u) {
                Ok(v) => v,
                Err(_) => return,
            };

            let vc = (fi.validator_count as u32 % 5).max(1); // 1..=4
            let scale = 1_000_000i128;

            // Clamp all metrics to [0, SCALE] so the input is structurally valid.
            // Cast to i128 before abs() — i32::MIN.abs() overflows i32 in debug mode.
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
                state_root: [0u8; 32],
            };

            // Optionally pre-fill the convergence window (exercises the δ_window path).
            let fill = (fi.window_fill as usize).min(3);
            for _ in 0..fill {
                state.convergence_window.push(FixedPoint::from_raw(d));
            }

            let mut input = EpochInput { updates: [None; MAX_VALIDATORS], update_count: vc };
            for i in 0..vc as usize {
                input.updates[i] = Some(ValidatorUpdate {
                    divergence_new: FixedPoint::from_raw(d),
                    conflict_new: FixedPoint::from_raw(c),
                    slash_accum_new: FixedPoint::from_raw(s),
                });
            }

            let prior_epoch = state.epoch;
            let result = advance_epoch(&mut state, &input, &[]);

            match result {
                Ok(_) => {
                    // Invariant 2: epoch must have advanced.
                    assert_eq!(state.epoch, prior_epoch + 1);
                    // Invariant 3: state_root must be non-zero.
                    assert_ne!(state.state_root, [0u8; 32]);
                }
                Err(halt) => {
                    // Invariant 4+5: halt is absorbing (TH-6).
                    assert_eq!(state.halt_reason, halt);
                    let frozen_epoch = state.epoch;
                    let frozen_root = state.state_root;

                    let idle = EpochInput { updates: [None; MAX_VALIDATORS], update_count: vc };
                    let r2 = advance_epoch(&mut state, &idle, &[]);
                    assert_eq!(r2, Err(halt));
                    assert_eq!(state.epoch, frozen_epoch);
                    assert_eq!(state.state_root, frozen_root);
                }
            }
        });
    }
}
