#![cfg(feature = "std")]

use qash_consensus::{
    lyapunov::{ConvergenceWindow, ValidatorMetrics},
    EpochState, HaltReason, MAX_VALIDATORS,
};
use qash_pal::hosted::{CanonicalInput, CanonicalValidatorUpdate, Host};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn genesis_state(validator_count: u32) -> EpochState {
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
    for i in 0..validator_count as usize {
        validator_ids[i][0..8].copy_from_slice(&(i as u64).to_le_bytes());
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
    }
}

fn unique_log_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qash-pal-{name}-{}-{nanos}.log",
        std::process::id()
    ))
}

fn updated_input(epoch: u64, validator_count: u32, raw: i64) -> CanonicalInput {
    let mut input = CanonicalInput::idle(epoch, validator_count).expect("valid idle input");
    input.updates[0] = Some(CanonicalValidatorUpdate {
        divergence_raw: raw,
        conflict_raw: raw / 2,
        slash_accum_raw: raw / 10,
    });
    input
}

#[test]
fn replaying_persisted_input_log_from_genesis_matches_crashed_state_root() {
    let path = unique_log_path("crash-restart");
    let genesis = genesis_state(4);
    let mut pre_crash_state = genesis;

    {
        let mut host = Host::new(&path).expect("host log can be created");
        let input0 = updated_input(
            pre_crash_state.epoch,
            pre_crash_state.validator_count,
            10_000,
        );
        host.apply_canonical_input(&mut pre_crash_state, &input0)
            .expect("first canonical input applies");

        host.enqueue_network_frame(b"domain-b-only noise".to_vec());
        host.send_network_frame(b"outbound domain-b-only bytes");
        host.set_attestation_quote([0x5a; 256]);
        host.request_reset();

        let input1 = updated_input(
            pre_crash_state.epoch,
            pre_crash_state.validator_count,
            20_000,
        );
        host.apply_canonical_input(&mut pre_crash_state, &input1)
            .expect("second canonical input applies");
    }

    let restarted = Host::new(&path).expect("host log can be reopened");
    let replayed_state = restarted
        .replay_from_genesis(genesis)
        .expect("persisted canonical log replays from genesis");

    assert_eq!(replayed_state.epoch, pre_crash_state.epoch);
    assert_eq!(replayed_state.state_root, pre_crash_state.state_root);
    assert_eq!(replayed_state.entropy_seed, pre_crash_state.entropy_seed);

    let _ = std::fs::remove_file(path);
}

#[test]
fn replaying_same_persisted_log_twice_from_genesis_is_identical() {
    let path = unique_log_path("deterministic-replay");
    let genesis = genesis_state(3);
    let mut state = genesis;

    let mut host = Host::new(&path).expect("host log can be created");
    for raw in [3_000, 4_000, 5_000] {
        let input = updated_input(state.epoch, state.validator_count, raw);
        host.apply_canonical_input(&mut state, &input)
            .expect("canonical input applies");
    }

    let replayed_once = host
        .replay_from_genesis(genesis)
        .expect("first replay succeeds");
    let replayed_twice = host
        .replay_from_genesis(genesis)
        .expect("second replay succeeds");

    assert_eq!(replayed_once.state_root, state.state_root);
    assert_eq!(replayed_twice.state_root, state.state_root);
    assert_eq!(replayed_once.state_root, replayed_twice.state_root);

    let _ = std::fs::remove_file(path);
}
