use sha3::{Digest, Sha3_256};

use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::hash::DomainTag;
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::params::consensus_params_hash;
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

const EXPECTED_PARAMS_HASH_V0: [u8; 32] = [
    67, 252, 243, 98, 176, 70, 79, 168, 182, 190, 200, 22, 3, 42, 115, 220, 153, 216, 120, 206,
    181, 220, 212, 221, 138, 120, 5, 76, 102, 32, 195, 233,
];

fn genesis_state() -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: 4,
        convergence_window: ConvergenceWindow::new(),
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
    }

    hash_window(&mut hasher, &state.convergence_window);

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

    // Temporarily reset halt_reason to compare all other fields.
    state.halt_reason = HaltReason::None;
    let fp_after = state_fingerprint(&state);

    assert_eq!(
        fp_before, fp_after,
        "state mutated during failed transition"
    );
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

    assert_eq!(
        advance_epoch(&mut state, &spike),
        Err(HaltReason::LyapunovViolation)
    );

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

    assert_eq!(
        advance_epoch(&mut state, &spike),
        Err(HaltReason::LyapunovViolation)
    );

    // Reset halt reason for fingerprint equality comparison.
    state.halt_reason = HaltReason::None;
    assert_eq!(state_fingerprint(&state), fp_before);
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

#[test]
fn transition_root_folds_all_active_primitives() {
    let mut state = genesis_state();
    let result = advance_epoch(&mut state, &idle_input(4)).expect("idle epoch advances");

    assert_eq!(
        result.state_root,
        qash_consensus::hash::combine_primitive_digests(&result.primitive_roots)
    );

    let mut changed = result.primitive_roots;
    changed[0].digest[0] ^= 0x01;
    assert_ne!(
        result.state_root,
        qash_consensus::hash::combine_primitive_digests(&changed)
    );

    changed = result.primitive_roots;
    changed[1].digest[0] ^= 0x01;
    assert_ne!(
        result.state_root,
        qash_consensus::hash::combine_primitive_digests(&changed)
    );
}
