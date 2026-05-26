use qash_consensus::envelope::PROTOCOL_VERSION_V1_1;
use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::hash::{h_domain, DomainTag};
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
use qash_consensus::transaction::{
    apply_all, prevalidate_all, TX0_WIRE_BYTES, TX_HEADER_BYTES, TX_TYPE_NOOP, TX_VERSION,
};
use qash_consensus::transition::{
    advance_epoch, encode_full_state_into, EpochInput, EpochState, HaltReason, ValidatorUpdate,
    FULL_STATE_MAX_BYTES, MAX_VALIDATORS,
};

fn state_with_validators(validator_count: u32) -> EpochState {
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
    for i in 0..validator_count as usize {
        validator_ids[i][0..8].copy_from_slice(&(i as u64 + 1).to_le_bytes());
    }

    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0x42; 32],
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

fn idle_input(validator_count: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        update_count: validator_count,
        protocol_version: PROTOCOL_VERSION_V1_1,
    }
}

fn max_validator_input() -> EpochInput {
    let mut input = idle_input(MAX_VALIDATORS as u32);
    for i in 0..MAX_VALIDATORS {
        input.updates[i] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(10),
            conflict_new: FixedPoint::from_raw(5),
            slash_accum_new: FixedPoint::from_raw(1),
        });
    }
    input
}

fn make_tx0_raw(author_id: [u8; 48], tx_sequence: u64, signature_byte: u8) -> [u8; TX0_WIRE_BYTES] {
    let mut raw = [0u8; TX0_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
    raw[4..12].copy_from_slice(&tx_sequence.to_le_bytes());
    raw[12..60].copy_from_slice(&author_id);
    raw[60..64].copy_from_slice(&0u32.to_le_bytes());
    raw[TX_HEADER_BYTES] = signature_byte;
    raw
}

#[test]
fn phase2r_tx_heavy_prevalidation_is_input_order_independent() {
    let state = state_with_validators(MAX_VALIDATORS as u32);
    let txs: Vec<[u8; TX0_WIRE_BYTES]> = state
        .validator_ids
        .iter()
        .take(MAX_VALIDATORS)
        .enumerate()
        .map(|(i, author)| make_tx0_raw(*author, 0, i as u8))
        .collect();

    let forward_refs: Vec<&[u8]> = txs.iter().map(|tx| tx.as_slice()).collect();
    let reverse_refs: Vec<&[u8]> = txs.iter().rev().map(|tx| tx.as_slice()).collect();

    let forward = prevalidate_all(&state, &forward_refs, MAX_VALIDATORS as u32).unwrap();
    let reverse = prevalidate_all(&state, &reverse_refs, MAX_VALIDATORS as u32).unwrap();

    assert_eq!(forward.applied_count, MAX_VALIDATORS as u32);
    assert_eq!(reverse.applied_count, MAX_VALIDATORS as u32);
    assert_eq!(forward.next_nonces, reverse.next_nonces);
}

#[test]
fn phase2r_tx_heavy_apply_all_matches_prevalidation_plan() {
    let mut state = state_with_validators(MAX_VALIDATORS as u32);
    let txs: Vec<[u8; TX0_WIRE_BYTES]> = state
        .validator_ids
        .iter()
        .take(MAX_VALIDATORS)
        .enumerate()
        .map(|(i, author)| make_tx0_raw(*author, 0, i as u8))
        .collect();
    let refs: Vec<&[u8]> = txs.iter().rev().map(|tx| tx.as_slice()).collect();
    let plan = prevalidate_all(&state, &refs, MAX_VALIDATORS as u32).unwrap();

    let applied = apply_all(&mut state, &refs, MAX_VALIDATORS as u32).unwrap();

    assert_eq!(applied, MAX_VALIDATORS as u32);
    assert_eq!(state.nonces, plan.next_nonces);
    assert!(state.nonces[..MAX_VALIDATORS]
        .iter()
        .all(|nonce| *nonce == 1));
}

#[test]
fn phase2r_max_validator_epoch_replay_is_repeatable() {
    let input = max_validator_input();
    let mut left = state_with_validators(MAX_VALIDATORS as u32);
    let mut right = state_with_validators(MAX_VALIDATORS as u32);

    let left_result = advance_epoch(&mut left, &input, &[]).unwrap();
    let right_result = advance_epoch(&mut right, &input, &[]).unwrap();

    assert_eq!(left_result.state_root, right_result.state_root);
    assert_eq!(
        left_result.public_transcript.receipt_root,
        right_result.public_transcript.receipt_root
    );
    assert_eq!(
        left_result.public_transcript.efb_root,
        right_result.public_transcript.efb_root
    );
}

#[test]
fn phase2r_state_root_commitment_matches_buffered_preimage() {
    let mut state = state_with_validators(4);
    let prior_root = state.state_root;
    let result = advance_epoch(&mut state, &idle_input(4), &[]).unwrap();

    let mut commitment_state = state;
    commitment_state.state_root = prior_root;

    let mut preimage = [0u8; FULL_STATE_MAX_BYTES];
    let preimage_len = encode_full_state_into(&commitment_state, &mut preimage);
    let recomputed = h_domain(DomainTag::StateRoot, &preimage[..preimage_len]);

    assert_eq!(result.state_root, recomputed);
}

/// Phase 2-R streaming preimage equivalence: advance_epoch (streaming path) must
/// produce the same state root as manually encoding to a buffer and hashing it.
/// Covers validator counts that trigger the sharding-roots branch (non-zero roots).
#[test]
fn phase2r_streaming_state_root_matches_buffered_for_varied_states() {
    for vc in [1u32, 4, 16, 128] {
        for epoch_steps in [0u32, 1, 5] {
            let mut state = state_with_validators(vc);
            for _ in 0..epoch_steps {
                advance_epoch(&mut state, &idle_input(vc), &[]).unwrap();
            }
            let prior_root = state.state_root;
            let result = advance_epoch(&mut state, &idle_input(vc), &[]).unwrap();

            let mut commitment_state = state;
            commitment_state.state_root = prior_root;

            let mut preimage = [0u8; FULL_STATE_MAX_BYTES];
            let preimage_len = encode_full_state_into(&commitment_state, &mut preimage);
            let buffered = h_domain(DomainTag::StateRoot, &preimage[..preimage_len]);

            assert_eq!(
                result.state_root, buffered,
                "streaming path must match buffered path for vc={vc} after {epoch_steps} steps"
            );
        }
    }
}

/// Phase 2-R single-pass admission parity: prevalidate_all with forward and
/// reversed input order must produce identical next_nonces (order-independence
/// of the single-pass path under the total-order sort).
#[test]
fn phase2r_single_pass_admission_is_order_independent_with_tx1() {
    use qash_consensus::fixed_point::FixedPoint;
    use qash_consensus::transaction::{prevalidate_all, TX1_WIRE_BYTES, TX_TYPE_SCORE_DECREMENT};

    let mut state = state_with_validators(4);
    // Give validator 1 some divergence so TX-1 can decrement it.
    state.validators[1].divergence = FixedPoint::from_raw(500_000);

    // TX-0 from slot 0, TX-1 from slot 2 targeting slot 1.
    let tx0 = make_tx0_raw(state.validator_ids[0], 0, 0);
    let tx1 = {
        let mut raw = [0u8; TX1_WIRE_BYTES];
        raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
        raw[2..4].copy_from_slice(&TX_TYPE_SCORE_DECREMENT.to_le_bytes());
        raw[4..12].copy_from_slice(&0u64.to_le_bytes()); // nonce 0
        raw[12..60].copy_from_slice(&state.validator_ids[2]);
        raw[60..64].copy_from_slice(&8u32.to_le_bytes()); // payload_len=8
        raw[64..68].copy_from_slice(&1u32.to_le_bytes()); // target_idx=1
        raw[68..72].copy_from_slice(&100u32.to_le_bytes()); // delta=100
        raw
    };

    let fwd = prevalidate_all(
        &state,
        &[tx0.as_slice(), tx1.as_slice()],
        MAX_VALIDATORS as u32,
    )
    .unwrap();
    let rev = prevalidate_all(
        &state,
        &[tx1.as_slice(), tx0.as_slice()],
        MAX_VALIDATORS as u32,
    )
    .unwrap();

    assert_eq!(fwd.applied_count, 2);
    assert_eq!(rev.applied_count, 2);
    assert_eq!(fwd.next_nonces, rev.next_nonces);
    assert_eq!(fwd.divergence_update_count, rev.divergence_update_count);
}
