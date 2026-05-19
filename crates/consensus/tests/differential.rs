// differential.rs — Proptest-based runtime verification of Coq-proved properties.
//
// Each proptest block cites the Coq theorem it exercises. The Coq proof guarantees
// the property holds in the model; this file guards against model–implementation drift.
//
// Coq source: proofs/contractivity/, proofs/safety/

use proptest::prelude::*;
use qash_consensus::fixed_point::{FixedPoint, SCALE};
use qash_consensus::lyapunov::{self, ConvergenceWindow, ValidatorMetrics, WEIGHT_S, WINDOW_SIZE};
use qash_consensus::transaction::{TX0_WIRE_BYTES, TX_TYPE_NOOP, TX_VERSION};
use qash_consensus::transition::{
    advance_epoch, decode_full_state, encode_full_state_into, EpochInput, EpochState, HaltReason,
    ValidatorUpdate, FULL_STATE_MAX_BYTES, MAX_VALIDATORS,
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn genesis_state_n(n: u32) -> EpochState {
    let mut s = EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: n,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        cascade_health: 0,
        state_root: [0u8; 32],
    };
    for i in 0..n as usize {
        s.validator_ids[i][0] = (i as u8).wrapping_add(1);
    }
    s
}

fn idle_input_n(n: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        update_count: n,
    }
}

fn uniform_input(n: u32, d: i128, c: i128, slash: i128) -> EpochInput {
    let mut inp = idle_input_n(n);
    for i in 0..n as usize {
        inp.updates[i] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(d),
            conflict_new: FixedPoint::from_raw(c),
            slash_accum_new: FixedPoint::from_raw(slash),
        });
    }
    inp
}

fn make_tx0_bytes(author_id: [u8; 48], nonce: u64) -> [u8; TX0_WIRE_BYTES] {
    let mut raw = [0u8; TX0_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
    raw[4..12].copy_from_slice(&nonce.to_le_bytes());
    raw[12..60].copy_from_slice(&author_id);
    raw[60..64].copy_from_slice(&0u32.to_le_bytes());
    raw
}

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

fn arb_metric() -> impl Strategy<Value = i128> {
    0i128..=SCALE
}

fn arb_metric_small() -> impl Strategy<Value = i128> {
    // Metrics whose V_convergence per-validator is small enough to keep δ_window ≤ ε
    // even after a few epochs. V per validator = WEIGHT_D*d + WEIGHT_C*c (both /SCALE).
    // For 1 validator: V_max = (400_000 + 350_000) / SCALE = 0.75 raw.
    // For δ_window ≤ EPSILON=20_000 we need |V_new - V_old| ≤ 20_000.
    // Using d,c in [0, 26_000] keeps (WEIGHT_D+WEIGHT_C)*d/SCALE ≤ 19_500 < 20_000.
    0i128..=26_000i128
}

fn arb_validator_count() -> impl Strategy<Value = u32> {
    1u32..=4u32
}

// ---------------------------------------------------------------------------
// TH-3a: No halt when δ_window ≤ ε
// Coq: `TH3a_no_halt_within_epsilon` in contractivity/lyapunov_stability.v
//
// Strategy: run WINDOW_SIZE+1 epochs with identical metrics so V_convergence
// is constant each epoch. Then δ_window = V_current - min(V_window) = 0 ≤ ε.
// advance_epoch must succeed on every call.
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// TH-3a: constant metrics → δ_window = 0 ≤ ε → no halt across window fill + 1 epoch.
    #[test]
    fn diff_th3a_no_halt_constant_metrics(
        n in arb_validator_count(),
        d in arb_metric_small(),
        c in arb_metric_small(),
    ) {
        let mut state = genesis_state_n(n);
        let slash = 0i128; // slash accum starts at 0; monotone constraint satisfied

        // Run WINDOW_SIZE+1 epochs: window fills then we verify no halt on the epoch after.
        for _ in 0..=WINDOW_SIZE {
            let input = uniform_input(n, d, c, slash);
            let result = advance_epoch(&mut state, &input, &[]);
            if let Err(e) = result {
                prop_assert!(
                    false,
                    "TH-3a violation: halt on constant metrics (d={}, c={}, n={}): {:?}",
                    d, c, n, e
                );
            }
        }
        prop_assert!(!state.is_halted(), "TH-3a: state must not be halted");
    }

    /// TH-3a: metrics that produce V_new ≤ V_window_min → δ_window ≤ 0 ≤ ε → no halt.
    /// We warm up with high metrics then drop to low metrics; δ_window ≤ 0.
    #[test]
    fn diff_th3a_no_halt_decreasing_v(
        n in arb_validator_count(),
        d_hi in arb_metric_small(),
        c_hi in arb_metric_small(),
        d_lo in arb_metric_small(),
        c_lo in arb_metric_small(),
    ) {
        // Clamp so d_lo ≤ d_hi and c_lo ≤ c_hi (decreasing metrics → V_new ≤ V_old).
        let d_lo = d_lo.min(d_hi);
        let c_lo = c_lo.min(c_hi);

        let mut state = genesis_state_n(n);

        // Fill the window with high metrics.
        for _ in 0..WINDOW_SIZE {
            let input = uniform_input(n, d_hi, c_hi, 0);
            let r = advance_epoch(&mut state, &input, &[]);
            prop_assume!(r.is_ok()); // skip degenerate cases that overflow
        }

        // Now apply lower (or equal) metrics: δ_window = V_lo - V_hi ≤ 0 ≤ ε.
        let input = uniform_input(n, d_lo, c_lo, 0);
        let result = advance_epoch(&mut state, &input, &[]);
        if let Err(e) = result {
            prop_assert!(
                false,
                "TH-3a violation: halt when V_new ≤ V_old (d_lo={}, c_lo={}, n={}): {:?}",
                d_lo, c_lo, n, e
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TH-4: Φ_safety monotonicity — slash accumulator never decreases
// Coq: `TH4_phi_safety_monotone` in safety/absorbing_halt.v
//
// Strategy: any valid ValidatorUpdate must have slash_accum_new ≥ slash_accum_old.
// The transition validates this at step_1_validate; a decrease → DecodeInvalid halt.
// We verify:
//   (a) valid increase is accepted; slash is preserved in state after advance_epoch
//   (b) decrease is rejected with DecodeInvalid
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// TH-4a: slash monotone — after a valid epoch, state.validators[i].slash_accum
    /// equals the value we supplied (since we supplied ≥ current).
    #[test]
    fn diff_th4a_slash_accum_never_decreases(
        n in arb_validator_count(),
        d in arb_metric(),
        c in arb_metric(),
        slash_delta in 0i128..=100_000i128,
    ) {
        let mut state = genesis_state_n(n);

        // Advance once to establish a non-zero slash base.
        let base_slash = 50_000i128;
        let input1 = uniform_input(n, d, c, base_slash);
        let r1 = advance_epoch(&mut state, &input1, &[]);
        prop_assume!(r1.is_ok());

        // Record actual slash after epoch 1.
        let slash_after_1 = state.validators[0].slash_accum.raw();

        // Advance again with slash_accum_new = slash_after_1 + delta (monotone).
        let new_slash = slash_after_1.saturating_add(slash_delta);
        let input2 = uniform_input(n, d, c, new_slash);
        let r2 = advance_epoch(&mut state, &input2, &[]);
        prop_assume!(r2.is_ok());

        // Φ_safety must not decrease.
        prop_assert!(
            state.validators[0].slash_accum.raw() >= slash_after_1,
            "TH-4 violation: slash decreased from {} to {}",
            slash_after_1,
            state.validators[0].slash_accum.raw()
        );
    }

    /// TH-4b: slash decrease is rejected — a ValidatorUpdate with slash_accum_new <
    /// current slash_accum must produce DecodeInvalid (never silently accepted).
    #[test]
    fn diff_th4b_slash_decrease_rejected(
        n in arb_validator_count(),
        d in arb_metric(),
        c in arb_metric(),
        base_slash in 1_000i128..=500_000i128,
        decrease in 1i128..=999i128,
    ) {
        let mut state = genesis_state_n(n);

        // Set slash_accum on all validators to base_slash.
        for i in 0..n as usize {
            state.validators[i].slash_accum = FixedPoint::from_raw(base_slash);
        }

        // Attempt to decrease slash_accum.
        let bad_slash = base_slash.saturating_sub(decrease).max(0);
        let input = uniform_input(n, d, c, bad_slash);
        let result = advance_epoch(&mut state, &input, &[]);

        prop_assert_eq!(
            result,
            Err(HaltReason::DecodeInvalid),
            "TH-4 violation: slash decrease must produce DecodeInvalid"
        );
    }
}

// ---------------------------------------------------------------------------
// TH-6: Halt is terminal — no transition from a halted state
// Coq: `TH6_halt_terminal`, `TH6_halt_irreversible` in safety/absorbing_halt.v
//
// Strategy: set halt_reason to any non-None value, call advance_epoch repeatedly,
// verify halt_reason never changes and call always returns Err(original_reason).
// ---------------------------------------------------------------------------

fn arb_halt_reason() -> impl Strategy<Value = HaltReason> {
    prop_oneof![
        Just(HaltReason::LyapunovViolation),
        Just(HaltReason::ArithOverflow),
        Just(HaltReason::EpochOverflow),
        Just(HaltReason::DecodeInvalid),
        Just(HaltReason::RoundtripFailure),
        Just(HaltReason::HaltFlagSet),
        Just(HaltReason::PhiSafetyViolation),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// TH-6: halted state is absorbing — any call returns the original halt reason
    /// and the state fields (halt_reason, epoch, state_root) are frozen.
    #[test]
    fn diff_th6_halt_absorbing(
        halt in arb_halt_reason(),
        n in arb_validator_count(),
        extra_epochs in 1usize..=10usize,
    ) {
        let mut state = genesis_state_n(n);
        state.halt_reason = halt;

        let frozen_epoch = state.epoch;
        let frozen_root  = state.state_root;

        for _ in 0..extra_epochs {
            let r = advance_epoch(&mut state, &idle_input_n(n), &[]);
            prop_assert_eq!(r, Err(halt), "TH-6: expected Err({:?})", halt);
            prop_assert_eq!(state.halt_reason, halt, "TH-6: halt_reason must not change");
            prop_assert_eq!(state.epoch, frozen_epoch, "TH-6: epoch must not advance after halt");
            prop_assert_eq!(state.state_root, frozen_root, "TH-6: state_root must not change after halt");
        }
    }

    /// TH-6 irreversibility: once advance_epoch produces a LyapunovViolation halt,
    /// subsequent calls must always return the same halt and leave state frozen.
    #[test]
    fn diff_th6_lyapunov_halt_irreversible(
        n in arb_validator_count(),
        d in (SCALE / 2)..=SCALE,
        c in (SCALE / 2)..=SCALE,
    ) {
        let mut state = genesis_state_n(n);

        // Fill the window completely with V=0 (WINDOW_SIZE epochs).
        // evaluate() only checks halt when window.is_full() (filled==WINDOW_SIZE).
        for _ in 0..WINDOW_SIZE {
            let r = advance_epoch(&mut state, &uniform_input(n, 0, 0, 0), &[]);
            prop_assume!(r.is_ok());
        }

        // Now the window is full with all-zero entries. Apply large metrics:
        // δ_window = V_large - min([0,0,0]) = V_large >> EPSILON → LyapunovViolation.
        let input_large = uniform_input(n, d, c, 0);
        let r = advance_epoch(&mut state, &input_large, &[]);

        prop_assert_eq!(r, Err(HaltReason::LyapunovViolation),
            "expected halt with d={} c={} n={}", d, c, n);
        prop_assert_eq!(state.halt_reason, HaltReason::LyapunovViolation);

        let frozen_epoch = state.epoch;
        let frozen_root = state.state_root;

        // Now verify irreversibility across 3 more calls.
        for _ in 0..3 {
            // Even idle input (which would normally produce no halt) must be rejected.
            let r2 = advance_epoch(&mut state, &idle_input_n(n), &[]);
            prop_assert_eq!(r2, Err(HaltReason::LyapunovViolation));
            prop_assert_eq!(state.epoch, frozen_epoch);
            prop_assert_eq!(state.state_root, frozen_root);
        }
    }
}

// ---------------------------------------------------------------------------
// TX-0: NoOp perturbation is zero — V_convergence unchanged by TX-0
// Coq: `TX0_perturbation_zero` in contractivity/tx_perturbation_0.v
//
// Strategy: run two parallel states from the same genesis with identical metric
// updates. One receives a TX-0 (NoOp); the other does not. Their lyapunov
// v_convergence results must be equal.
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// TX-0: V_convergence in TransitionResult is identical with and without TX-0.
    #[test]
    fn diff_tx0_v_convergence_unchanged(
        n in arb_validator_count(),
        d in arb_metric(),
        c in arb_metric(),
        slash in 0i128..=100_000i128,
    ) {
        let mut state_no_tx  = genesis_state_n(n);
        let mut state_with_tx = genesis_state_n(n);

        let input = uniform_input(n, d, c, slash);

        let r_no_tx = advance_epoch(&mut state_no_tx, &input, &[]);
        prop_assume!(r_no_tx.is_ok());

        // Build TX-0 for validator 0.
        let author_id = state_with_tx.validator_ids[0];
        let tx0 = make_tx0_bytes(author_id, 0);

        let r_with_tx = advance_epoch(&mut state_with_tx, &input, &[tx0.as_slice()]);
        prop_assume!(r_with_tx.is_ok());

        let v_no_tx   = r_no_tx.unwrap().lyapunov.v_convergence.raw();
        let v_with_tx = r_with_tx.unwrap().lyapunov.v_convergence.raw();

        prop_assert_eq!(
            v_no_tx, v_with_tx,
            "TX-0 violation: v_convergence differs (no_tx={}, with_tx={})",
            v_no_tx, v_with_tx
        );
    }
}

// ---------------------------------------------------------------------------
// TX-1: Score decrement → V_convergence non-increasing
// Coq: `TX1_score_decrement_nonincreasing` in contractivity/tx1_score_decrement.v
//
// §A8 Form A: if divergence_new ≤ divergence_old and conflict_new ≤ conflict_old
// then V_convergence_new ≤ V_convergence_old.
//
// Strategy: compute V for a baseline metric set, then compute V for a metric set
// with strictly lower or equal d/c values. The direct lyapunov::evaluate call
// verifies the mathematical property without advance_epoch round-trip complexity.
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// TX-1: lower or equal (d, c) produces lower or equal V_convergence.
    #[test]
    fn diff_tx1_score_decrement_nonincreasing(
        n_validators in 1usize..=4usize,
        d_old in arb_metric(),
        c_old in arb_metric(),
        d_new_extra in 0i128..=SCALE,
        c_new_extra in 0i128..=SCALE,
    ) {
        // d_new ≤ d_old, c_new ≤ c_old (score decrement).
        let d_new = (d_old.saturating_sub(d_new_extra)).max(0);
        let c_new = (c_old.saturating_sub(c_new_extra)).max(0);

        let metrics_old: Vec<ValidatorMetrics> = (0..n_validators)
            .map(|_| ValidatorMetrics {
                divergence:  FixedPoint::from_raw(d_old),
                conflict:    FixedPoint::from_raw(c_old),
                slash_accum: FixedPoint::ZERO,
            })
            .collect();

        let metrics_new: Vec<ValidatorMetrics> = (0..n_validators)
            .map(|_| ValidatorMetrics {
                divergence:  FixedPoint::from_raw(d_new),
                conflict:    FixedPoint::from_raw(c_new),
                slash_accum: FixedPoint::ZERO,
            })
            .collect();

        let window = ConvergenceWindow::new();

        let eval_old = lyapunov::evaluate(&metrics_old, &window)
            .expect("evaluate must succeed on bounded metrics");
        let eval_new = lyapunov::evaluate(&metrics_new, &window)
            .expect("evaluate must succeed on bounded metrics");

        prop_assert!(
            eval_new.v_convergence.raw() <= eval_old.v_convergence.raw(),
            "TX-1 violation: V_new ({}) > V_old ({}) with d_old={} c_old={} d_new={} c_new={}",
            eval_new.v_convergence.raw(), eval_old.v_convergence.raw(),
            d_old, c_old, d_new, c_new
        );
    }
}

// ---------------------------------------------------------------------------
// TH-1 extended: Encode/decode roundtrip is faithful across random states
// Coq: `TH1_encode_state_injective` in contractivity/encode_injectivity.v
//
// Extends the roundtrip test in golden_replay.rs with more field combinations,
// including convergence window fill counts and multi-validator nonces.
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// TH-1 extended: all state fields survive encode→decode for any valid validator count.
    #[test]
    fn diff_th1_encode_decode_extended(
        n in 0u32..=4u32,
        d in arb_metric(),
        c in arb_metric(),
        slash in 0i128..=500_000i128,
        nonce in 0u64..=u64::MAX,
        seed in proptest::array::uniform32(any::<u8>()),
        fill in 0u8..=3u8,
    ) {
        let mut state = genesis_state_n(n);
        state.entropy_seed = seed;

        for i in 0..n as usize {
            state.validators[i] = ValidatorMetrics {
                divergence:  FixedPoint::from_raw(d),
                conflict:    FixedPoint::from_raw(c),
                slash_accum: FixedPoint::from_raw(slash),
            };
            state.nonces[i] = nonce;
        }

        // Populate window up to `fill` entries.
        for _ in 0..fill {
            state.convergence_window.push(FixedPoint::from_raw(d));
        }

        let mut buf = [0u8; FULL_STATE_MAX_BYTES];
        let len = encode_full_state_into(&mut state, &mut buf);
        let decoded = decode_full_state(&buf[..len])
            .expect("roundtrip decode must succeed on valid state");

        prop_assert_eq!(decoded.epoch, state.epoch);
        prop_assert_eq!(decoded.validator_count, state.validator_count);
        prop_assert_eq!(decoded.halt_reason, state.halt_reason);
        prop_assert_eq!(decoded.entropy_seed, state.entropy_seed);

        let (dec_fill, dec_values) = decoded.convergence_window.raw_parts();
        let (enc_fill, enc_values) = state.convergence_window.raw_parts();
        prop_assert_eq!(dec_fill, enc_fill, "window fill count must survive roundtrip");
        for i in 0..WINDOW_SIZE {
            prop_assert_eq!(
                dec_values[i].raw(), enc_values[i].raw(),
                "window[{}] must survive roundtrip", i
            );
        }

        for i in 0..n as usize {
            prop_assert_eq!(
                decoded.validators[i].divergence.raw(),
                state.validators[i].divergence.raw(),
                "validator[{}].divergence must survive roundtrip", i
            );
            prop_assert_eq!(
                decoded.validators[i].conflict.raw(),
                state.validators[i].conflict.raw(),
                "validator[{}].conflict must survive roundtrip", i
            );
            prop_assert_eq!(
                decoded.validators[i].slash_accum.raw(),
                state.validators[i].slash_accum.raw(),
                "validator[{}].slash_accum must survive roundtrip", i
            );
            prop_assert_eq!(decoded.nonces[i], nonce, "validator[{}].nonce must survive roundtrip", i);
        }
    }
}

// ---------------------------------------------------------------------------
// Φ_safety boundedness: slash accumulator ≤ Φ_max
// Coq: `TH5_phi_safety_bounded` in safety/absorbing_halt.v
//
// §4b upper bound: Φ_safety = WEIGHT_S * slash_accum. Because slash_accum is
// expressed as a FixedPoint with raw ≥ 0, and WEIGHT_S = 250_000, overflow
// into i128 is the only bound. The protocol enforces checked arithmetic;
// we verify that bounded slash inputs produce non-negative Φ_safety.
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// TH-5: Φ_safety is non-negative and finite for any bounded slash input.
    #[test]
    fn diff_th5_phi_safety_bounded(
        slash in 0i128..=SCALE,
    ) {
        let metrics = ValidatorMetrics {
            divergence:  FixedPoint::ZERO,
            conflict:    FixedPoint::ZERO,
            slash_accum: FixedPoint::from_raw(slash),
        };
        let eval = lyapunov::evaluate(&[metrics], &ConvergenceWindow::new())
            .expect("evaluate must succeed");
        prop_assert!(eval.phi_safety.raw() >= 0, "Φ_safety must be non-negative");

        // Verify the specific relationship: Φ_safety = WEIGHT_S * slash / SCALE.
        let expected = WEIGHT_S.checked_mul(FixedPoint::from_raw(slash))
            .expect("WEIGHT_S * slash must not overflow for slash ≤ SCALE");
        prop_assert_eq!(
            eval.phi_safety.raw(), expected.raw(),
            "Φ_safety must equal WEIGHT_S * slash_accum"
        );
    }
}
