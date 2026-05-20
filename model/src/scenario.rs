//! Named scenario builders for simulation and testing.
//!
//! Each function returns `(initial_state, inputs)` that can be fed directly
//! to `run()`. Scenarios are deterministic and self-contained — no I/O.

use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::WINDOW_SIZE;
use qash_consensus::{EpochInput, EpochState, ValidatorUpdate, MAX_VALIDATORS};

use crate::genesis;

// ---------------------------------------------------------------------------
// near_halt — triggers a LyapunovViolation halt
// ---------------------------------------------------------------------------

/// Build the canonical near-halt scenario for `validator_count` validators.
///
/// Protocol path exercised:
/// 1. `WINDOW_SIZE + 1` idle epochs fill the convergence window with V ≈ 0.
/// 2. A final spike epoch sets D = 0.9 and C = 0.9 for every validator.
///    This drives V_convergence to ~3×10^6 (4 validators) against a
///    near-zero window minimum, so δ_window ≈ 3×10^6 >> ε = 20_000.
///    TH-3b concludes: halt is triggered.
///
/// Coq correspondence: lyapunov_stability.v — TH-3b (halt iff δ > ε).
pub fn near_halt(validator_count: u32) -> (EpochState, Vec<EpochInput>) {
    let state = genesis(validator_count, None);
    let mut inputs = Vec::with_capacity(WINDOW_SIZE + 2);

    // Warm-up: fill the convergence window with V = 0.
    for _ in 0..=WINDOW_SIZE {
        inputs.push(idle_input(validator_count));
    }

    // Spike: near-maximum divergence and conflict for all validators.
    let high = FixedPoint::from_raw(900_000); // 0.9 × SCALE
    let mut spike = EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: qash_consensus::envelope::PROTOCOL_VERSION_V1_1,
        update_count: validator_count,
    };
    for i in 0..validator_count as usize {
        spike.updates[i] = Some(ValidatorUpdate {
            divergence_new: high,
            conflict_new: high,
            slash_accum_new: FixedPoint::ZERO,
        });
    }
    inputs.push(spike);

    (state, inputs)
}

// ---------------------------------------------------------------------------
// steady_state — N epochs of zero-metric activity (convergence window = 0)
// ---------------------------------------------------------------------------

/// Build a steady-state scenario: `epochs` consecutive idle epochs.
///
/// V_convergence = 0 throughout. No halt is triggered.
/// Used to verify that idle validators produce a stable, non-halting trace.
pub fn steady_state(validator_count: u32, epochs: usize) -> (EpochState, Vec<EpochInput>) {
    let state = genesis(validator_count, None);
    let inputs = (0..epochs).map(|_| idle_input(validator_count)).collect();
    (state, inputs)
}

// ---------------------------------------------------------------------------
// single_spike — one high-metric epoch followed by recovery
// ---------------------------------------------------------------------------

/// Single spike then recovery: one bad epoch, then idle.
///
/// If the window has not filled before the spike, δ_window ≤ V_spike (no
/// min to subtract). If the spike is below ε, no halt occurs.
/// After the spike the next idle epoch reduces the average.
///
/// Useful for verifying that a single sub-ε spike does not cause a halt.
pub fn single_spike(
    validator_count: u32,
    spike_raw: i128,
    recovery_epochs: usize,
) -> (EpochState, Vec<EpochInput>) {
    let state = genesis(validator_count, None);
    let mut inputs = Vec::with_capacity(1 + recovery_epochs);

    let v = FixedPoint::from_raw(spike_raw);
    let mut spike = EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: qash_consensus::envelope::PROTOCOL_VERSION_V1_1,
        update_count: validator_count,
    };
    for i in 0..validator_count as usize {
        spike.updates[i] = Some(ValidatorUpdate {
            divergence_new: v,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
    }
    inputs.push(spike);

    for _ in 0..recovery_epochs {
        inputs.push(idle_input(validator_count));
    }

    (state, inputs)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn idle_input(validator_count: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: qash_consensus::envelope::PROTOCOL_VERSION_V1_1,
        update_count: validator_count,
    }
}
