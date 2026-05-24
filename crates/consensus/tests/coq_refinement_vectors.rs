//! Coq refinement vectors for the initial correspondence surface.

use qash_consensus::encoding::{
    compute_leaf_index, encode_state_header, encode_validator_dynamic, STATE_HEADER_SIZE,
    VALIDATOR_DYNAMIC_SIZE,
};
use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::{evaluate, ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

#[path = "json_vectors.rs"]
mod json_vectors;

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn genesis(vc: u32) -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: vc,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        cascade_health: 0,
        state_root: [0u8; 32],
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
        causal_fingerprint: [0u8; 32],
    }
}
fn idle(vc: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: qash_consensus::envelope::PROTOCOL_VERSION_V1_1,
        update_count: vc,
    }
}

#[test]
fn coq_encoding_vectors_match_rust_encoding_identifiers() {
    let file_name = "proofs/model/encoding_vectors.json";
    let vectors = json_vectors::parse_vectors_file(
        file_name,
        include_str!("../../../proofs/model/encoding_vectors.json"),
    );

    let header = json_vectors::vector_by_id(&vectors, "ENC-HEADER-0", file_name);
    let mut header_out = [0u8; STATE_HEADER_SIZE as usize];
    encode_state_header(1, 4, 0, &[0u8; 32], &mut header_out);
    assert_eq!(
        hex_encode(&header_out),
        json_vectors::required_str(header, "encoded_hex", file_name, "ENC-HEADER-0")
    );

    let leaf = json_vectors::vector_by_id(&vectors, "ENC-LEAF-0", file_name);
    assert_eq!(
        hex_encode(&compute_leaf_index(7, 1, &[0u8; 32])),
        json_vectors::required_str(leaf, "encoded_hex", file_name, "ENC-LEAF-0")
    );

    let validator = json_vectors::vector_by_id(&vectors, "ENC-VALIDATOR-DYNAMIC-0", file_name);
    let mut validator_out = [0u8; VALIDATOR_DYNAMIC_SIZE as usize];
    encode_validator_dynamic(
        FixedPoint::from_raw(500_000),
        FixedPoint::from_raw(250_000),
        FixedPoint::from_raw(1_000),
        &mut validator_out,
    );
    assert_eq!(
        hex_encode(&validator_out),
        json_vectors::required_str(
            validator,
            "encoded_hex",
            file_name,
            "ENC-VALIDATOR-DYNAMIC-0"
        )
    );
}

#[test]
fn coq_lyapunov_transition_observations_match_advance_epoch() {
    let file_name = "proofs/model/transition_observations.json";
    let vectors = json_vectors::parse_vectors_file(
        file_name,
        include_str!("../../../proofs/model/transition_observations.json"),
    );

    let idle_vec = json_vectors::required_object(
        json_vectors::vector_by_id(&vectors, "LYAP-IDLE-4", file_name),
        "expect",
        file_name,
        "LYAP-IDLE-4",
    );
    let mut idle_state = genesis(4);
    let idle_result = advance_epoch(&mut idle_state, &idle(4), &[]).expect("idle advance");
    let (idle_filled, idle_window) = idle_state.convergence_window.raw_parts();
    assert_eq!(
        idle_state.epoch as i64,
        json_vectors::required_i64(idle_vec, "epoch", file_name, "LYAP-IDLE-4")
    );
    assert_eq!(
        idle_state.halt_reason as u8 as i64,
        json_vectors::required_i64(idle_vec, "halt_reason", file_name, "LYAP-IDLE-4")
    );
    assert_eq!(
        idle_result.lyapunov.v_convergence.raw() as i64,
        json_vectors::required_i64(idle_vec, "v_convergence", file_name, "LYAP-IDLE-4")
    );
    assert_eq!(
        idle_result.lyapunov.delta_window.raw() as i64,
        json_vectors::required_i64(idle_vec, "delta_window", file_name, "LYAP-IDLE-4")
    );
    assert_eq!(idle_filled, 1);
    assert_eq!(idle_window[0].raw(), 0);

    let halt_vec = json_vectors::required_object(
        json_vectors::vector_by_id(&vectors, "LYAP-HALT-4-900K", file_name),
        "expect",
        file_name,
        "LYAP-HALT-4-900K",
    );
    let mut halt_state = genesis(4);
    for _ in 0..WINDOW_SIZE {
        advance_epoch(&mut halt_state, &idle(4), &[]).expect("fill zero window");
    }
    let mut spike = idle(4);
    for slot in spike.updates.iter_mut().take(4) {
        *slot = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(900_000),
            conflict_new: FixedPoint::from_raw(900_000),
            slash_accum_new: FixedPoint::ZERO,
        });
    }
    assert_eq!(
        advance_epoch(&mut halt_state, &spike, &[]),
        Err(HaltReason::LyapunovViolation)
    );
    let (halt_filled, halt_window) = halt_state.convergence_window.raw_parts();
    assert_eq!(
        halt_state.epoch as i64,
        json_vectors::required_i64(halt_vec, "epoch", file_name, "LYAP-HALT-4-900K")
    );
    assert_eq!(
        halt_state.halt_reason as u8 as i64,
        json_vectors::required_i64(halt_vec, "halt_reason", file_name, "LYAP-HALT-4-900K")
    );
    assert_eq!(
        json_vectors::required_i64(halt_vec, "v_convergence", file_name, "LYAP-HALT-4-900K"),
        2_700_000
    );
    assert_eq!(
        json_vectors::required_i64(halt_vec, "delta_window", file_name, "LYAP-HALT-4-900K"),
        2_700_000
    );
    assert_eq!(halt_filled as usize, WINDOW_SIZE);
    assert_eq!(
        halt_window.iter().map(|v| v.raw()).collect::<Vec<_>>(),
        vec![0, 0, 0]
    );

    let epsilon_vec = json_vectors::required_object(
        json_vectors::vector_by_id(&vectors, "LYAP-EPSILON-1", file_name),
        "expect",
        file_name,
        "LYAP-EPSILON-1",
    );
    let mut epsilon_state = genesis(1);
    epsilon_state
        .convergence_window
        .push(FixedPoint::from_raw(100_000));
    epsilon_state
        .convergence_window
        .push(FixedPoint::from_raw(100_000));
    epsilon_state
        .convergence_window
        .push(FixedPoint::from_raw(100_000));
    let mut epsilon_input = idle(1);
    epsilon_input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(300_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    let epsilon_result =
        advance_epoch(&mut epsilon_state, &epsilon_input, &[]).expect("epsilon advance");
    let (epsilon_filled, epsilon_window) = epsilon_state.convergence_window.raw_parts();
    assert_eq!(
        epsilon_state.epoch as i64,
        json_vectors::required_i64(epsilon_vec, "epoch", file_name, "LYAP-EPSILON-1")
    );
    assert_eq!(
        epsilon_state.halt_reason as u8 as i64,
        json_vectors::required_i64(epsilon_vec, "halt_reason", file_name, "LYAP-EPSILON-1")
    );
    assert_eq!(
        epsilon_result.lyapunov.v_convergence.raw() as i64,
        json_vectors::required_i64(epsilon_vec, "v_convergence", file_name, "LYAP-EPSILON-1")
    );
    assert_eq!(
        epsilon_result.lyapunov.delta_window.raw() as i64,
        json_vectors::required_i64(epsilon_vec, "delta_window", file_name, "LYAP-EPSILON-1")
    );
    assert_eq!(epsilon_filled as usize, WINDOW_SIZE);
    assert_eq!(
        epsilon_window.iter().map(|v| v.raw()).collect::<Vec<_>>(),
        vec![120_000, 100_000, 100_000]
    );

    let invalid_vec = json_vectors::required_object(
        json_vectors::vector_by_id(&vectors, "LYAP-DECODE-INVALID-NEG-D", file_name),
        "expect",
        file_name,
        "LYAP-DECODE-INVALID-NEG-D",
    );
    let mut invalid_state = genesis(1);
    let mut invalid_input = idle(1);
    invalid_input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(-1),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    let invalid_eval = evaluate(
        &invalid_state.validators[..invalid_state.validator_count as usize],
        &invalid_state.convergence_window,
    )
    .expect("invalid baseline eval");
    assert_eq!(
        advance_epoch(&mut invalid_state, &invalid_input, &[]),
        Err(HaltReason::DecodeInvalid)
    );
    let (invalid_filled, _) = invalid_state.convergence_window.raw_parts();
    assert_eq!(
        invalid_state.epoch as i64,
        json_vectors::required_i64(invalid_vec, "epoch", file_name, "LYAP-DECODE-INVALID-NEG-D")
    );
    assert_eq!(
        invalid_state.halt_reason as u8 as i64,
        json_vectors::required_i64(
            invalid_vec,
            "halt_reason",
            file_name,
            "LYAP-DECODE-INVALID-NEG-D"
        )
    );
    assert_eq!(
        invalid_eval.v_convergence.raw() as i64,
        json_vectors::required_i64(
            invalid_vec,
            "v_convergence",
            file_name,
            "LYAP-DECODE-INVALID-NEG-D"
        )
    );
    assert_eq!(
        invalid_eval.delta_window.raw() as i64,
        json_vectors::required_i64(
            invalid_vec,
            "delta_window",
            file_name,
            "LYAP-DECODE-INVALID-NEG-D"
        )
    );
    assert_eq!(invalid_filled, 0);

    let absorb_vec = json_vectors::required_object(
        json_vectors::vector_by_id(&vectors, "LYAP-ABSORB-HALT", file_name),
        "expect",
        file_name,
        "LYAP-ABSORB-HALT",
    );
    let mut absorb_state = genesis(1);
    absorb_state.epoch = 7;
    absorb_state.halt_reason = HaltReason::LyapunovViolation;
    absorb_state
        .convergence_window
        .push(FixedPoint::from_raw(3));
    absorb_state
        .convergence_window
        .push(FixedPoint::from_raw(2));
    absorb_state
        .convergence_window
        .push(FixedPoint::from_raw(1));
    let mut absorb_input = idle(1);
    absorb_input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(300_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    let projected = [ValidatorMetrics {
        divergence: FixedPoint::from_raw(300_000),
        conflict: FixedPoint::ZERO,
        slash_accum: FixedPoint::ZERO,
    }];
    let absorb_eval =
        evaluate(&projected, &absorb_state.convergence_window).expect("absorbing eval");
    assert_eq!(
        advance_epoch(&mut absorb_state, &absorb_input, &[]),
        Err(HaltReason::LyapunovViolation)
    );
    let (absorb_filled, absorb_window) = absorb_state.convergence_window.raw_parts();
    assert_eq!(
        absorb_state.epoch as i64,
        json_vectors::required_i64(absorb_vec, "epoch", file_name, "LYAP-ABSORB-HALT")
    );
    assert_eq!(
        absorb_state.halt_reason as u8 as i64,
        json_vectors::required_i64(absorb_vec, "halt_reason", file_name, "LYAP-ABSORB-HALT")
    );
    assert_eq!(
        absorb_eval.v_convergence.raw() as i64,
        json_vectors::required_i64(absorb_vec, "v_convergence", file_name, "LYAP-ABSORB-HALT")
    );
    assert_eq!(
        absorb_eval.delta_window.raw() as i64,
        json_vectors::required_i64(absorb_vec, "delta_window", file_name, "LYAP-ABSORB-HALT")
    );
    assert_eq!(absorb_filled as usize, WINDOW_SIZE);
    assert_eq!(
        absorb_window.iter().map(|v| v.raw()).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}
