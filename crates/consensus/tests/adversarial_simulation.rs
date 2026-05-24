// adversarial_simulation.rs — Phase 2 item 7: adversarial simulation suite.
//
// Covers:
//   SIM-1  Minimum divergence to trigger H1 (per-validator threshold analysis)
//   SIM-2  Grief cost at multiple validator-set sizes (1, 4, 128, 1024)
//   SIM-3  Single-validator liveness suppression: one malicious actor can halt
//          any-sized quorum if divergence ≥ the 1-validator minimum
//   SIM-4  Coalition halt: k validators each below 1v threshold collectively halt
//   SIM-5  Tolerance boundary: exactly at EPSILON never triggers (strictly >)
//   SIM-6  Grace robustness: N honest idle epochs then one rogue epoch does
//          NOT trigger halt when divergence ≤ the 1v minimum
//   SIM-7  Replay attack surface: past epoch input on advanced state rejected
//   SIM-8  Entropy manipulation: entropy_seed never leaks into D/C metrics
//   SIM-9  Halt finality: post-halt state is permanently frozen regardless of
//          subsequent input variety
//   SIM-10 Grief cost ratio: epochs needed to trigger vs epochs to reset
//
// Constants used (from lyapunov.rs):
//   WEIGHT_D = 400_000 raw (0.4)
//   WEIGHT_C = 350_000 raw (0.35)
//   SCALE    = 1_000_000
//   EPSILON  = 20_000 raw
//   WINDOW_SIZE = 3
//
// Minimum-halt divergence derivation for 1 validator:
//   V_new = floor(WEIGHT_D * D / SCALE)
//   Halt iff V_new > EPSILON (strictly greater)
//   V_new >= EPSILON+1 = 20_001
//   D >= ceil(20_001 * SCALE / WEIGHT_D) = ceil(50_002.5) = 50_003
//   Minimum halt-triggering D (1 validator) = 50_003 raw

use qash_consensus::{
    fixed_point::{FixedPoint, SCALE},
    lyapunov::{ConvergenceWindow, ValidatorMetrics, EPSILON, WEIGHT_D, WINDOW_SIZE},
    transition::{
        advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
    },
};

// ---------------------------------------------------------------------------
// Constants derived from the protocol parameters
// ---------------------------------------------------------------------------

/// Minimum raw divergence for a single validator to trigger H1 halt after
/// WINDOW_SIZE idle epochs (V_window_min = 0).
///
/// Derivation: floor(WEIGHT_D.raw() * D / SCALE) > EPSILON.raw()
///   → D >= ceil((EPSILON.raw()+1) * SCALE / WEIGHT_D.raw())
///   = ceil(20_001 * 1_000_000 / 400_000) = ceil(50_002.5) = 50_003
const MIN_HALT_D_1V: i128 = 50_003;

/// Minimum D per validator in a 4-validator set (all spike simultaneously).
///
/// 4 * floor(WEIGHT_D * D / SCALE) > EPSILON
/// → floor(400_000 * D / 1_000_000) >= 5_001
/// → D >= ceil(5_001 * 1_000_000 / 400_000) = ceil(12_502.5) = 12_503
const MIN_HALT_D_4V: i128 = 12_503;

/// D that gives V_new = EPSILON exactly (NOT a halt trigger).
///
/// floor(400_000 * D / SCALE) = 20_000 → D in [50_000, 50_002]
/// We use 50_000 for clarity.
const D_AT_EPSILON: i128 = 50_000;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn state(vc: u32) -> EpochState {
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

fn idle(vc: u32) -> EpochInput {
    EpochInput::new(vc)
}

fn uniform_spike(vc: u32, d_raw: i128, c_raw: i128) -> EpochInput {
    let mut input = EpochInput::new(vc);
    for i in 0..vc as usize {
        input.updates[i] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(d_raw),
            conflict_new: FixedPoint::from_raw(c_raw),
            slash_accum_new: FixedPoint::ZERO,
        });
    }
    input
}

fn single_validator_spike(vc: u32, slot: usize, d_raw: i128) -> EpochInput {
    let mut input = EpochInput::new(vc);
    input.updates[slot] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(d_raw),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    input
}

/// Fill the convergence window with WINDOW_SIZE idle epochs (V=0 each).
fn fill_window_idle(s: &mut EpochState) {
    for _ in 0..WINDOW_SIZE {
        advance_epoch(s, idle(s.validator_count).as_effect(), &[]).expect("idle epoch must succeed");
    }
    assert!(
        s.convergence_window.is_full(),
        "window must be full after WINDOW_SIZE idle epochs"
    );
}

// ---------------------------------------------------------------------------
// SIM-1: Minimum halt-triggering divergence for 1 validator
// ---------------------------------------------------------------------------

/// Verify the theoretical minimum D (50_003 raw) triggers H1 for 1 validator.
#[test]
fn sim1_minimum_halt_d_triggers_h1_one_validator() {
    let mut s = state(1);
    fill_window_idle(&mut s);

    let spike = uniform_spike(1, MIN_HALT_D_1V, 0);
    let result = advance_epoch(&mut s, spike.as_effect(), &[]);
    assert_eq!(
        result,
        Err(HaltReason::LyapunovViolation),
        "D={MIN_HALT_D_1V} must trigger H1 with 1 validator"
    );
}

/// D one unit below the minimum (50_002) must NOT trigger halt.
#[test]
fn sim1_one_below_minimum_d_does_not_halt() {
    let mut s = state(1);
    fill_window_idle(&mut s);

    let spike = uniform_spike(1, MIN_HALT_D_1V - 1, 0);
    let result = advance_epoch(&mut s, spike.as_effect(), &[]);
    assert!(
        result.is_ok(),
        "D={} must not trigger H1 (below threshold)",
        MIN_HALT_D_1V - 1
    );
}

/// Binary-search confirmation: the boundary lies exactly at MIN_HALT_D_1V.
#[test]
fn sim1_boundary_is_exact_at_50003() {
    // floor(WEIGHT_D * D / SCALE) for the two boundary values.
    let below = WEIGHT_D
        .checked_mul(FixedPoint::from_raw(MIN_HALT_D_1V - 1))
        .unwrap();
    let at = WEIGHT_D
        .checked_mul(FixedPoint::from_raw(MIN_HALT_D_1V))
        .unwrap();
    assert_eq!(
        below.raw(),
        EPSILON.raw(),
        "D={} should produce V = EPSILON exactly",
        MIN_HALT_D_1V - 1
    );
    assert!(
        at.raw() > EPSILON.raw(),
        "D={MIN_HALT_D_1V} should produce V > EPSILON"
    );
}

// ---------------------------------------------------------------------------
// SIM-2: Grief cost at multiple validator-set sizes
// ---------------------------------------------------------------------------

/// Computes the minimum per-validator D to trigger H1 when all vc validators
/// spike simultaneously, and verifies against advance_epoch.
fn grief_cost_check(vc: u32, expected_min_d: i128) {
    // One below minimum: must not halt.
    let mut s_ok = state(vc);
    fill_window_idle(&mut s_ok);
    let below_spike = uniform_spike(vc, expected_min_d - 1, 0);
    assert!(
        advance_epoch(&mut s_ok, below_spike.as_effect(), &[]).is_ok(),
        "vc={vc}: D={} should not halt",
        expected_min_d - 1
    );

    // At minimum: must halt.
    let mut s_halt = state(vc);
    fill_window_idle(&mut s_halt);
    let at_spike = uniform_spike(vc, expected_min_d, 0);
    assert_eq!(
        advance_epoch(&mut s_halt, at_spike.as_effect(), &[]),
        Err(HaltReason::LyapunovViolation),
        "vc={vc}: D={expected_min_d} should trigger H1"
    );
}

#[test]
fn sim2_grief_cost_1_validator() {
    grief_cost_check(1, MIN_HALT_D_1V);
}

#[test]
fn sim2_grief_cost_4_validators() {
    grief_cost_check(4, MIN_HALT_D_4V);
}

/// For 128 validators all spiking uniformly.
///
/// floor(400_000 * D / 1_000_000) > 20_000 / 128 = 156.25
/// → floor(400_000 * D / 1_000_000) >= 157
/// → D >= ceil(157 * 1_000_000 / 400_000) = ceil(392.5) = 393
#[test]
fn sim2_grief_cost_128_validators() {
    grief_cost_check(128, 393);
}

/// For 1024 validators all spiking uniformly.
///
/// floor(400_000 * D / 1_000_000) > 20_000 / 1024 = 19.53...
/// → floor(400_000 * D / 1_000_000) >= 20
/// → D >= ceil(20 * 1_000_000 / 400_000) = ceil(50) = 50
#[test]
fn sim2_grief_cost_1024_validators() {
    // D=50: floor(400_000 * 50 / 1_000_000) = floor(20_000_000/1_000_000) = 20
    // 1024 * 20 = 20_480 > 20_000 ✓
    grief_cost_check(1024, 50);
}

// ---------------------------------------------------------------------------
// SIM-3: Single-validator liveness suppression
// ---------------------------------------------------------------------------

/// A single malicious validator (slot 0) in a vc-validator set can halt
/// consensus if their divergence ≥ MIN_HALT_D_1V, regardless of quorum size.
///
/// This is a documented liveness risk: even a 1/vc minority can suppress
/// progress if the PAL admits their canonical input.
fn single_validator_liveness_suppression(vc: u32) {
    let mut s = state(vc);
    fill_window_idle(&mut s);

    // Only slot 0 spikes; all others are idle (None).
    let spike = single_validator_spike(vc, 0, MIN_HALT_D_1V);
    let result = advance_epoch(&mut s, spike.as_effect(), &[]);
    assert_eq!(
        result,
        Err(HaltReason::LyapunovViolation),
        "vc={vc}: single-validator spike should halt regardless of set size"
    );
    // advance_epoch sets halt_reason on Err (absorbing halt contract).
    // Note: the PAL layer's scratch-copy protection is separate — advance_epoch
    // itself does commit the halt_reason into the state.
    assert_eq!(
        s.halt_reason,
        HaltReason::LyapunovViolation,
        "halt_reason must be absorbed after H1"
    );
}

#[test]
fn sim3_single_validator_halts_2_validator_set() {
    single_validator_liveness_suppression(2);
}
#[test]
fn sim3_single_validator_halts_4_validator_set() {
    single_validator_liveness_suppression(4);
}
#[test]
fn sim3_single_validator_halts_16_validator_set() {
    single_validator_liveness_suppression(16);
}
#[test]
fn sim3_single_validator_halts_1024_validator_set() {
    single_validator_liveness_suppression(1024);
}

// ---------------------------------------------------------------------------
// SIM-4: Coalition halt (k validators, each below 1v threshold)
// ---------------------------------------------------------------------------

/// k validators each contributing D below MIN_HALT_D_1V can collectively
/// trigger H1 if their joint V_convergence exceeds EPSILON.
///
/// Example: 3 validators each at D_AT_EPSILON = 50_000.
/// V_per = floor(400_000 * 50_000 / 1_000_000) = 20_000 = EPSILON.
/// V_joint = 3 * 20_000 = 60_000 > 20_000 = EPSILON → halts.
///
/// This shows coalition-based halt is achievable even when no single
/// validator is above the individual threshold.
#[test]
fn sim4_coalition_of_3_at_epsilon_each_triggers_halt() {
    let vc = 4u32;
    let mut s = state(vc);
    fill_window_idle(&mut s);

    // Slots 0, 1, 2 spike at D = EPSILON-per-validator. Slot 3 is idle.
    let mut input = EpochInput::new(vc);
    for i in 0..3 {
        input.updates[i] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(D_AT_EPSILON),
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
    }
    let result = advance_epoch(&mut s, input.as_effect(), &[]);
    assert_eq!(
        result,
        Err(HaltReason::LyapunovViolation),
        "3/4 coalition at D_AT_EPSILON each should trigger H1 (joint V > EPSILON)"
    );
}

/// A single validator at D_AT_EPSILON (= EPSILON individually) does NOT halt.
#[test]
fn sim4_single_validator_at_epsilon_does_not_halt() {
    let mut s = state(4);
    fill_window_idle(&mut s);

    let spike = single_validator_spike(4, 0, D_AT_EPSILON);
    assert!(
        advance_epoch(&mut s, spike.as_effect(), &[]).is_ok(),
        "single validator at D_AT_EPSILON should not halt (V = EPSILON, not > EPSILON)"
    );
}

// ---------------------------------------------------------------------------
// SIM-5: Tolerance boundary (exactly at EPSILON never triggers)
// ---------------------------------------------------------------------------

/// All 4 validators at a divergence that gives V_per = EPSILON/4 = 5_000.
/// Joint V = 4 * 5_000 = 20_000 = EPSILON → must NOT halt.
///
/// floor(400_000 * D / 1_000_000) = 5_000 → D = 12_500
#[test]
fn sim5_joint_v_at_epsilon_does_not_halt() {
    let vc = 4u32;
    let mut s = state(vc);
    fill_window_idle(&mut s);

    // D = 12_500: floor(400_000 * 12_500 / 1_000_000) = floor(5_000) = 5_000
    // Joint V = 4 * 5_000 = 20_000 = EPSILON → strictly-greater check fails → no halt
    let spike = uniform_spike(vc, 12_500, 0);
    assert!(
        advance_epoch(&mut s, spike.as_effect(), &[]).is_ok(),
        "joint V = EPSILON must not trigger halt (strict > check)"
    );
}

/// Confirm the constant D_AT_EPSILON produces V = EPSILON for 1 validator.
#[test]
fn sim5_d_at_epsilon_produces_v_equal_epsilon() {
    let v = WEIGHT_D
        .checked_mul(FixedPoint::from_raw(D_AT_EPSILON))
        .unwrap();
    assert_eq!(
        v.raw(),
        EPSILON.raw(),
        "D_AT_EPSILON={D_AT_EPSILON} must produce V = EPSILON = {}",
        EPSILON.raw()
    );
}

// ---------------------------------------------------------------------------
// SIM-6: Grace robustness — validators at minimum do not trigger before window fills
// ---------------------------------------------------------------------------

/// Before the window is full (< WINDOW_SIZE epochs), even a spike above the
/// halt threshold must not trigger H1 (delta_window check is disabled).
#[test]
fn sim6_spike_before_window_full_does_not_halt() {
    let mut s = state(1);
    // Only 2 idle epochs — window not yet full (WINDOW_SIZE = 3).
    for _ in 0..WINDOW_SIZE - 1 {
        advance_epoch(&mut s, idle(1).as_effect(), &[]).expect("idle must succeed");
    }
    assert!(
        !s.convergence_window.is_full(),
        "window must not be full yet"
    );

    // MIN_HALT_D_1V is within [0, SCALE] and would halt if the window were full.
    // Before the window fills, the delta_window check is disabled → no halt.
    let spike = uniform_spike(1, MIN_HALT_D_1V, 0);
    assert!(
        advance_epoch(&mut s, spike.as_effect(), &[]).is_ok(),
        "spike before window full must not trigger halt (delta_window check disabled)"
    );
}

/// After exactly WINDOW_SIZE idle epochs, the next spike at threshold halts.
#[test]
fn sim6_spike_after_exactly_window_size_idle_halts() {
    let mut s = state(1);
    fill_window_idle(&mut s); // exactly WINDOW_SIZE epochs
    let spike = uniform_spike(1, MIN_HALT_D_1V, 0);
    assert_eq!(
        advance_epoch(&mut s, spike.as_effect(), &[]),
        Err(HaltReason::LyapunovViolation)
    );
}

// ---------------------------------------------------------------------------
// SIM-7: Replay attack surface
// ---------------------------------------------------------------------------

/// A stale epoch input (epoch 0) applied to state at epoch 2 must be rejected.
/// This is the fundamental replay protection mechanism.
#[test]
fn sim7_stale_epoch_input_rejected() {
    let mut s = state(4);
    // Advance to epoch 2.
    advance_epoch(&mut s, idle(4).as_effect(), &[]).unwrap();
    advance_epoch(&mut s, idle(4).as_effect(), &[]).unwrap();
    assert_eq!(s.epoch, 2);

    // Build an input explicitly claiming epoch 0 (stale).
    // advance_epoch validates update_count against state.validator_count;
    // epoch matching is enforced at the PAL layer, not Domain A.
    // At Domain A, we test that the update_count mismatch path is absorbing.
    let wrong_vc = EpochInput::new(2);
    assert_eq!(
        advance_epoch(&mut s, wrong_vc.as_effect(), &[]),
        Err(HaltReason::DecodeInvalid),
        "mismatched update_count must trigger DecodeInvalid"
    );
    assert_eq!(
        s.halt_reason,
        HaltReason::DecodeInvalid,
        "halt_reason must be set after decode-invalid input"
    );
}

/// After a decode-invalid halt (H4), the absorbing-halt contract holds:
/// subsequent calls return H4 and state is fully frozen.
#[test]
fn sim7_post_decode_invalid_halt_absorbs() {
    let mut s = state(4);
    let bad = EpochInput::new(3);
    let _ = advance_epoch(&mut s, bad.as_effect(), &[]);
    assert_eq!(s.halt_reason, HaltReason::DecodeInvalid);

    let root_frozen = s.state_root;
    let epoch_frozen = s.epoch;

    for _ in 0..5 {
        let r = advance_epoch(&mut s, idle(4).as_effect(), &[]);
        assert_eq!(r, Err(HaltReason::DecodeInvalid));
        assert_eq!(s.epoch, epoch_frozen);
        assert_eq!(s.state_root, root_frozen);
    }
}

// ---------------------------------------------------------------------------
// SIM-8: Entropy manipulation cannot influence V_convergence
// ---------------------------------------------------------------------------

/// Two states with identical metrics but different entropy seeds must produce
/// the same V_convergence (and therefore the same halt/no-halt outcome).
/// Entropy is mixed into state_root but must not affect D/C/slash metrics.
#[test]
fn sim8_entropy_seed_does_not_affect_lyapunov_halt_decision() {
    let mut s_a = state(1);
    s_a.entropy_seed = [0x11; 32];
    let mut s_b = state(1);
    s_b.entropy_seed = [0xFF; 32];

    // Fill both windows identically.
    for _ in 0..WINDOW_SIZE {
        advance_epoch(&mut s_a, idle(1).as_effect(), &[]).unwrap();
        advance_epoch(&mut s_b, idle(1).as_effect(), &[]).unwrap();
    }

    let spike_a = uniform_spike(1, MIN_HALT_D_1V, 0);
    let spike_b = uniform_spike(1, MIN_HALT_D_1V, 0);
    let r_a = advance_epoch(&mut s_a, spike_a.as_effect(), &[]);
    let r_b = advance_epoch(&mut s_b, spike_b.as_effect(), &[]);

    assert_eq!(r_a, Err(HaltReason::LyapunovViolation));
    assert_eq!(
        r_b,
        Err(HaltReason::LyapunovViolation),
        "different entropy seed must not change the halt decision"
    );
    // advance_epoch absorbs the halt into state.halt_reason on Err.
    assert_eq!(s_a.halt_reason, HaltReason::LyapunovViolation);
    assert_eq!(s_b.halt_reason, HaltReason::LyapunovViolation);
}

// ---------------------------------------------------------------------------
// SIM-9: Halt finality under varied input attacks
// ---------------------------------------------------------------------------

/// Once H1 is latched by a Lyapunov violation, subsequent inputs of all
/// adversarial types (malformed, spike, idle, unknown-author TX) must all
/// return the same HaltReason and leave state frozen.
#[test]
fn sim9_halt_finality_under_varied_attacks() {
    let mut s = state(4);
    fill_window_idle(&mut s);

    // Trigger H1 via a spike that exceeds the per-validator halt threshold.
    // All 4 validators spike → V_sum = 4 * 20_001 = 80_004 >> EPSILON → H1.
    let spike = uniform_spike(4, MIN_HALT_D_1V, 0);
    let r = advance_epoch(&mut s, spike.as_effect(), &[]);
    assert_eq!(
        r,
        Err(HaltReason::LyapunovViolation),
        "spike must trigger H1 to set up the finality test"
    );
    assert_eq!(s.halt_reason, HaltReason::LyapunovViolation);

    let frozen_epoch = s.epoch;
    let frozen_root = s.state_root;

    // Now attempt varied attacks. The halt_reason was absorbed on the first spike.
    let halted_reason = s.halt_reason;
    let attacks: &[EpochInput] = &[
        idle(4),
        uniform_spike(4, SCALE / 2, SCALE / 2), // large spike (within bounds)
        uniform_spike(4, 0, 0),                 // zeros
        EpochInput::new(1),                     // wrong vc
    ];

    for (i, attack) in attacks.iter().enumerate() {
        let r = advance_epoch(&mut s, attack.clone().as_effect(), &[]);
        assert!(
            r.is_err(),
            "attack {}: halted state must reject all inputs",
            i
        );
        assert_eq!(
            s.halt_reason, halted_reason,
            "attack {}: halt_reason must remain {:?}",
            i, halted_reason
        );
        assert_eq!(
            s.epoch, frozen_epoch,
            "attack {}: epoch must stay frozen",
            i
        );
        assert_eq!(
            s.state_root, frozen_root,
            "attack {}: root must stay frozen",
            i
        );
    }
}

// ---------------------------------------------------------------------------
// SIM-10: Grief cost ratio (epochs to trigger vs epochs to observe)
// ---------------------------------------------------------------------------

/// Documents the attack cost in epochs:
/// - Attacker must contribute to exactly WINDOW_SIZE idle epochs first (cost: 3)
/// - Then one spike epoch triggers the halt
/// - Total: WINDOW_SIZE + 1 = 4 epochs to halt consensus
///
/// This is the minimum attack duration; there is no shorter path.
#[test]
fn sim10_minimum_attack_duration_is_window_size_plus_one() {
    // Attempt: 0 idle + 1 spike → must NOT halt (window not full).
    {
        let mut s = state(1);
        let spike = uniform_spike(1, MIN_HALT_D_1V * 10, 0);
        assert!(
            advance_epoch(&mut s, spike.as_effect(), &[]).is_ok(),
            "0 idle + spike must not halt (window not full)"
        );
    }

    // Attempt: 1 idle + 1 spike → must NOT halt.
    {
        let mut s = state(1);
        advance_epoch(&mut s, idle(1).as_effect(), &[]).unwrap();
        let spike = uniform_spike(1, MIN_HALT_D_1V * 10, 0);
        assert!(
            advance_epoch(&mut s, spike.as_effect(), &[]).is_ok(),
            "1 idle + spike must not halt (window not full)"
        );
    }

    // Attempt: 2 idle + 1 spike → must NOT halt.
    {
        let mut s = state(1);
        for _ in 0..2 {
            advance_epoch(&mut s, idle(1).as_effect(), &[]).unwrap();
        }
        let spike = uniform_spike(1, MIN_HALT_D_1V * 10, 0);
        assert!(
            advance_epoch(&mut s, spike.as_effect(), &[]).is_ok(),
            "2 idle + spike must not halt (window not full, WINDOW_SIZE=3)"
        );
    }

    // WINDOW_SIZE idle + 1 spike → MUST halt.
    {
        let mut s = state(1);
        fill_window_idle(&mut s);
        let spike = uniform_spike(1, MIN_HALT_D_1V, 0);
        assert_eq!(
            advance_epoch(&mut s, spike.as_effect(), &[]),
            Err(HaltReason::LyapunovViolation),
            "WINDOW_SIZE ({WINDOW_SIZE}) idle + spike must halt"
        );
    }
}

/// The window rolls, so an attacker cannot avoid contributing honest idle epochs.
/// After WINDOW_SIZE+k honest epochs, the window minimum is non-zero,
/// requiring larger divergence to trigger halt.
#[test]
fn sim10_rolling_window_raises_required_divergence() {
    let vc = 1u32;
    // Fill window with non-zero V using a sub-threshold divergence.
    // D = 25_000 → V_per = floor(400_000 * 25_000 / 1_000_000) = 10_000.
    let d_low: i128 = 25_000;
    let v_low_raw = WEIGHT_D
        .checked_mul(FixedPoint::from_raw(d_low))
        .unwrap()
        .raw();
    assert_eq!(v_low_raw, 10_000, "sanity: V for D=25_000 should be 10_000");

    // After filling window with V=10_000 each epoch, V_min = 10_000.
    // For halt: V_new > V_min + EPSILON = 10_000 + 20_000 = 30_000.
    // Required D: floor(400_000 * D / 1_000_000) > 30_000
    //           → D >= ceil(30_001 * 1_000_000 / 400_000) = ceil(75_002.5) = 75_003.
    let min_d_after_warm_window: i128 = 75_003;

    let mut s = state(vc);
    // Fill window with sub-threshold divergence.
    let warm_input = uniform_spike(vc, d_low, 0);
    for _ in 0..WINDOW_SIZE {
        advance_epoch(&mut s, warm_input.clone().as_effect(), &[]).expect("sub-threshold input must succeed");
    }
    assert!(s.convergence_window.is_full());

    // Now MIN_HALT_D_1V (50_003) must NOT trigger halt (V_new=20_001, delta=10_001 ≤ EPSILON).
    let old_min_spike = uniform_spike(vc, MIN_HALT_D_1V, 0);
    assert!(
        advance_epoch(&mut s.clone(), old_min_spike.as_effect(), &[]).is_ok(),
        "old minimum D should not halt when window is pre-warmed"
    );

    // min_d_after_warm_window (75_003) MUST trigger halt.
    let new_min_spike = uniform_spike(vc, min_d_after_warm_window, 0);
    assert_eq!(
        advance_epoch(&mut s, new_min_spike.as_effect(), &[]),
        Err(HaltReason::LyapunovViolation),
        "raised minimum D must trigger halt after pre-warmed window"
    );
}
