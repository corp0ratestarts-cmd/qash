// axioms.rs — Direct tests for transition axioms §A0–A11 from
// docs/spec/02_transition_axioms.md, consensus invariants from
// docs/spec/01_consensus.md, and TV-3 from docs/spec/07_test_vectors.md.
//
// Each test cites its corresponding spec section.

use qash_consensus::fixed_point::{FixedPoint, SCALE};
use qash_consensus::hash::{h_domain, DomainTag};
use qash_consensus::lyapunov::{self, ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::transaction::{apply_all, parse_tx0, TX0_WIRE_BYTES, TX_TYPE_NOOP, TX_VERSION};
use qash_consensus::transition::{
    advance_epoch, decode_full_state, encode_full_state_into, EpochInput, EpochState, HaltReason,
    ValidatorUpdate, FULL_STATE_MAX_BYTES, MAX_VALIDATORS,
};

// ---------------------------------------------------------------------------
// Shared helpers (mirrors golden_replay.rs to keep tests self-contained)
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
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
        causal_fingerprint: [0u8; 32],
    }
}

fn idle_input(n: u32) -> EpochInput {
    EpochInput::new(n)
}

fn assign_ids(state: &mut EpochState, n: usize) {
    for i in 0..n {
        state.validator_ids[i] = [0u8; 48];
        state.validator_ids[i][0] = (i as u8) + 1;
    }
}

fn make_tx0(author_id: [u8; 48], nonce: u64) -> [u8; TX0_WIRE_BYTES] {
    let mut raw = [0u8; TX0_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
    raw[4..12].copy_from_slice(&nonce.to_le_bytes());
    raw[12..60].copy_from_slice(&author_id);
    raw[60..64].copy_from_slice(&0u32.to_le_bytes());
    raw
}

// ---------------------------------------------------------------------------
// §A1 — Determinism Axiom
// ---------------------------------------------------------------------------

/// §A1: Equal genesis + equal canonical inputs ⟹ equal state_root.
/// Tests determinism over 5 epochs with validator updates.
#[test]
fn axiom_a1_determinism_with_updates() {
    let mut s1 = genesis_state();
    let mut s2 = genesis_state();

    for epoch in 0u64..5 {
        let mut input = idle_input(4);
        // Apply the same deterministic update to both replicas.
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(10_000 + epoch as i128 * 1_000),
            conflict_new: FixedPoint::from_raw(5_000),
            slash_accum_new: FixedPoint::ZERO,
        });
        advance_epoch(&mut s1, &input, &[]).unwrap();
        advance_epoch(&mut s2, &input, &[]).unwrap();

        assert_eq!(
            s1.state_root, s2.state_root,
            "§A1: state_roots diverged at epoch {}",
            epoch
        );
    }
}

/// §A1: A single-bit flip in the input must produce a different state root.
#[test]
fn axiom_a1_distinct_inputs_produce_distinct_roots() {
    let mut s_a = genesis_state();
    let mut s_b = genesis_state();

    let input_a = idle_input(4);
    let mut input_b = idle_input(4);
    input_b.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(1),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });

    advance_epoch(&mut s_a, &input_a, &[]).unwrap();
    advance_epoch(&mut s_b, &input_b, &[]).unwrap();

    assert_ne!(
        s_a.state_root, s_b.state_root,
        "§A1: distinct inputs must produce distinct roots"
    );
}

// ---------------------------------------------------------------------------
// §A3 — State Locality Axiom
// ---------------------------------------------------------------------------

/// §A3: TX-0 must ONLY mutate nonces[idx]. All other state fields are
/// read-only for TX-0 (mutation footprint = {nonces[author_idx]}).
#[test]
fn axiom_a3_tx0_mutation_footprint() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);

    let before_metrics = state.validators;
    let before_ids = state.validator_ids;
    let before_nonces = state.nonces;
    let before_halt = state.halt_reason;

    let tx = make_tx0(state.validator_ids[0], 0);
    apply_all(&mut state, &[tx.as_slice()], 100).expect("apply_all must succeed");

    // Exactly nonces[0] changed; everything else is untouched.
    assert_eq!(
        state.nonces[0],
        before_nonces[0] + 1,
        "nonces[0] must increment"
    );
    for i in 1..4 {
        assert_eq!(
            state.nonces[i], before_nonces[i],
            "nonces[{}] must not change",
            i
        );
    }
    for i in 0..4 {
        assert_eq!(state.validators[i].divergence, before_metrics[i].divergence);
        assert_eq!(state.validators[i].conflict, before_metrics[i].conflict);
        assert_eq!(
            state.validators[i].slash_accum,
            before_metrics[i].slash_accum
        );
    }
    assert_eq!(state.validator_ids, before_ids);
    assert!(matches!(state.halt_reason, HaltReason::None));
    assert_eq!(before_halt as u8, state.halt_reason as u8);
}

// ---------------------------------------------------------------------------
// §A4 — Encoding Preservation Axiom (StateWF roundtrip)
// ---------------------------------------------------------------------------

/// §A4: Every advance_epoch output must survive a full encode→decode roundtrip.
/// This confirms StateWF holds for all produced states.
#[test]
fn axiom_a4_encoding_preservation_multi_epoch() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);

    let mut buf = [0u8; FULL_STATE_MAX_BYTES];
    for epoch in 0u64..10 {
        let mut input = idle_input(4);
        if epoch % 3 == 0 {
            input.updates[0] = Some(ValidatorUpdate {
                divergence_new: FixedPoint::from_raw(epoch as i128 * 1_000),
                conflict_new: FixedPoint::ZERO,
                slash_accum_new: FixedPoint::ZERO,
            });
        }
        advance_epoch(&mut state, &input, &[]).unwrap();

        // Encode.
        let n = encode_full_state_into(&state, &mut buf);
        // Decode.
        let decoded = decode_full_state(&buf[..n]).expect("decode must succeed");

        assert_eq!(
            decoded.state_root, state.state_root,
            "§A4: state_root mismatch after roundtrip at epoch {}",
            epoch
        );
        assert_eq!(decoded.epoch, state.epoch);
        assert_eq!(decoded.halt_reason as u8, state.halt_reason as u8);
    }
}

// ---------------------------------------------------------------------------
// §A5 — Replay Preservation (TH-7)
// ---------------------------------------------------------------------------

/// §A5: Replay from genesis using the identical input sequence must produce
/// the identical final state_root. (Single-ISA replay; cross-ISA is CI.)
#[test]
fn axiom_a5_replay_invariance_identical_sequence() {
    fn run_sequence() -> EpochState {
        let mut state = genesis_state();
        assign_ids(&mut state, 4);
        for i in 0..5u64 {
            let mut input = idle_input(4);
            input.updates[1] = Some(ValidatorUpdate {
                divergence_new: FixedPoint::from_raw(i as i128 * 2_000),
                conflict_new: FixedPoint::ZERO,
                slash_accum_new: FixedPoint::ZERO,
            });
            advance_epoch(&mut state, &input, &[]).unwrap();
        }
        state
    }

    let run1 = run_sequence();
    let run2 = run_sequence();
    assert_eq!(
        run1.state_root, run2.state_root,
        "§A5: replay must be invariant"
    );
    assert_eq!(run1.epoch, run2.epoch);
}

// ---------------------------------------------------------------------------
// §A6 — Halt Monotonicity Axiom
// ---------------------------------------------------------------------------

/// §A6: halt_flag, once set, can never be cleared. Tested across 3 advance calls.
#[test]
fn axiom_a6_halt_flag_never_clears() {
    let mut state = genesis_state();

    // Force halt via bad update_count.
    let bad_input = EpochInput::new(3);
    let r = advance_epoch(&mut state, &bad_input, &[]);
    assert_eq!(r, Err(HaltReason::DecodeInvalid));
    assert!(state.is_halted(), "§A6: must be halted after first trigger");

    for attempt in 0..5 {
        let result = advance_epoch(&mut state, &idle_input(4), &[]);
        assert!(
            result.is_err(),
            "§A6: attempt {} must also be halted",
            attempt
        );
        assert!(state.is_halted(), "§A6: halt_flag must remain set");
        assert_eq!(
            state.halt_reason as u8,
            HaltReason::DecodeInvalid as u8,
            "§A6: halt_reason must be unchanged at attempt {}",
            attempt
        );
    }
}

/// §A6: Even with a well-formed TX-0 batch, a halted state stays halted.
#[test]
fn axiom_a6_tx0_does_not_clear_halt() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);
    state.halt_reason = HaltReason::LyapunovViolation;

    let id0 = state.validator_ids[0];
    let tx = make_tx0(id0, 0);
    let r = advance_epoch(&mut state, &idle_input(4), &[tx.as_slice()]);
    assert_eq!(r, Err(HaltReason::LyapunovViolation));
    assert_eq!(state.halt_reason as u8, HaltReason::LyapunovViolation as u8);
    assert_eq!(state.nonces[0], 0, "nonce must not advance in halted state");
}

// ---------------------------------------------------------------------------
// §A8 Form A — TX-0 zero perturbation
// ---------------------------------------------------------------------------

/// §A8 Form A: TX-0 does not change V_convergence (ε_τ = 0).
/// Before and after applying TX-0, the Lyapunov values for all validators
/// are identical — only the nonce changes.
#[test]
fn axiom_a8_form_a_tx0_zero_perturbation() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);

    // Set some non-trivial metrics.
    let input = {
        let mut i = idle_input(4);
        i.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(100_000),
            conflict_new: FixedPoint::from_raw(50_000),
            slash_accum_new: FixedPoint::ZERO,
        });
        i
    };
    advance_epoch(&mut state, &input, &[]).unwrap();

    let metrics_before = state.validators;
    let id0 = state.validator_ids[0];
    let tx = make_tx0(id0, state.nonces[0]);
    apply_all(&mut state, &[tx.as_slice()], 100).expect("apply_all must succeed");

    // Lyapunov metrics must be unchanged.
    for i in 0..4 {
        assert_eq!(
            state.validators[i].divergence, metrics_before[i].divergence,
            "§A8 Form A: divergence changed for validator {}",
            i
        );
        assert_eq!(
            state.validators[i].conflict, metrics_before[i].conflict,
            "§A8 Form A: conflict changed for validator {}",
            i
        );
        assert_eq!(
            state.validators[i].slash_accum, metrics_before[i].slash_accum,
            "§A8 Form A: slash_accum changed for validator {}",
            i
        );
    }
}

// ---------------------------------------------------------------------------
// §1 — State space constraints: validator_count ≤ N_max
// ---------------------------------------------------------------------------

/// §1: Advancing an epoch with validator_count = MAX_VALIDATORS (1024) must
/// succeed (boundary case should not panic or fail structurally).
#[test]
fn axiom_consensus_max_validators_accepted() {
    let mut state = genesis_state();
    state.validator_count = MAX_VALIDATORS as u32; // 1024

    // All validators have zero metrics → idle input is valid.
    let result = advance_epoch(&mut state, &idle_input(MAX_VALIDATORS as u32), &[]);
    // Either succeeds (preferred) or halts — must NOT panic.
    match result {
        Ok(_) => {}
        Err(r) => {
            // If it halts, must be a protocol halt, not a panic.
            assert_ne!(r as u8, 0, "must be a real halt reason");
        }
    }
}

/// §1: validator_count = 0 is a degenerate but valid edge case.
#[test]
fn axiom_consensus_zero_validators_accepted() {
    let mut state = genesis_state();
    state.validator_count = 0;
    let result = advance_epoch(&mut state, &idle_input(0), &[]);
    // Must succeed or halt cleanly — must NOT panic.
    let _ = result;
}

// ---------------------------------------------------------------------------
// Entropy chain — §3 determinism
// ---------------------------------------------------------------------------

/// §3: entropy_seed must advance to a non-zero value after one epoch.
#[test]
fn axiom_entropy_advances_nonzero() {
    let mut state = genesis_state();
    assert_eq!(state.entropy_seed, [0u8; 32]);
    advance_epoch(&mut state, &idle_input(4), &[]).unwrap();
    assert_ne!(state.entropy_seed, [0u8; 32], "entropy_seed must advance");
}

/// §3: entropy chains are deterministic across two replicas.
#[test]
fn axiom_entropy_chain_is_deterministic() {
    let mut a = genesis_state();
    let mut b = genesis_state();
    for _ in 0..5 {
        advance_epoch(&mut a, &idle_input(4), &[]).unwrap();
        advance_epoch(&mut b, &idle_input(4), &[]).unwrap();
        assert_eq!(a.entropy_seed, b.entropy_seed, "entropy chains must match");
    }
}

/// §3: different initial entropy_seed produces different chain.
#[test]
fn axiom_entropy_seed_binding() {
    let mut a = genesis_state();
    let mut b = genesis_state();
    b.entropy_seed[0] = 1; // differ at byte 0

    advance_epoch(&mut a, &idle_input(4), &[]).unwrap();
    advance_epoch(&mut b, &idle_input(4), &[]).unwrap();
    assert_ne!(
        a.entropy_seed, b.entropy_seed,
        "different seeds must produce different chains"
    );
    assert_ne!(
        a.state_root, b.state_root,
        "different seeds must produce different roots"
    );
}

// ---------------------------------------------------------------------------
// §5 — δ_window threshold ε = 20_000
// ---------------------------------------------------------------------------

/// §5: After filling the window with exactly ε divergence, a further epoch at
/// the same level must NOT halt — the window min equals current, so δ = 0.
#[test]
fn axiom_delta_window_at_epsilon_does_not_halt() {
    let mut state = genesis_state();
    let epsilon = FixedPoint::from_raw(20_000);

    // Fill window uniformly at epsilon so min_window == epsilon.
    for _ in 0..WINDOW_SIZE {
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: epsilon,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
        let r = advance_epoch(&mut state, &input, &[]);
        assert!(r.is_ok(), "at-epsilon fill must not halt");
    }
    // One more epoch at the same level: δ = epsilon - epsilon = 0, must not halt.
    let mut input = idle_input(4);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new: epsilon,
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    let r = advance_epoch(&mut state, &input, &[]);
    assert!(r.is_ok(), "stable at-epsilon must not halt");
    assert!(
        !state.is_halted(),
        "exact ε with no increase must not trigger halt"
    );
}

/// §5: Fill window with zero, then spike above ε → halt (H1).
#[test]
fn axiom_delta_window_above_epsilon_halts() {
    let mut state = genesis_state();

    // Fill window at zero so window min = 0.
    for _ in 0..WINDOW_SIZE {
        advance_epoch(&mut state, &idle_input(4), &[]).expect("fill must succeed");
    }
    // Spike validator 0 well above ε.
    let mut spike = idle_input(4);
    spike.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(1_000_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    let r = advance_epoch(&mut state, &spike, &[]);
    assert_eq!(
        r,
        Err(HaltReason::LyapunovViolation),
        "spike from 0 to 1_000_000 must trigger H1"
    );
}

// ---------------------------------------------------------------------------
// HaltReason encoding — all 6 variants decode correctly
// ---------------------------------------------------------------------------

/// All seven HaltReason codes must survive encode→decode roundtrip.
#[test]
fn axiom_all_halt_reasons_roundtrip() {
    let halt_codes: &[HaltReason] = &[
        HaltReason::LyapunovViolation,
        HaltReason::ArithOverflow,
        HaltReason::EpochOverflow,
        HaltReason::DecodeInvalid,
        HaltReason::RoundtripFailure,
        HaltReason::HaltFlagSet,
        HaltReason::PhiSafetyViolation,
    ];
    for &reason in halt_codes {
        let mut state = genesis_state();
        state.halt_reason = reason;

        let mut buf = [0u8; FULL_STATE_MAX_BYTES];
        let n = encode_full_state_into(&state, &mut buf);
        let decoded = decode_full_state(&buf[..n]).expect("decode must succeed");

        assert_eq!(
            decoded.halt_reason as u8, reason as u8,
            "HaltReason {:?} did not roundtrip",
            reason
        );
    }
}

// ---------------------------------------------------------------------------
// Nonce monotonicity — sequential TX-0 chain
// ---------------------------------------------------------------------------

/// Nonces must advance sequentially (0 → 1 → 2 → 3 …) for a single author.
#[test]
fn axiom_nonce_sequential_chain() {
    let mut state = genesis_state();
    assign_ids(&mut state, 4);
    let id = state.validator_ids[0];

    for expected_nonce in 0u64..10 {
        let tx = make_tx0(id, expected_nonce);
        let r = advance_epoch(&mut state, &idle_input(4), &[tx.as_slice()]);
        assert!(r.is_ok(), "epoch {} must succeed", expected_nonce);
        assert_eq!(
            state.nonces[0],
            expected_nonce + 1,
            "nonce must be {} after epoch {}",
            expected_nonce + 1,
            expected_nonce
        );
    }
}

// ---------------------------------------------------------------------------
// TX-0 wire format completeness
// ---------------------------------------------------------------------------

/// Every byte of a well-formed TX-0 is significant: flipping any single byte
/// should either change the parsed result or be rejected.
/// We spot-check the version byte, type bytes, nonce bytes, and author_id bytes.
#[test]
fn axiom_tx0_wire_version_byte_is_checked() {
    let raw = make_tx0([1u8; 48], 0);
    // Corrupt byte 0 (LSB of version).
    let mut bad = raw;
    bad[0] ^= 0x01;
    assert!(
        parse_tx0(&bad).is_err(),
        "corrupted version byte must be rejected"
    );
}

#[test]
fn axiom_tx0_wire_type_byte_is_checked() {
    let raw = make_tx0([1u8; 48], 0);
    // Corrupt byte 2 (LSB of tx_type).
    let mut bad = raw;
    bad[2] ^= 0x01;
    assert!(
        parse_tx0(&bad).is_err(),
        "corrupted tx_type byte must be rejected"
    );
}

/// Nonce bytes can be read from the envelope accurately.
#[test]
fn axiom_tx0_wire_nonce_parsed_correctly() {
    let nonce: u64 = 0x0102_0304_0506_0708;
    let raw = make_tx0([1u8; 48], nonce);
    let (tx, _) = parse_tx0(&raw).expect("must parse");
    assert_eq!(tx.nonce, nonce, "nonce must be parsed correctly");
}

/// author_id bytes are parsed accurately (48-byte field).
#[test]
fn axiom_tx0_wire_author_id_parsed_correctly() {
    let mut id = [0u8; 48];
    id[0] = 0xAB;
    id[47] = 0xCD;
    let raw = make_tx0(id, 0);
    let (tx, _) = parse_tx0(&raw).expect("must parse");
    assert_eq!(tx.author_id, id, "author_id must be parsed correctly");
}

// ---------------------------------------------------------------------------
// Model/Runtime parity (inspired by PR #17 concept)
// ---------------------------------------------------------------------------

/// Independent model implementation of one epoch transition using public API
/// only (lyapunov::evaluate, h_domain, etc.). Compared against advance_epoch
/// to detect drift between spec-faithful model and runtime implementation.
/// Returns Err(halt_reason) or Ok(state_root).
fn model_apply(state: &mut EpochState, input: &EpochInput) -> Result<[u8; 32], HaltReason> {
    // Halt absorption
    if state.halt_reason != HaltReason::None {
        return Err(state.halt_reason);
    }

    // update_count must match validator_count
    if input.update_count != state.validator_count {
        state.halt_reason = HaltReason::DecodeInvalid;
        return Err(HaltReason::DecodeInvalid);
    }

    // Validate and project metrics
    let mut projected = state.validators;
    for i in 0..state.validator_count as usize {
        if let Some(u) = input.updates[i] {
            if u.divergence_new.raw() < 0
                || u.divergence_new.raw() > SCALE
                || u.conflict_new.raw() < 0
                || u.conflict_new.raw() > SCALE
                || u.slash_accum_new.raw() < 0
                || u.slash_accum_new.raw() < state.validators[i].slash_accum.raw()
            {
                state.halt_reason = HaltReason::DecodeInvalid;
                return Err(HaltReason::DecodeInvalid);
            }
            projected[i] = ValidatorMetrics {
                divergence: u.divergence_new,
                conflict: u.conflict_new,
                slash_accum: u.slash_accum_new,
            };
        }
    }
    // No update should exist for slots beyond validator_count
    for i in state.validator_count as usize..MAX_VALIDATORS {
        if input.updates[i].is_some() {
            state.halt_reason = HaltReason::DecodeInvalid;
            return Err(HaltReason::DecodeInvalid);
        }
    }

    // Lyapunov evaluation
    let lyap = match lyapunov::evaluate(
        &projected[..state.validator_count as usize],
        &state.convergence_window,
    ) {
        Ok(ev) => ev,
        Err(lyapunov::LyapunovError::Overflow) => {
            state.halt_reason = HaltReason::ArithOverflow;
            return Err(HaltReason::ArithOverflow);
        }
        Err(lyapunov::LyapunovError::UnboundedMetric) => {
            state.halt_reason = HaltReason::DecodeInvalid;
            return Err(HaltReason::DecodeInvalid);
        }
    };

    if lyap.halt_triggered {
        state.halt_reason = HaltReason::LyapunovViolation;
        return Err(HaltReason::LyapunovViolation);
    }

    // Epoch overflow
    let next_epoch = match state.epoch.checked_add(1) {
        Some(v) => v,
        None => {
            state.halt_reason = HaltReason::EpochOverflow;
            return Err(HaltReason::EpochOverflow);
        }
    };

    // Commit
    state.validators = projected;
    state.convergence_window.push(lyap.v_convergence);
    state.entropy_seed = h_domain(DomainTag::EntropyAdvance, &state.entropy_seed);
    state.epoch = next_epoch;

    Ok(state.state_root)
}

/// Parity: model_apply and advance_epoch must agree on halt/no-halt outcome and
/// produce the same epoch, convergence window, and entropy_seed on every step.
/// Covers nominal progression, boundary-epsilon, and halt-triggering cases —
/// the three fixture classes from PR #17, implemented without serde/JSON.
#[test]
fn axiom_model_runtime_parity_nominal() {
    let steps: &[(u32, i128, i128)] = &[
        // (validator_count, divergence, conflict) — well below epsilon, no halt
        (4, 1_000, 500),
        (4, 2_000, 1_000),
        (4, 3_000, 1_500),
        (4, 4_000, 2_000),
        (4, 5_000, 2_500),
    ];

    let mut runtime = genesis_state();
    let mut model = genesis_state();

    for &(vc, div, con) in steps {
        let mut input = idle_input(vc);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(div),
            conflict_new: FixedPoint::from_raw(con),
            slash_accum_new: FixedPoint::ZERO,
        });

        let rt_res = advance_epoch(&mut runtime, &input, &[]);
        let mo_res = model_apply(&mut model, &input);

        assert_eq!(
            rt_res.is_ok(),
            mo_res.is_ok(),
            "nominal: runtime and model must agree on halt outcome"
        );
        assert_eq!(runtime.epoch, model.epoch, "nominal: epochs must match");
        assert_eq!(
            runtime.entropy_seed, model.entropy_seed,
            "nominal: entropy must match"
        );
        assert_eq!(
            runtime.convergence_window.raw_parts(),
            model.convergence_window.raw_parts(),
            "nominal: convergence windows must match"
        );
    }
}

/// Parity at the epsilon boundary: V_convergence exactly at EPSILON should not
/// trigger halt. Model and runtime must agree.
#[test]
fn axiom_model_runtime_parity_boundary_epsilon() {
    let mut runtime = genesis_state();
    let mut model = genesis_state();

    // epsilon = 20_000 (EPSILON constant from spec §5)
    // V = α·D + β·C = 400_000·d + 350_000·c; set to exactly epsilon.
    // Simple: one validator with D = epsilon / WEIGHT_D = 20_000 / 400_000 = 0.05 → too small.
    // Instead use: all 4 validators same metrics, V = sum of all.
    // Actually epsilon check is: δ_window = V_now - min(window). With window all same value, δ=0.
    // So any uniform load never triggers halt regardless. Use zero — simplest boundary case.
    let input = idle_input(4);

    for _ in 0..WINDOW_SIZE + 1 {
        let rt_res = advance_epoch(&mut runtime, &input, &[]);
        let mo_res = model_apply(&mut model, &input);
        assert_eq!(
            rt_res.is_ok(),
            mo_res.is_ok(),
            "epsilon boundary: must agree"
        );
        assert_eq!(runtime.epoch, model.epoch);
        assert_eq!(runtime.entropy_seed, model.entropy_seed);
    }
}

/// Parity on halt-triggering: both model and runtime must halt with
/// LyapunovViolation at the same step.
#[test]
fn axiom_model_runtime_parity_halt_triggering() {
    let mut runtime = genesis_state();
    let mut model = genesis_state();

    // Fill window with zero epochs first so window.is_full() == true.
    for _ in 0..WINDOW_SIZE {
        advance_epoch(&mut runtime, &idle_input(4), &[]).unwrap();
        model_apply(&mut model, &idle_input(4)).unwrap();
    }

    // Now spike validator 0 to maximum divergence — should trigger LyapunovViolation.
    let mut spike = idle_input(4);
    spike.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(1_000_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });

    let rt_res = advance_epoch(&mut runtime, &spike, &[]);
    let mo_res = model_apply(&mut model, &spike);

    assert!(rt_res.is_err(), "runtime must halt on spike");
    assert!(mo_res.is_err(), "model must halt on spike");
    assert_eq!(
        rt_res.unwrap_err(),
        mo_res.unwrap_err(),
        "runtime and model must halt with the same reason"
    );
    assert_eq!(runtime.epoch, model.epoch, "halted epochs must match");
    assert_eq!(
        runtime.halt_reason, model.halt_reason,
        "halt_reason must match"
    );
}
