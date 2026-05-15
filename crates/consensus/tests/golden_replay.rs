use sha3::{Digest, Sha3_256};

use qash_consensus::hash::DomainTag;
use qash_consensus::params::consensus_params_hash;
use qash_consensus::transition::{
    advance_epoch, decode_full_state, encode_full_state_into,
    EpochInput, EpochState, HaltReason, ValidatorUpdate,
    FULL_STATE_MAX_BYTES, MAX_VALIDATORS,
};
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::fixed_point::{FixedPoint, SCALE};

use proptest::prelude::*;

const EXPECTED_PARAMS_HASH_V0: [u8; 32] = [
    56, 29, 201, 142, 216, 6, 210, 169, 115, 237, 60, 131, 127, 134, 88, 115,
    154, 7, 20, 52, 92, 236, 129, 14, 173, 186, 52, 21, 59, 190, 112, 2,
];

fn genesis_state() -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: 4,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        state_root: [0u8; 32],
    }
}

fn idle_input(n: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        update_count: n,
    }
}

fn hash_window(hasher: &mut Sha3_256, window: &ConvergenceWindow) {
    let (filled, values) = window.raw_parts();
    hasher.update([filled, 0x00, 0x00, 0x00]);
    for v in values.iter() {
        hasher.update(v.raw().to_le_bytes());
    }
}

fn state_fingerprint(state: &EpochState) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update((DomainTag::InternalHash as u32).to_le_bytes());

    hasher.update(state.epoch.to_le_bytes());
    hasher.update(state.validator_count.to_le_bytes());
    hasher.update([state.halt_reason as u8, 0x00, 0x00, 0x00]);
    hasher.update(state.entropy_seed);

    for i in 0..state.validator_count as usize {
        let v = &state.validators[i];
        hasher.update(v.divergence.raw().to_le_bytes());
        hasher.update(v.conflict.raw().to_le_bytes());
        hasher.update(v.slash_accum.raw().to_le_bytes());
        hasher.update(state.nonces[i].to_le_bytes());
    }

    hash_window(&mut hasher, &state.convergence_window);
    hasher.update(state.state_root);

    let out = hasher.finalize();
    let mut res = [0u8; 32];
    res.copy_from_slice(&out);
    res
}

#[test]
fn golden_params_gate_v0() {
    let actual = consensus_params_hash();
    assert_eq!(
        actual, EXPECTED_PARAMS_HASH_V0,
        "consensus parameters changed — regenerate golden vectors / update EXPECTED_PARAMS_HASH_V0"
    );
}

#[test]
fn halt_freezes_entire_state_except_halt_reason() {
    let mut state = genesis_state();

    // Fill window
    for _ in 0..WINDOW_SIZE {
        let input = idle_input(state.validator_count);
        let r = advance_epoch(&mut state, &input);
        assert!(r.is_ok());
    }

    // Fingerprint before spike
    let fp_before = state_fingerprint(&state);

    // Spike (must halt once window full)
    let mut spike = idle_input(state.validator_count);
    spike.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(1_000_000),
        conflict_new: FixedPoint::ZERO,
        // absolute: monotone
        slash_accum_new: FixedPoint::ZERO,
    });

    let res = advance_epoch(&mut state, &spike);
    assert_eq!(res, Err(HaltReason::LyapunovViolation));

    // Temporarily reset halt_reason to compare all other fields
    let stored = state.halt_reason;
    state.halt_reason = HaltReason::None;
    let fp_after = state_fingerprint(&state);
    state.halt_reason = stored;

    assert_eq!(fp_before, fp_after, "state mutated during failed transition");
}

#[test]
fn golden_halt_reason_preserved() {
    let mut state = genesis_state();

    // Fill window
    for _ in 0..WINDOW_SIZE {
        let n = state.validator_count;
        let r = advance_epoch(&mut state, &idle_input(n));
        assert!(r.is_ok());
    }

    // Trigger halt
    let mut spike = idle_input(state.validator_count);
    spike.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(1_000_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });

    assert_eq!(advance_epoch(&mut state, &spike), Err(HaltReason::LyapunovViolation));

    // Subsequent calls return SAME reason and do not mutate.
    let fp = state_fingerprint(&state);
    for _ in 0..10 {
        let n = state.validator_count;
        let r = advance_epoch(&mut state, &idle_input(n));
        assert_eq!(r, Err(HaltReason::LyapunovViolation));
        assert_eq!(state_fingerprint(&state), fp);
    }
}

#[test]
fn window_check_precedes_push() {
    let mut state = genesis_state();

    // Fill window with idle epochs (V_convergence=0)
    for _ in 0..WINDOW_SIZE {
        let n = state.validator_count;
        let r = advance_epoch(&mut state, &idle_input(n));
        assert!(r.is_ok());
    }
    assert!(state.convergence_window.is_full());

    let fp_before = state_fingerprint(&state);

    // Spike: must halt, and must not push the spike into the window (no mutation leak).
    let mut spike = idle_input(state.validator_count);
    spike.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(1_000_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });

    assert_eq!(advance_epoch(&mut state, &spike), Err(HaltReason::LyapunovViolation));

    // Reset halt reason for fingerprint equality comparison
    let stored = state.halt_reason;
    state.halt_reason = HaltReason::None;
    assert_eq!(state_fingerprint(&state), fp_before);
    state.halt_reason = stored;
}

#[test]
fn within_epsilon_does_not_halt() {
    let mut state = genesis_state();

    // Fill window with small nonzero V_convergence
    for _ in 0..WINDOW_SIZE {
        let mut input = idle_input(state.validator_count);
        for j in 0..state.validator_count as usize {
            input.updates[j] = Some(ValidatorUpdate {
                divergence_new: FixedPoint::from_raw(10_000),
                conflict_new: FixedPoint::ZERO,
                slash_accum_new: FixedPoint::ZERO,
            });
        }
        let r = advance_epoch(&mut state, &input);
        assert!(r.is_ok());
    }

    // Slight bump within epsilon
    let mut input = idle_input(state.validator_count);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(15_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });

    let r = advance_epoch(&mut state, &input);
    assert!(r.is_ok());
}

// ---------------------------------------------------------------------------
// Full-state encoding roundtrip tests (Item 1)
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_full_state_empty_validators() {
    let mut state = genesis_state();
    state.validator_count = 0;

    let mut buf = [0u8; FULL_STATE_MAX_BYTES];
    let len = encode_full_state_into(&mut state, &mut buf);

    let decoded = decode_full_state(&buf[..len]).expect("decode failed");

    assert_eq!(decoded.epoch, state.epoch);
    assert_eq!(decoded.halt_reason, state.halt_reason);
    assert_eq!(decoded.entropy_seed, state.entropy_seed);
    assert_eq!(decoded.validator_count, state.validator_count);
    assert_eq!(decoded.state_root, state.state_root);
}

#[test]
fn roundtrip_full_state_four_validators() {
    let mut state = genesis_state();
    state.validators[0] = ValidatorMetrics {
        divergence:  FixedPoint::from_raw(500_000),
        conflict:    FixedPoint::from_raw(250_000),
        slash_accum: FixedPoint::from_raw(1_000),
    };
    state.nonces[0] = 42;

    let mut buf = [0u8; FULL_STATE_MAX_BYTES];
    let len = encode_full_state_into(&mut state, &mut buf);

    let decoded = decode_full_state(&buf[..len]).expect("decode failed");

    assert_eq!(decoded.epoch, state.epoch);
    assert_eq!(decoded.validator_count, state.validator_count);
    assert_eq!(
        decoded.validators[0].divergence.raw(),
        state.validators[0].divergence.raw()
    );
    assert_eq!(
        decoded.validators[0].conflict.raw(),
        state.validators[0].conflict.raw()
    );
    assert_eq!(
        decoded.validators[0].slash_accum.raw(),
        state.validators[0].slash_accum.raw()
    );
    assert_eq!(decoded.nonces[0], 42);
}

#[test]
fn roundtrip_full_state_with_window() {
    let mut state = genesis_state();

    // Advance three epochs to fill the window.
    for _ in 0..WINDOW_SIZE {
        let n = state.validator_count;
        advance_epoch(&mut state, &idle_input(n)).unwrap();
    }

    let mut buf = [0u8; FULL_STATE_MAX_BYTES];
    let len = encode_full_state_into(&mut state, &mut buf);
    let decoded = decode_full_state(&buf[..len]).expect("decode failed");

    assert_eq!(decoded.convergence_window.is_full(), state.convergence_window.is_full());

    let (filled_orig, vals_orig) = state.convergence_window.raw_parts();
    let (filled_dec, vals_dec)   = decoded.convergence_window.raw_parts();
    assert_eq!(filled_orig, filled_dec);
    for i in 0..WINDOW_SIZE {
        assert_eq!(vals_orig[i].raw(), vals_dec[i].raw());
    }
}

#[test]
fn roundtrip_halted_state() {
    let mut state = genesis_state();
    state.halt_reason = HaltReason::LyapunovViolation;

    let mut buf = [0u8; FULL_STATE_MAX_BYTES];
    let len = encode_full_state_into(&mut state, &mut buf);

    let decoded = decode_full_state(&buf[..len]).expect("decode failed");
    assert_eq!(decoded.halt_reason, HaltReason::LyapunovViolation);
}

#[test]
fn state_root_is_deterministic() {
    // Same input must always produce the same state root.
    let mut s1 = genesis_state();
    let mut s2 = genesis_state();

    for _ in 0..WINDOW_SIZE {
        let n = s1.validator_count;
        advance_epoch(&mut s1, &idle_input(n)).unwrap();
        advance_epoch(&mut s2, &idle_input(n)).unwrap();
    }

    assert_eq!(s1.state_root, s2.state_root, "state root is not deterministic");
    assert_ne!(s1.state_root, [0u8; 32], "state root must be non-zero after epochs");
}

#[test]
fn state_root_changes_each_epoch() {
    let mut state = genesis_state();
    let mut prev_root = state.state_root;

    for _ in 0..WINDOW_SIZE {
        let n = state.validator_count;
        advance_epoch(&mut state, &idle_input(n)).unwrap();
        assert_ne!(state.state_root, prev_root, "state root did not change after epoch");
        prev_root = state.state_root;
    }
}

// ---------------------------------------------------------------------------
// Canonical state root golden vector (TH-7 empirical anchor)
// ---------------------------------------------------------------------------
//
// Sequence: genesis (4 validators, all ZERO, seed=[0;32]) → 3 idle epochs.
// This hard-coded root must be identical across x86_64, aarch64, riscv64gc.
// If this test fails after a change to the encoding or hash logic, regenerate
// by running with PRINT_GOLDEN=1 and updating the constant below.
const EXPECTED_STATE_ROOT_3_EPOCHS: [u8; 32] = [
     31,  87, 150, 182, 211, 186,  34, 210,
    199, 112, 230, 159, 156,  68, 106, 132,
    146,  85, 146,  35, 111, 147, 214, 246,
     76,  21, 102, 230, 142,  51,  23, 185,
];

/// Compute the canonical 3-epoch state root for the genesis sequence.
fn canonical_3epoch_root() -> [u8; 32] {
    let mut state = genesis_state();
    for _ in 0..3 {
        let n = state.validator_count;
        advance_epoch(&mut state, &idle_input(n)).unwrap();
    }
    state.state_root
}

#[test]
fn state_root_canonical_seq_print() {
    // Prints the canonical root when run with --nocapture.
    // Used to bootstrap EXPECTED_STATE_ROOT_3_EPOCHS.
    let root = canonical_3epoch_root();
    println!("CANONICAL_STATE_ROOT_3_EPOCHS = {:?}", root);
}

/// TH-7 empirical anchor: canonical 3-epoch state root must be identical
/// across all authorized ISAs (x86_64, aarch64, riscv64gc).
#[test]
fn state_root_canonical_seq_golden() {
    let root = canonical_3epoch_root();
    assert_eq!(
        root, EXPECTED_STATE_ROOT_3_EPOCHS,
        "canonical state root changed — update EXPECTED_STATE_ROOT_3_EPOCHS \
         only after verifying all three ISA targets produce the new value"
    );
}

// ---------------------------------------------------------------------------
// Property-based tests (proptest, Item 4)
// ---------------------------------------------------------------------------

fn arb_bounded_fp() -> impl Strategy<Value = i128> {
    0i128..=SCALE
}

fn arb_nonneg_slash() -> impl Strategy<Value = i128> {
    0i128..=1_000_000i128
}

fn arb_entropy_seed() -> impl Strategy<Value = [u8; 32]> {
    proptest::array::uniform32(any::<u8>())
}

proptest! {
    /// P1: Full-state encoding roundtrip — decode(encode(s)) == s on all relevant fields.
    #[test]
    fn prop_encode_decode_roundtrip(
        vc in 0u32..=4u32,
        d in arb_bounded_fp(),
        c in arb_bounded_fp(),
        s in arb_nonneg_slash(),
        nonce in 0u64..=1000u64,
        halt_byte in 0u8..=6u8,
    ) {
        let mut state = genesis_state();
        state.validator_count = vc;
        let halt_reason = match halt_byte {
            1 => HaltReason::LyapunovViolation,
            2 => HaltReason::ArithOverflow,
            3 => HaltReason::EpochOverflow,
            4 => HaltReason::DecodeInvalid,
            5 => HaltReason::RoundtripFailure,
            6 => HaltReason::HaltFlagSet,
            _ => HaltReason::None,
        };
        state.halt_reason = halt_reason;
        for i in 0..vc as usize {
            state.validators[i] = ValidatorMetrics {
                divergence:  FixedPoint::from_raw(d),
                conflict:    FixedPoint::from_raw(c),
                slash_accum: FixedPoint::from_raw(s),
            };
            state.nonces[i] = nonce;
        }

        let mut buf = [0u8; FULL_STATE_MAX_BYTES];
        let len = encode_full_state_into(&mut state, &mut buf);
        let decoded = decode_full_state(&buf[..len])
            .expect("roundtrip decode must succeed on valid state");

        prop_assert_eq!(decoded.epoch, state.epoch);
        prop_assert_eq!(decoded.validator_count, state.validator_count);
        prop_assert_eq!(decoded.halt_reason, state.halt_reason);
        prop_assert_eq!(decoded.entropy_seed, state.entropy_seed);
        prop_assert_eq!(decoded.state_root, state.state_root);

        for i in 0..vc as usize {
            prop_assert_eq!(
                decoded.validators[i].divergence.raw(),
                state.validators[i].divergence.raw()
            );
            prop_assert_eq!(
                decoded.validators[i].conflict.raw(),
                state.validators[i].conflict.raw()
            );
            prop_assert_eq!(
                decoded.validators[i].slash_accum.raw(),
                state.validators[i].slash_accum.raw()
            );
            prop_assert_eq!(decoded.nonces[i], nonce);
        }
    }

    /// P2: State root is deterministic — identical inputs always produce identical output.
    #[test]
    fn prop_state_root_deterministic(seed in arb_entropy_seed()) {
        let make_state = |seed: [u8; 32]| -> EpochState {
            let mut s = genesis_state();
            s.entropy_seed = seed;
            s
        };

        let mut s1 = make_state(seed);
        let mut s2 = make_state(seed);

        for _ in 0..WINDOW_SIZE {
            let n = s1.validator_count;
            advance_epoch(&mut s1, &idle_input(n)).unwrap();
            advance_epoch(&mut s2, &idle_input(n)).unwrap();
        }

        prop_assert_eq!(s1.state_root, s2.state_root);
    }

    /// P3: Halt is absorbing — advance_epoch on a halted state never changes halt_reason.
    #[test]
    fn prop_halt_is_absorbing(halt_byte in 1u8..=6u8) {
        let halt_reason = match halt_byte {
            1 => HaltReason::LyapunovViolation,
            2 => HaltReason::ArithOverflow,
            3 => HaltReason::EpochOverflow,
            4 => HaltReason::DecodeInvalid,
            5 => HaltReason::RoundtripFailure,
            _ => HaltReason::HaltFlagSet,
        };
        let mut state = genesis_state();
        state.halt_reason = halt_reason;

        for _ in 0..5 {
            let n = state.validator_count;
            let r = advance_epoch(&mut state, &idle_input(n));
            prop_assert_eq!(r, Err(halt_reason));
            prop_assert_eq!(state.halt_reason, halt_reason);
        }
    }

    /// P4: FixedPoint checked_add of two non-negative values produces a non-negative result.
    #[test]
    fn prop_fixed_point_add_nonneg(a in 0i128..SCALE, b in 0i128..SCALE) {
        let fa = FixedPoint::from_raw(a);
        let fb = FixedPoint::from_raw(b);
        if let Ok(r) = fa.checked_add(fb) {
            prop_assert!(r.raw() >= 0);
        }
    }

    /// P5: Lyapunov V_convergence and Φ_safety are non-negative for bounded metrics.
    #[test]
    fn prop_lyapunov_nonneg(d in 0i128..=SCALE, c in 0i128..=SCALE, s in 0i128..=SCALE) {
        use qash_consensus::lyapunov::{evaluate, ValidatorMetrics as VM};
        let metrics = VM {
            divergence:  FixedPoint::from_raw(d),
            conflict:    FixedPoint::from_raw(c),
            slash_accum: FixedPoint::from_raw(s),
        };
        let eval = evaluate(&[metrics], &ConvergenceWindow::new())
            .expect("evaluate must succeed on bounded metrics");
        prop_assert!(eval.v_convergence.raw() >= 0);
        prop_assert!(eval.phi_safety.raw() >= 0);
        prop_assert!(eval.v_total.raw() >= 0);
    }

    /// P6: Encoding roundtrip preserves convergence window fill count.
    #[test]
    fn prop_window_roundtrip(
        v0 in 0i128..=500_000i128,
        v1 in 0i128..=500_000i128,
        filled in 0u8..=3u8,
    ) {
        let mut state = genesis_state();
        // Manually push exactly `filled` values into the window.
        let push_vals = [v0, v1, 0i128];
        for i in 0..filled as usize {
            state.convergence_window.push(FixedPoint::from_raw(push_vals[i]));
        }

        let mut buf = [0u8; FULL_STATE_MAX_BYTES];
        let len = encode_full_state_into(&mut state, &mut buf);
        let decoded = decode_full_state(&buf[..len]).expect("decode failed");

        let (orig_filled, _) = state.convergence_window.raw_parts();
        let (dec_filled, _)  = decoded.convergence_window.raw_parts();
        prop_assert_eq!(orig_filled, dec_filled);
    }
}
