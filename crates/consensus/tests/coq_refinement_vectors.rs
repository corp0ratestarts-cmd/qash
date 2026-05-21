//! Coq refinement vectors for the initial correspondence surface.
//!
//! The values in `proofs/model/encoding_vectors.json` and
//! `proofs/model/transition_observations.json` are backed by checked Examples in
//! `proofs/model/Model.v` and exercised here against the Rust identifiers with
//! matching names.

use qash_consensus::encoding::{
    compute_leaf_index, encode_state_header, encode_validator_dynamic, STATE_HEADER_SIZE,
    VALIDATOR_DYNAMIC_SIZE,
};
use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::{evaluate, ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn extract_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\": \"");
    let start = json.find(&needle)? + needle.len();
    let end = json[start..].find('"')? + start;
    Some(&json[start..end])
}

fn vector_block<'a>(json: &'a str, id: &str) -> &'a str {
    let id_needle = format!("\"id\": \"{id}\"");
    let id_pos = json
        .find(&id_needle)
        .unwrap_or_else(|| panic!("missing vector {id}"));
    let start = json[..id_pos].rfind('{').unwrap_or(id_pos);
    let rest = &json[id_pos..];
    let next = rest
        .find("\n    },")
        .map(|offset| id_pos + offset + 6)
        .unwrap_or(json.len());
    &json[start..next]
}

fn extract_i64(json: &str, key: &str) -> i64 {
    let needle = format!("\"{key}\": ");
    let start = json
        .find(&needle)
        .unwrap_or_else(|| panic!("missing key {key}"))
        + needle.len();
    let rest = &json[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end]
        .parse()
        .unwrap_or_else(|_| panic!("invalid integer for {key}"))
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
    let json = include_str!("../../../proofs/model/encoding_vectors.json");
    assert!(json.contains("proofs/model/Model.v::encode_state_header_tv0_checked"));

    let header = vector_block(json, "ENC-HEADER-0");
    let mut header_out = [0u8; STATE_HEADER_SIZE as usize];
    encode_state_header(1, 4, 0, &[0u8; 32], &mut header_out);
    assert_eq!(
        hex_encode(&header_out),
        extract_str(header, "encoded_hex").expect("header encoded_hex")
    );

    let leaf = vector_block(json, "ENC-LEAF-0");
    assert_eq!(
        hex_encode(&compute_leaf_index(7, 1, &[0u8; 32])),
        extract_str(leaf, "encoded_hex").expect("leaf encoded_hex")
    );

    let validator = vector_block(json, "ENC-VALIDATOR-DYNAMIC-0");
    let mut validator_out = [0u8; VALIDATOR_DYNAMIC_SIZE as usize];
    encode_validator_dynamic(
        FixedPoint::from_raw(500_000),
        FixedPoint::from_raw(250_000),
        FixedPoint::from_raw(1_000),
        &mut validator_out,
    );
    assert_eq!(
        hex_encode(&validator_out),
        extract_str(validator, "encoded_hex").expect("validator encoded_hex")
    );
}

#[test]
fn coq_lyapunov_transition_observations_match_advance_epoch() {
    let json = include_str!("../../../proofs/model/transition_observations.json");

    let idle_vec = vector_block(json, "LYAP-IDLE-4");
    let mut idle_state = genesis(4);
    let idle_result = advance_epoch(&mut idle_state, &idle(4), &[]).expect("idle advance");
    let (idle_filled, idle_window) = idle_state.convergence_window.raw_parts();
    assert_eq!(idle_state.epoch as i64, extract_i64(idle_vec, "epoch"));
    assert_eq!(
        idle_state.halt_reason as u8 as i64,
        extract_i64(idle_vec, "halt_reason")
    );
    assert_eq!(
        idle_result.lyapunov.v_convergence.raw() as i64,
        extract_i64(idle_vec, "v_convergence")
    );
    assert_eq!(
        idle_result.lyapunov.delta_window.raw() as i64,
        extract_i64(idle_vec, "delta_window")
    );
    assert_eq!(idle_filled, 1);
    assert_eq!(idle_window[0].raw(), 0);

    let halt_vec = vector_block(json, "LYAP-HALT-4-900K");
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
    assert_eq!(halt_state.epoch as i64, extract_i64(halt_vec, "epoch"));
    assert_eq!(
        halt_state.halt_reason as u8 as i64,
        extract_i64(halt_vec, "halt_reason")
    );
    assert_eq!(extract_i64(halt_vec, "v_convergence"), 2_700_000);
    assert_eq!(extract_i64(halt_vec, "delta_window"), 2_700_000);
    assert_eq!(halt_filled as usize, WINDOW_SIZE);
    assert_eq!(
        halt_window.iter().map(|v| v.raw()).collect::<Vec<_>>(),
        vec![0, 0, 0]
    );

    let epsilon_vec = vector_block(json, "LYAP-EPSILON-1");
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
        extract_i64(epsilon_vec, "epoch")
    );
    assert_eq!(
        epsilon_state.halt_reason as u8 as i64,
        extract_i64(epsilon_vec, "halt_reason")
    );
    assert_eq!(
        epsilon_result.lyapunov.v_convergence.raw() as i64,
        extract_i64(epsilon_vec, "v_convergence")
    );
    assert_eq!(
        epsilon_result.lyapunov.delta_window.raw() as i64,
        extract_i64(epsilon_vec, "delta_window")
    );
    assert_eq!(epsilon_filled as usize, WINDOW_SIZE);
    assert_eq!(
        epsilon_window.iter().map(|v| v.raw()).collect::<Vec<_>>(),
        vec![120_000, 100_000, 100_000]
    );

    let invalid_vec = vector_block(json, "LYAP-DECODE-INVALID-NEG-D");
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
        extract_i64(invalid_vec, "epoch")
    );
    assert_eq!(
        invalid_state.halt_reason as u8 as i64,
        extract_i64(invalid_vec, "halt_reason")
    );
    assert_eq!(
        invalid_eval.v_convergence.raw() as i64,
        extract_i64(invalid_vec, "v_convergence")
    );
    assert_eq!(
        invalid_eval.delta_window.raw() as i64,
        extract_i64(invalid_vec, "delta_window")
    );
    assert_eq!(invalid_filled, 0);

    let absorb_vec = vector_block(json, "LYAP-ABSORB-HALT");
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
    assert_eq!(absorb_state.epoch as i64, extract_i64(absorb_vec, "epoch"));
    assert_eq!(
        absorb_state.halt_reason as u8 as i64,
        extract_i64(absorb_vec, "halt_reason")
    );
    assert_eq!(
        absorb_eval.v_convergence.raw() as i64,
        extract_i64(absorb_vec, "v_convergence")
    );
    assert_eq!(
        absorb_eval.delta_window.raw() as i64,
        extract_i64(absorb_vec, "delta_window")
    );
    assert_eq!(absorb_filled as usize, WINDOW_SIZE);
    assert_eq!(
        absorb_window.iter().map(|v| v.raw()).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}
