#![cfg(feature = "std")]

use qash_consensus::{
    advance_epoch,
    lyapunov::{ConvergenceWindow, ValidatorMetrics},
    EpochInput, EpochState, FixedPoint, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};
use qash_pal::hosted::{
    CanonicalInput, CanonicalValidatorUpdate, Host, PreparedHalt,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn genesis_state(validator_count: u32) -> EpochState {
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
    for i in 0..validator_count as usize {
        validator_ids[i][0..8].copy_from_slice(&(i as u64 + 1).to_le_bytes());
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
        causal_fingerprint: [0u8; 32],
        state_root: [0u8; 32],
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
    }
}

fn unique_log_path(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qash-pal-halt-layering-{tag}-{}-{nanos}.log",
        std::process::id()
    ))
}

fn consensus_spike_input(validator_count: u32, divergence_raw: i64) -> EpochInput {
    let mut input = EpochInput::new(validator_count);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(divergence_raw as i128),
        conflict_new: FixedPoint::from_raw((divergence_raw / 4) as i128),
        slash_accum_new: FixedPoint::from_raw((divergence_raw / 20) as i128),
    });
    input
}

fn hosted_spike_input(epoch: u64, validator_count: u32, divergence_raw: i64) -> CanonicalInput {
    let mut input =
        CanonicalInput::idle(epoch, validator_count).expect("idle input must be valid");
    input.updates[0] = Some(CanonicalValidatorUpdate {
        divergence_raw,
        conflict_raw: divergence_raw / 4,
        slash_accum_raw: divergence_raw / 20,
    });
    input
}

#[test]
fn domain_a_absorbing_halt_is_deterministic_and_replayable() {
    let mut before = genesis_state(1);
    let idle = EpochInput::new(before.validator_count);
    for _ in 0..3 {
        advance_epoch(&mut before, &idle, &[]).expect("idle epoch applies");
    }

    let mut a = before;
    let mut b = before;
    let spike = consensus_spike_input(before.validator_count, 1_000_000);

    assert_eq!(
        advance_epoch(&mut a, &spike, &[]),
        Err(HaltReason::LyapunovViolation)
    );
    assert_eq!(
        advance_epoch(&mut b, &spike, &[]),
        Err(HaltReason::LyapunovViolation)
    );
    assert_eq!(a.halt_reason, b.halt_reason);
    assert_eq!(a.epoch, b.epoch);
    assert_eq!(a.state_root, b.state_root);

    let frozen = a;
    assert_eq!(
        advance_epoch(&mut a, &idle, &[]),
        Err(HaltReason::LyapunovViolation)
    );
    assert_eq!(a.halt_reason, frozen.halt_reason);
    assert_eq!(a.epoch, frozen.epoch);
    assert_eq!(a.state_root, frozen.state_root);
}

#[test]
fn pal_absorbing_halt_preparation_zeroizes_and_marks_domain_b_actions() {
    let path = unique_log_path("ops");
    let mut host = Host::new(&path).expect("host can be created");
    let mut critical_memory = [0xA5u8; 64];

    let prepared =
        host.prepare_absorbing_halt(&mut critical_memory, HaltReason::LyapunovViolation);

    assert!(critical_memory.iter().all(|b| *b == 0));
    assert_eq!(prepared.reason, HaltReason::LyapunovViolation);
    assert!(prepared.critical_memory_zeroized);
    assert!(prepared.scheduler_disable_requested);
    assert!(prepared.watchdog_reset_requested);
    assert!(host.reset_requested());

    let _non_returning_entrypoint: fn(PreparedHalt) -> ! = PreparedHalt::enter_non_returning_loop;

    let _ = std::fs::remove_file(path);
}

#[test]
fn pal_halt_preparation_cannot_perturb_domain_a_state_roots() {
    let genesis = genesis_state(2);
    let path_noisy = unique_log_path("noisy");
    let path_clean = unique_log_path("clean");
    let mut noisy_state = genesis;
    let mut clean_state = genesis;

    {
        let mut host = Host::new(&path_noisy).expect("noisy host can be created");
        let input0 = hosted_spike_input(noisy_state.epoch, noisy_state.validator_count, 5_000);
        host.apply_canonical_input(&mut noisy_state, &input0)
            .expect("first noisy epoch applies");

        let mut critical_memory = [0x5Au8; 32];
        let _prepared =
            host.prepare_absorbing_halt(&mut critical_memory, HaltReason::HaltFlagSet);

        let input1 = hosted_spike_input(noisy_state.epoch, noisy_state.validator_count, 7_000);
        host.apply_canonical_input(&mut noisy_state, &input1)
            .expect("second noisy epoch applies");
    }

    {
        let mut host = Host::new(&path_clean).expect("clean host can be created");
        let input0 = hosted_spike_input(clean_state.epoch, clean_state.validator_count, 5_000);
        host.apply_canonical_input(&mut clean_state, &input0)
            .expect("first clean epoch applies");
        let input1 = hosted_spike_input(clean_state.epoch, clean_state.validator_count, 7_000);
        host.apply_canonical_input(&mut clean_state, &input1)
            .expect("second clean epoch applies");
    }

    assert_eq!(noisy_state.epoch, clean_state.epoch);
    assert_eq!(noisy_state.state_root, clean_state.state_root);
    assert_eq!(noisy_state.entropy_seed, clean_state.entropy_seed);
    assert_eq!(noisy_state.halt_reason, clean_state.halt_reason);

    let _ = std::fs::remove_file(path_noisy);
    let _ = std::fs::remove_file(path_clean);
}
