// adversarial.rs — TH-A1 through TH-A3 adversarial scenario tests.
//
// Tests are prefixed `adversarial_` so `scripts/run_adversarial_scenarios.sh`
// picks them up via `cargo test adversarial_`.
//
// Coverage:
//   TV-3    TX-0 replay rejection
//   TH-A2   Batch ordering invariance; duplicate-in-batch idempotence
//   TH-A3   Byzantine inputs: malformed wire, invalid version, unknown type,
//            unknown author, wrong nonce, tx batch with non-authors,
//            slash monotonicity, update-count mismatch
//   §A3     State locality — TX-0 mutates only nonces[idx]
//   §A4     Encoding injectivity — distinct state fields → distinct roots
//   §A6     Halt absorption — all six HaltReason codes remain latched

use qash_consensus::transaction::{
    apply_all, is_admissible, parse_tx0,
    TxError, TX_TYPE_NOOP, TX_VERSION, TX0_WIRE_BYTES,
};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::fixed_point::{FixedPoint, SCALE};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn genesis_state() -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: 4,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        cascade_health: 0,
        state_root: [0u8; 32],
    }
}

fn idle_input(n: u32) -> EpochInput {
    EpochInput { updates: [None; MAX_VALIDATORS], update_count: n }
}

/// Build a well-formed TX-0 envelope with the given author_id and nonce.
fn make_tx0(author_id: [u8; 48], nonce: u64) -> [u8; TX0_WIRE_BYTES] {
    let mut raw = [0u8; TX0_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
    raw[4..12].copy_from_slice(&nonce.to_le_bytes());
    raw[12..60].copy_from_slice(&author_id);
    raw[60..64].copy_from_slice(&0u32.to_le_bytes()); // payload_len = 0
    // signature bytes remain zero (opaque in Domain A)
    raw
}

/// Assign distinct, recognisable validator IDs to the first `n` slots.
fn assign_ids(state: &mut EpochState, n: usize) {
    for i in 0..n {
        state.validator_ids[i] = [0u8; 48];
        state.validator_ids[i][0] = (i as u8) + 1;
    }
}

// ---------------------------------------------------------------------------
// TV-3 — Replay rejection
// ---------------------------------------------------------------------------

/// TV-3: submitting the same TX-0 in a second epoch (nonce already consumed)
/// must be rejected with NonceMismatch, leaving the nonce unchanged.
#[test]
fn adversarial_tx0_replay_rejected() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);
    let id0 = state.validator_ids[0];

    // First submission: nonce 0 → succeeds, nonce advances to 1.
    let tx = make_tx0(id0, 0);
    let r = advance_epoch(&mut state, &idle_input(4), &[tx.as_slice()]);
    assert!(r.is_ok(), "first TX-0 must succeed");
    assert_eq!(state.nonces[0], 1, "nonce must advance to 1");

    // Second submission with the same nonce=0 in the next epoch.
    let replay = make_tx0(id0, 0);
    let (parsed, _) = parse_tx0(replay.as_slice()).expect("parse must succeed");
    let err = is_admissible(&state, &parsed).expect_err("replay must be rejected");
    assert!(
        matches!(err, TxError::NonceMismatch { expected: 1, got: 0 }),
        "expected NonceMismatch {{expected:1, got:0}}, got: {:?}", err
    );
    // Nonce must remain 1; no state mutation from replay attempt.
    assert_eq!(state.nonces[0], 1);
}

/// TV-3 variant: replay inside apply_all — the second occurrence of the same TX
/// in a single epoch's batch is silently skipped (nonce moves on first apply,
/// nonce-mismatch on second → skipped).
#[test]
fn adversarial_duplicate_tx_in_batch_is_idempotent() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);
    let id0 = state.validator_ids[0];

    let tx = make_tx0(id0, 0);
    // Same TX bytes submitted twice in the same batch.
    let txs: &[&[u8]] = &[tx.as_slice(), tx.as_slice()];
    let applied = apply_all(&mut state, txs, 100).expect("apply_all must not error");
    // Exactly one TX should be applied (the second is rejected by nonce check).
    assert_eq!(applied, 1, "only one of the duplicate pair should be applied");
    assert_eq!(state.nonces[0], 1);
}

// ---------------------------------------------------------------------------
// TH-A2 — Batch ordering invariance
// ---------------------------------------------------------------------------

/// Submitting TX-0s for validators 0, 1, 2 in three different orderings must
/// yield identical post-state (sort_key canonicalises the order).
#[test]
fn adversarial_tx_batch_order_is_irrelevant() {
    let mut base = genesis_state();
    assign_ids(&mut base, 4);

    let tx0 = make_tx0(base.validator_ids[0], 0);
    let tx1 = make_tx0(base.validator_ids[1], 0);
    let tx2 = make_tx0(base.validator_ids[2], 0);

    let orderings: &[&[&[u8]]] = &[
        &[tx0.as_slice(), tx1.as_slice(), tx2.as_slice()],
        &[tx2.as_slice(), tx0.as_slice(), tx1.as_slice()],
        &[tx1.as_slice(), tx2.as_slice(), tx0.as_slice()],
    ];

    let mut roots: Vec<[u8; 32]> = Vec::new();
    for &batch in orderings {
        let mut state = base;
        advance_epoch(&mut state, &idle_input(4), batch).expect("must succeed");
        roots.push(state.state_root);
    }

    assert_eq!(roots[0], roots[1], "ordering 0 vs 2 diverged");
    assert_eq!(roots[0], roots[2], "ordering 0 vs 3 diverged");
}

// ---------------------------------------------------------------------------
// TH-A3 Byzantine inputs — wire format rejections
// ---------------------------------------------------------------------------

/// Envelope shorter than TX0_WIRE_BYTES → MalformedEnvelope.
#[test]
fn adversarial_malformed_envelope_too_short() {
    let short = vec![0u8; TX0_WIRE_BYTES - 1];
    let err = parse_tx0(&short).expect_err("short envelope must be rejected");
    assert_eq!(err, TxError::MalformedEnvelope);
}

/// Envelope with zero length → MalformedEnvelope.
#[test]
fn adversarial_malformed_envelope_empty() {
    let err = parse_tx0(&[]).expect_err("empty envelope must be rejected");
    assert_eq!(err, TxError::MalformedEnvelope);
}

/// version ≠ TX_VERSION (0x0001) → InvalidVersion.
#[test]
fn adversarial_invalid_version_rejected() {
    let mut raw = make_tx0([0u8; 48], 0);
    raw[0..2].copy_from_slice(&0x0002u16.to_le_bytes()); // version = 2
    let err = parse_tx0(&raw).expect_err("wrong version must be rejected");
    assert_eq!(err, TxError::InvalidVersion);
}

/// version = 0 → InvalidVersion.
#[test]
fn adversarial_version_zero_rejected() {
    let mut raw = make_tx0([0u8; 48], 0);
    raw[0..2].copy_from_slice(&0x0000u16.to_le_bytes());
    let err = parse_tx0(&raw).expect_err("version 0 must be rejected");
    assert_eq!(err, TxError::InvalidVersion);
}

/// tx_type ≠ TX_TYPE_NOOP (0x0000) → UnknownType.
#[test]
fn adversarial_unknown_tx_type_rejected() {
    let mut raw = make_tx0([0u8; 48], 0);
    raw[2..4].copy_from_slice(&0xFFFFu16.to_le_bytes()); // bogus type
    let err = parse_tx0(&raw).expect_err("unknown tx_type must be rejected");
    assert_eq!(err, TxError::UnknownType);
}

/// author_id not in the validator set → AuthorNotFound.
#[test]
fn adversarial_unknown_author_rejected() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);

    let stranger_id = [0xFF; 48]; // not in the validator set
    let raw = make_tx0(stranger_id, 0);
    let (tx, _) = parse_tx0(&raw).expect("parse must succeed");
    let err = is_admissible(&state, &tx).expect_err("unknown author must be rejected");
    assert_eq!(err, TxError::AuthorNotFound);
}

/// author_id all-zero (default; not explicitly registered) → AuthorNotFound.
#[test]
fn adversarial_zeroed_author_id_rejected() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);

    let raw = make_tx0([0u8; 48], 0); // all-zero id, not in the set
    let (tx, _) = parse_tx0(&raw).expect("parse must succeed");
    let err = is_admissible(&state, &tx).expect_err("all-zero author must be rejected");
    assert_eq!(err, TxError::AuthorNotFound);
}

/// Correct author but nonce is one ahead of current → NonceMismatch.
#[test]
fn adversarial_future_nonce_rejected() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);
    let id0 = state.validator_ids[0];

    let raw = make_tx0(id0, 1); // current nonce is 0; sending 1 is "future"
    let (tx, _) = parse_tx0(&raw).expect("parse must succeed");
    let err = is_admissible(&state, &tx).expect_err("future nonce must be rejected");
    assert!(
        matches!(err, TxError::NonceMismatch { expected: 0, got: 1 }),
        "wrong error: {:?}", err
    );
}

/// Correct author but nonce is far ahead → NonceMismatch.
#[test]
fn adversarial_far_future_nonce_rejected() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);
    let id0 = state.validator_ids[0];

    let raw = make_tx0(id0, u64::MAX);
    let (tx, _) = parse_tx0(&raw).expect("parse must succeed");
    let err = is_admissible(&state, &tx).expect_err("MAX nonce must be rejected");
    assert!(matches!(err, TxError::NonceMismatch { expected: 0, .. }));
}

/// A non-validator submitting a syntactically valid TX is rejected end-to-end
/// via advance_epoch (no state change).
#[test]
fn adversarial_non_validator_tx_end_to_end_rejected() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);
    let root_before = state.state_root;
    let nonces_before = state.nonces;

    let stranger = [0xAB; 48];
    let raw = make_tx0(stranger, 0);
    let r = advance_epoch(&mut state, &idle_input(4), &[raw.as_slice()]);
    assert!(r.is_ok(), "epoch must still advance (unknown author TX is skipped)");
    // Nonces must be unchanged.
    assert_eq!(state.nonces, nonces_before, "no nonces must change");
    // State root may differ (epoch entropy advances) but nonces are stable.
    let _ = root_before; // root changes from epoch; we check nonces only
}

// ---------------------------------------------------------------------------
// TH-A3 Byzantine inputs — epoch-level admissibility violations
// ---------------------------------------------------------------------------

/// update_count < validator_count triggers H4 (DecodeInvalid) absorbing halt.
#[test]
fn adversarial_update_count_mismatch_halts() {
    let mut state = genesis_state(); // validator_count = 4
    let bad = EpochInput { updates: [None; MAX_VALIDATORS], update_count: 3 };
    let r = advance_epoch(&mut state, &bad, &[]);
    assert_eq!(r, Err(HaltReason::DecodeInvalid));
    assert_eq!(state.halt_reason, HaltReason::DecodeInvalid);
}

/// slash_accum decrease in ValidatorUpdate triggers H4 absorbing halt.
#[test]
fn adversarial_slash_decrease_halts() {
    let mut state = genesis_state();
    state.validators[0].slash_accum = FixedPoint::from_raw(1_000);
    let mut input = idle_input(4);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new:  FixedPoint::ZERO,
        conflict_new:    FixedPoint::ZERO,
        slash_accum_new: FixedPoint::from_raw(500), // decrease → invalid
    });
    let r = advance_epoch(&mut state, &input, &[]);
    assert_eq!(r, Err(HaltReason::DecodeInvalid));
}

/// Negative divergence in ValidatorUpdate triggers H4 absorbing halt.
#[test]
fn adversarial_negative_divergence_halts() {
    let mut state = genesis_state();
    let mut input = idle_input(4);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new:  FixedPoint::from_raw(-1),
        conflict_new:    FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    let r = advance_epoch(&mut state, &input, &[]);
    assert_eq!(r, Err(HaltReason::DecodeInvalid));
}

/// Negative conflict in ValidatorUpdate triggers H4 absorbing halt.
#[test]
fn adversarial_negative_conflict_halts() {
    let mut state = genesis_state();
    let mut input = idle_input(4);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new:  FixedPoint::ZERO,
        conflict_new:    FixedPoint::from_raw(-1),
        slash_accum_new: FixedPoint::ZERO,
    });
    let r = advance_epoch(&mut state, &input, &[]);
    assert_eq!(r, Err(HaltReason::DecodeInvalid));
}

// ---------------------------------------------------------------------------
// §A6 — Halt absorption: all six HaltReason codes are absorbing
// ---------------------------------------------------------------------------

/// Once a halt flag is set to any of H1–H6, subsequent advance_epoch calls
/// must return Err with the SAME halt reason and leave state frozen.
#[test]
fn adversarial_halt_absorbing_lyapunov_violation() {
    check_halt_absorption(HaltReason::LyapunovViolation);
}

#[test]
fn adversarial_halt_absorbing_arith_overflow() {
    check_halt_absorption(HaltReason::ArithOverflow);
}

#[test]
fn adversarial_halt_absorbing_epoch_overflow() {
    check_halt_absorption(HaltReason::EpochOverflow);
}

#[test]
fn adversarial_halt_absorbing_decode_invalid() {
    check_halt_absorption(HaltReason::DecodeInvalid);
}

#[test]
fn adversarial_halt_absorbing_roundtrip_failure() {
    check_halt_absorption(HaltReason::RoundtripFailure);
}

#[test]
fn adversarial_halt_absorbing_halt_flag_set() {
    check_halt_absorption(HaltReason::HaltFlagSet);
}

fn check_halt_absorption(reason: HaltReason) {
    let mut state = genesis_state();
    state.halt_reason = reason;

    // Snapshot frozen state fields.
    let epoch_before = state.epoch;
    let nonces_before = state.nonces;
    let root_before = state.state_root;

    // Any advance must return the same halt.
    let r1 = advance_epoch(&mut state, &idle_input(4), &[]);
    assert_eq!(r1, Err(reason), "first advance must return Err({:?})", reason);
    assert_eq!(state.halt_reason, reason);
    assert_eq!(state.epoch, epoch_before, "epoch must not advance");
    assert_eq!(state.nonces, nonces_before, "nonces must not change");
    assert_eq!(state.state_root, root_before, "state_root must not change");

    // Second advance: still frozen.
    let r2 = advance_epoch(&mut state, &idle_input(4), &[]);
    assert_eq!(r2, Err(reason), "second advance must also return Err({:?})", reason);
}

// ---------------------------------------------------------------------------
// §A3 — State locality: TX-0 mutates ONLY nonces[idx]
// ---------------------------------------------------------------------------

/// After applying one TX-0 for validator 0, only nonces[0] changes.
/// All other nonces, validator metrics, entropy_seed, halt_reason, and
/// validator_ids are unchanged by the TX-0 application itself.
///
/// (The epoch also advances entropy_seed and state_root as a side-effect of
/// advance_epoch — so we test apply_all directly to isolate TX-0 locality.)
#[test]
fn adversarial_tx0_state_locality() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);

    // Capture pre-TX snapshots.
    let metrics_before = state.validators;
    let ids_before = state.validator_ids;
    let nonces_before = state.nonces;

    let id0 = state.validator_ids[0];
    let tx = make_tx0(id0, 0);
    let applied = apply_all(&mut state, &[tx.as_slice()], 100).expect("must succeed");
    assert_eq!(applied, 1);

    // Only nonces[0] may change.
    assert_eq!(state.nonces[0], 1, "nonce 0 must advance");
    for i in 1..4 {
        assert_eq!(state.nonces[i], nonces_before[i], "nonce {} must not change", i);
    }
    // Validator metrics must be untouched by TX-0.
    for i in 0..4 {
        assert_eq!(
            state.validators[i].divergence, metrics_before[i].divergence,
            "validator {} divergence changed", i
        );
        assert_eq!(
            state.validators[i].slash_accum, metrics_before[i].slash_accum,
            "validator {} slash_accum changed", i
        );
    }
    // Validator IDs must be unchanged.
    assert_eq!(state.validator_ids, ids_before);
    // halt_reason and epoch are also unchanged by apply_all alone.
    assert!(matches!(state.halt_reason, HaltReason::None));
    assert_eq!(state.epoch, 0);
}

// ---------------------------------------------------------------------------
// §A4 — Encoding injectivity (Rust-level): distinct fields → distinct roots
// ---------------------------------------------------------------------------

/// Different epoch counters produce different state roots.
#[test]
fn adversarial_encoding_injectivity_epoch() {
    let mut a = genesis_state();
    let mut b = genesis_state();
    advance_epoch(&mut a, &idle_input(4), &[]).unwrap();
    advance_epoch(&mut b, &idle_input(4), &[]).unwrap();
    assert_eq!(a.state_root, b.state_root, "same inputs must produce same root");
    // Manually corrupt epoch in b after advance — root must differ when recomputed.
    // (We can't recompute without re-running advance_epoch, so we test a simpler form:
    //  two epochs of advancement vs one — roots must differ.)
    advance_epoch(&mut b, &idle_input(4), &[]).unwrap();
    assert_ne!(a.state_root, b.state_root, "different epochs must produce different roots");
}

/// Different nonce values produce different state roots.
#[test]
fn adversarial_encoding_injectivity_nonce() {
    let mut s_a = genesis_state();
    let mut s_b = genesis_state();
    assign_ids(&mut s_a, 4);
    assign_ids(&mut s_b, 4);

    let id0 = s_a.validator_ids[0];
    let tx = make_tx0(id0, 0);
    // Advance s_a with TX-0 (nonce becomes 1), s_b without.
    advance_epoch(&mut s_a, &idle_input(4), &[tx.as_slice()]).unwrap();
    advance_epoch(&mut s_b, &idle_input(4), &[]).unwrap();

    assert_ne!(
        s_a.state_root, s_b.state_root,
        "different nonce states must produce different roots"
    );
}

/// Different validator IDs produce different state roots.
#[test]
fn adversarial_encoding_injectivity_validator_id() {
    let mut s_a = genesis_state();
    let mut s_b = genesis_state();

    // s_a: validator_ids[0] = [1, 0, ...]
    s_a.validator_ids[0][0] = 1;
    // s_b: validator_ids[0] = [2, 0, ...]
    s_b.validator_ids[0][0] = 2;

    advance_epoch(&mut s_a, &idle_input(4), &[]).unwrap();
    advance_epoch(&mut s_b, &idle_input(4), &[]).unwrap();

    assert_ne!(
        s_a.state_root, s_b.state_root,
        "different validator_ids must produce different roots"
    );
}

/// Different slash accumulators produce different state roots.
#[test]
fn adversarial_encoding_injectivity_slash_accum() {
    let mut s_a = genesis_state();
    let mut s_b = genesis_state();

    // Give s_b a non-zero slash accumulator for validator 0.
    let mut input_b = idle_input(4);
    input_b.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::ZERO,
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::from_raw(1_000),
    });

    advance_epoch(&mut s_a, &idle_input(4), &[]).unwrap();
    advance_epoch(&mut s_b, &input_b, &[]).unwrap();

    assert_ne!(
        s_a.state_root, s_b.state_root,
        "different slash_accum must produce different roots"
    );
}

// ---------------------------------------------------------------------------
// §A1 — Partition safety simulation
// ---------------------------------------------------------------------------

/// TH-A1: Two validators process epochs independently ("partitioned"), then
/// when given the same merged canonical input set, produce the same root.
///
/// This tests that Domain-A computation depends only on the canonical state
/// and input set, not on communication history or ordering within a partition.
#[test]
fn adversarial_partition_same_inputs_same_root() {
    // "Partition A" processes 2 idle epochs, then merges.
    let mut partition_a = genesis_state();
    advance_epoch(&mut partition_a, &idle_input(4), &[]).unwrap();
    advance_epoch(&mut partition_a, &idle_input(4), &[]).unwrap();

    // "Partition B" processes 2 idle epochs independently (same inputs).
    let mut partition_b = genesis_state();
    advance_epoch(&mut partition_b, &idle_input(4), &[]).unwrap();
    advance_epoch(&mut partition_b, &idle_input(4), &[]).unwrap();

    // After the same deterministic inputs, both must agree.
    assert_eq!(
        partition_a.state_root, partition_b.state_root,
        "identical inputs on partitioned nodes must yield identical roots (TH-A1)"
    );
    assert_eq!(partition_a.epoch, partition_b.epoch);
    assert_eq!(partition_a.nonces, partition_b.nonces);
}

/// TH-A1 variant: partitions that advance through a halt trigger must
/// agree on the halted state.
#[test]
fn adversarial_partition_halt_is_deterministic() {
    fn advance_to_halt(state: &mut EpochState) {
        // Fill window with zero-divergence epochs so halt check activates.
        for _ in 0..WINDOW_SIZE {
            let _ = advance_epoch(state, &idle_input(state.validator_count), &[]);
        }
        // Spike validator 0 above ε to trigger H1.
        let mut spike = idle_input(state.validator_count);
        spike.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(1_000_000),
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
        let _ = advance_epoch(state, &spike, &[]);
    }

    let mut a = genesis_state();
    let mut b = genesis_state();
    advance_to_halt(&mut a);
    advance_to_halt(&mut b);

    assert!(a.is_halted(), "partition A must be halted");
    assert!(b.is_halted(), "partition B must be halted");
    assert_eq!(
        a.halt_reason, b.halt_reason,
        "both partitions must halt with the same reason"
    );
    assert_eq!(
        a.state_root, b.state_root,
        "halted state roots must agree across partitions"
    );
}

// ---------------------------------------------------------------------------
// Lyapunov halt trigger (H1)
// ---------------------------------------------------------------------------

/// Filling the window with idle epochs then spiking divergence above ε triggers H1.
/// The halt mechanism requires the window to be full before checking δ_window > ε.
#[test]
fn adversarial_lyapunov_violation_triggers_halt() {
    let mut state = genesis_state();

    // Fill window with zero-divergence epochs.
    for _ in 0..WINDOW_SIZE {
        advance_epoch(&mut state, &idle_input(4), &[]).expect("idle epoch must succeed");
    }
    assert!(state.convergence_window.is_full(), "window must be full");

    // Spike divergence to 1_000_000 for validator 0; well above ε = 20_000.
    let mut spike = idle_input(4);
    spike.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(1_000_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    let r = advance_epoch(&mut state, &spike, &[]);
    assert_eq!(r, Err(HaltReason::LyapunovViolation),
        "spike above ε must trigger H1 (LyapunovViolation)");
    assert!(state.is_halted());
}

// ---------------------------------------------------------------------------
// Phase 2 boundary tests (Issue #22 — hardening)
// ---------------------------------------------------------------------------

/// §A4: D > SCALE must be rejected with DecodeInvalid.
#[test]
fn adversarial_divergence_exceeds_scale() {
    let mut state = genesis_state();
    let mut input = idle_input(4);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new:  FixedPoint::from_raw(SCALE + 1),
        conflict_new:    FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    assert_eq!(
        advance_epoch(&mut state, &input, &[]),
        Err(HaltReason::DecodeInvalid),
        "D > SCALE must trigger DecodeInvalid"
    );
}

/// §A4: C > SCALE must be rejected with DecodeInvalid.
#[test]
fn adversarial_conflict_exceeds_scale() {
    let mut state = genesis_state();
    let mut input = idle_input(4);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new:  FixedPoint::ZERO,
        conflict_new:    FixedPoint::from_raw(SCALE + 1),
        slash_accum_new: FixedPoint::ZERO,
    });
    assert_eq!(
        advance_epoch(&mut state, &input, &[]),
        Err(HaltReason::DecodeInvalid),
        "C > SCALE must trigger DecodeInvalid"
    );
}

/// §A4: update present at slot index >= validator_count must be rejected.
/// Lines 461-465 in transition.rs enforce this; this test is the CI gate.
#[test]
fn adversarial_update_beyond_validator_count() {
    let mut state = genesis_state(); // validator_count = 4
    let mut input = idle_input(4);
    // Slot 4 is beyond vc=4 (valid slots are 0..3).
    input.updates[4] = Some(ValidatorUpdate {
        divergence_new:  FixedPoint::ZERO,
        conflict_new:    FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    assert_eq!(
        advance_epoch(&mut state, &input, &[]),
        Err(HaltReason::DecodeInvalid),
        "update at slot >= validator_count must trigger DecodeInvalid"
    );
}

/// §A4: slash_accum below the Φ_safety threshold is valid; same value on next epoch
/// (no-op, monotone) is also valid.
/// PHI_MAX_SAFE = 500_000_000 raw; WEIGHT_S = 0.25; threshold per validator = 2_000_000_000.
#[test]
fn adversarial_max_slash_boundary_is_valid() {
    // 1_900_000_000 < 2_000_000_000 (phi halt threshold for one validator).
    let large: i128 = 1_900_000_000;
    let mut state = genesis_state();
    // Set slash_accum to a large valid value below the Φ_safety halt gate.
    let mut input = idle_input(4);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new:  FixedPoint::ZERO,
        conflict_new:    FixedPoint::ZERO,
        slash_accum_new: FixedPoint::from_raw(large),
    });
    assert!(advance_epoch(&mut state, &input, &[]).is_ok(), "large slash below phi threshold must be accepted");

    // Same value again (no-op, satisfies monotonicity) — must still succeed.
    let mut input2 = idle_input(4);
    input2.updates[0] = Some(ValidatorUpdate {
        divergence_new:  FixedPoint::ZERO,
        conflict_new:    FixedPoint::ZERO,
        slash_accum_new: FixedPoint::from_raw(large),
    });
    assert!(advance_epoch(&mut state, &input2, &[]).is_ok(), "same slash (no-op) must be accepted");
}

/// H7: slash_accum at or above the Φ_safety threshold triggers PhiSafetyViolation halt.
/// PHI_MAX_SAFE = 500_000_000 raw; WEIGHT_S = 0.25; per-validator threshold = 2_000_000_000.
#[test]
fn adversarial_phi_safety_halt_triggers_at_threshold() {
    // 2_000_000_000 raw slash for one validator → phi = 500_000_000 = PHI_MAX_SAFE → halt.
    let at_threshold: i128 = 2_000_000_000;
    let mut state = genesis_state();
    let mut input = idle_input(4);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new:  FixedPoint::ZERO,
        conflict_new:    FixedPoint::ZERO,
        slash_accum_new: FixedPoint::from_raw(at_threshold),
    });
    assert_eq!(
        advance_epoch(&mut state, &input, &[]),
        Err(HaltReason::PhiSafetyViolation),
        "slash at phi threshold must trigger PhiSafetyViolation"
    );
    assert_eq!(state.halt_reason, HaltReason::PhiSafetyViolation);
}
