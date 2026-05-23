//! Griefing Analysis Suite
//!
//! Quantifies the economic cost and recovery duration for consensus halts
//! triggered by adversarial divergence (H1) and slash accumulation (H7).

use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::ConvergenceWindow;
use qash_consensus::lyapunov::ValidatorMetrics;
use qash_consensus::lyapunov::WINDOW_SIZE;
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

fn init_state(vc: u32) -> EpochState {
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
    for i in 0..vc as usize {
        validator_ids[i][0..8].copy_from_slice(&(i as u64 + 1).to_le_bytes());
    }
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: vc,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids,
        cascade_health: 0,
        state_root: [0u8; 32],
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
        causal_fingerprint: [0u8; 32],
    }
}

fn idle_input(vc: u32) -> EpochInput {
    let mut input = EpochInput::new(vc);
    for i in 0..vc as usize {
        input.updates[i] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
    }
    input
}

fn spike_input(vc: u32, d_raw: i128) -> EpochInput {
    let mut input = EpochInput::new(vc);
    for i in 0..vc as usize {
        input.updates[i] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(d_raw),
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
    }
    input
}

#[test]
fn analysis_recovery_duration_after_epsilon_spike() {
    let vc = 1u32;
    let mut s = init_state(vc);
    // 1. Fill window with idle (V=0)
    for _ in 0..WINDOW_SIZE {
        advance_epoch(&mut s, &idle_input(vc), &[]).unwrap();
    }
    // 2. Spike at 50k (V=20k=EPSILON)
    advance_epoch(&mut s, &spike_input(vc, 50_000), &[]).unwrap();

    // 3. Count how many idles until 20k is out of window
    let mut recovery_epochs = 0;
    // Count how many idles until the 20k spike is completely out of the window.
    // The spike is out of the window when all values in the window are 0.
    while s
        .convergence_window
        .raw_parts()
        .1
        .iter()
        .any(|v| v.raw() > 0)
    {
        advance_epoch(&mut s, &idle_input(vc), &[]).unwrap();
        recovery_epochs += 1;
        if recovery_epochs > 10 {
            break;
        }
    }
    // E1: [20k, 0, 0] -> has spike
    // E2: [0, 20k, 0] -> has spike
    // E3: [0, 0, 20k] -> has spike
    // E4: [0, 0, 0]   -> cleared
    // So it takes exactly WINDOW_SIZE (3) epochs of idling to clear the spike.
    assert_eq!(recovery_epochs, 3);
}

#[test]
fn analysis_oscillation_attack_efficiency() {
    let vc = 1u32;
    let mut s = init_state(vc);
    // Spike every 2 epochs (WINDOW_SIZE-1) to keep min=0
    for _ in 0..5 {
        advance_epoch(&mut s, &spike_input(vc, 50_000), &[]).unwrap();
        for _ in 0..WINDOW_SIZE - 2 {
            advance_epoch(&mut s, &idle_input(vc), &[]).unwrap();
        }
        assert_eq!(s.convergence_window.min_value().raw(), 0);
    }
}

#[test]
fn analysis_h7_accumulation_limit() {
    let vc = 1u32;
    let mut s = init_state(vc);
    let slash_increment = 500_000_000;
    for i in 1..=3 {
        let mut input = EpochInput::new(vc);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::from_raw(i * slash_increment),
        });
        advance_epoch(&mut s, &input, &[]).unwrap();
    }
    let mut input = EpochInput::new(vc);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::ZERO,
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::from_raw(2_000_000_000),
    });
    let res = advance_epoch(&mut s, &input, &[]);
    assert_eq!(res, Err(HaltReason::PhiSafetyViolation));
    assert_eq!(s.halt_reason, HaltReason::PhiSafetyViolation);
}
