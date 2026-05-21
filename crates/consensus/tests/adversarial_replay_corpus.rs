//! Adversarial replay corpus for TH-3/TH-7 closure.
//!
//! These cases pin accept/reject behavior under malformed and edge replay
//! sequences. The platform-determinism workflow runs consensus tests across
//! Tier A ISAs, so this corpus becomes cross-ISA evidence.

use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::transaction::{
    TX1_PAYLOAD_BYTES, TX1_WIRE_BYTES, TX_HEADER_BYTES, TX_TYPE_SCORE_DECREMENT, TX_VERSION,
};
use qash_consensus::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

fn genesis(validator_count: u32) -> EpochState {
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
    for i in 0..validator_count as usize {
        validator_ids[i][0] = i as u8 + 1;
    }
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count,
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

fn idle(validator_count: u32) -> EpochInput {
    EpochInput::new(validator_count)
}

fn tx1(author_id: [u8; 48], target_idx: u32, delta: u32) -> [u8; TX1_WIRE_BYTES] {
    let mut raw = [0u8; TX1_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_SCORE_DECREMENT.to_le_bytes());
    raw[12..60].copy_from_slice(&author_id);
    raw[60..64].copy_from_slice(&(TX1_PAYLOAD_BYTES as u32).to_le_bytes());
    raw[64..68].copy_from_slice(&target_idx.to_le_bytes());
    raw[68..72].copy_from_slice(&delta.to_le_bytes());
    raw[TX_HEADER_BYTES] = 1;
    raw
}

#[test]
fn adversarial_replay_corpus_decode_invalid_is_deterministic() {
    let mut a = genesis(2);
    let mut b = genesis(2);
    let mut bad = idle(2);
    bad.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(-1),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });

    assert_eq!(
        advance_epoch(&mut a, &bad, &[]),
        Err(HaltReason::DecodeInvalid)
    );
    assert_eq!(
        advance_epoch(&mut b, &bad, &[]),
        Err(HaltReason::DecodeInvalid)
    );
    assert_eq!(a.epoch, b.epoch);
    assert_eq!(a.halt_reason, b.halt_reason);
    assert_eq!(a.state_root, b.state_root);
}

#[test]
fn adversarial_replay_corpus_epsilon_boundary_is_accepted() {
    let mut state = genesis(1);
    state.validators[0].divergence = FixedPoint::from_raw(250_000);
    for _ in 0..WINDOW_SIZE {
        state.convergence_window.push(FixedPoint::from_raw(100_000));
    }
    let mut input = idle(1);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(300_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });

    let result = advance_epoch(&mut state, &input, &[]).expect("delta == epsilon accepted");

    assert_eq!(state.halt_reason, HaltReason::None);
    assert_eq!(result.lyapunov.delta_window.raw(), 20_000);
    assert_eq!(result.lyapunov.v_convergence.raw(), 120_000);
}

#[test]
fn adversarial_replay_corpus_tx1_valid_and_invalid_paths_are_deterministic() {
    let mut a = genesis(2);
    let mut b = genesis(2);
    a.validators[1].divergence = FixedPoint::from_raw(100);
    b.validators[1].divergence = FixedPoint::from_raw(100);
    let invalid = tx1(a.validator_ids[0], 1, 200);
    let valid = tx1(a.validator_ids[0], 1, 40);

    let result_a = advance_epoch(&mut a, &idle(2), &[invalid.as_slice(), valid.as_slice()])
        .expect("epoch accepts with one valid TX-1");
    let result_b = advance_epoch(&mut b, &idle(2), &[valid.as_slice(), invalid.as_slice()])
        .expect("epoch accepts with one valid TX-1");

    assert_eq!(a.validators[1].divergence.raw(), 60);
    assert_eq!(b.validators[1].divergence.raw(), 60);
    assert_eq!(a.nonces[0], 1);
    assert_eq!(b.nonces[0], 1);
    assert_eq!(a.state_root, b.state_root);
    assert_eq!(result_a.public_transcript, result_b.public_transcript);
}
