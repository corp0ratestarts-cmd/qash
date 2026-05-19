#![cfg(feature = "std")]

use qash_consensus::{lyapunov::{ConvergenceWindow, ValidatorMetrics}, EpochState, HaltReason, MAX_VALIDATORS};
use qash_pal::hosted::{CanonicalInput, CanonicalValidatorUpdate, Host};

fn genesis_state(validator_count: u32) -> EpochState {
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
    for i in 0..validator_count as usize {
        validator_ids[i][0..8].copy_from_slice(&(i as u64 + 1).to_le_bytes());
    }
    EpochState { epoch: 0, halt_reason: HaltReason::None, entropy_seed: [0u8; 32], validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS], validator_count, convergence_window: ConvergenceWindow::new(), nonces: [0u64; MAX_VALIDATORS], validator_ids, cascade_health: 0, state_root: [0u8; 32] }
}

fn spike_input(epoch: u64, validator_count: u32, divergence_raw: i64) -> CanonicalInput {
    let mut input = CanonicalInput::idle(epoch, validator_count).expect("idle input");
    input.updates[0] = Some(CanonicalValidatorUpdate { divergence_raw, conflict_raw: divergence_raw / 4, slash_accum_raw: divergence_raw / 20 });
    input
}

#[test]
fn pqc_feature_toggle_produces_identical_domain_a_outputs() {
    let mut s = genesis_state(3);
    let tmp = std::env::temp_dir().join(format!("qash-pqc-parity-{}.log", std::process::id()));
    let mut host = Host::new(&tmp).expect("host");

    let i0 = spike_input(s.epoch, s.validator_count, 25_000);
    host.apply_canonical_input(&mut s, &i0).expect("apply i0");
    let i1 = spike_input(s.epoch, s.validator_count, 75_000);
    host.apply_canonical_input(&mut s, &i1).expect("apply i1");

    println!("FINAL_ROOT={}", s.state_root.iter().map(|b| format!("{b:02x}")).collect::<String>());
    let _ = std::fs::remove_file(tmp);
}
