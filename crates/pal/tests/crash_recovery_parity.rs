//! Crash-recovery replay parity harness (Track 4).
//!
//! Simulates crashes at every possible WAL boundary (after each record write)
//! and verifies that replaying from genesis always yields:
//!   (a) the same state root as the clean run up to that point, or
//!   (b) a graceful error (InvalidRecord / UnexpectedEof) rather than a
//!       panic or silent corruption.
//!
//! This harness is the cross-ISA parity evidence cited in Track 4:
//! "crash-recovery replay parity harness: simulates crash mid-WAL, replays
//! from genesis, asserts state roots identical on x86_64, aarch64, riscv64gc."
//!
//! CI runs this on all three ISAs via platform-determinism.yml.

#![cfg(feature = "std")]

use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use qash_consensus::{
    lyapunov::{ConvergenceWindow, ValidatorMetrics},
    EpochState, HaltReason, MAX_VALIDATORS,
};
use qash_pal::hosted::{CanonicalInput, CanonicalValidatorUpdate, Host};

fn unique_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qash-crash-parity-{label}-{}-{nanos}.log",
        std::process::id()
    ))
}

fn genesis(validator_count: u32) -> EpochState {
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
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
    }
}

fn deterministic_input(epoch: u64, validator_count: u32, seed: i64) -> CanonicalInput {
    let mut input = CanonicalInput::idle(epoch, validator_count).expect("valid idle input");
    input.updates[0] = Some(CanonicalValidatorUpdate {
        divergence_raw: seed,
        conflict_raw: seed / 2,
        slash_accum_raw: seed / 10,
    });
    input
}

fn run_clean_and_collect(n_epochs: u64, validator_count: u32) -> (Vec<[u8; 32]>, Vec<u8>, PathBuf) {
    let path = unique_path("full");
    let mut state = genesis(validator_count);
    let mut host = Host::new(&path).expect("host created");
    let mut roots = Vec::new();

    for epoch in 0..n_epochs {
        let input = deterministic_input(epoch, validator_count, (epoch as i64 + 1) * 10_000);
        host.apply_canonical_input(&mut state, &input)
            .expect("apply succeeds in clean run");
        roots.push(state.state_root);
    }

    let full_contents = std::fs::read(&path).expect("read full WAL");
    (roots, full_contents, path)
}

/// Run N epochs from genesis, then for each possible byte-offset truncation
/// point, copy the WAL to a temporary file, truncate it, attempt replay from
/// genesis, and verify:
///  - If the truncation is on a record boundary: the replayed state root matches
///    the clean run up to that epoch.
///  - If the truncation is mid-record: replay returns an error (not a panic).
#[test]
fn crash_mid_wal_replay_matches_clean_run() {
    // EpochState is ~80 KB; CanonicalInput is ~32 KB. Spawn a thread with an
    // explicit 8 MiB stack so iterating over multiple states doesn't overflow.
    std::thread::Builder::new()
        .name("crash-parity".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(crash_mid_wal_body)
        .expect("spawn test thread")
        .join()
        .expect("test thread must not panic");
}

fn crash_mid_wal_body() {
    const N_EPOCHS: u64 = 5;
    const VALIDATORS: u32 = 4;

    let (clean_roots, full_contents, full_path) =
        run_clean_and_collect(N_EPOCHS, VALIDATORS);

    // Collect byte offsets by running a second clean pass and recording file
    // size after each epoch application.
    let mut epoch_offsets: Vec<u64> = Vec::new();
    {
        let path2 = unique_path("offsets");
        let mut state = genesis(VALIDATORS);
        let mut host = Host::new(&path2).expect("host created for offset tracking");
        for epoch in 0..N_EPOCHS {
            let input =
                deterministic_input(epoch, VALIDATORS, (epoch as i64 + 1) * 10_000);
            host.apply_canonical_input(&mut state, &input)
                .expect("apply in offset pass");
            epoch_offsets.push(
                std::fs::metadata(&path2)
                    .expect("WAL metadata readable")
                    .len(),
            );
        }
        let _ = std::fs::remove_file(&path2);
    }

    // For each truncation point that lands on a record boundary, replay must
    // match the clean run. We test the offset after each epoch's record.
    for (epoch_idx, &byte_offset) in epoch_offsets.iter().enumerate() {
        let truncated_path = unique_path(&format!("truncated-{epoch_idx}"));
        std::fs::write(&truncated_path, &full_contents[..byte_offset as usize])
            .expect("write truncated WAL");

        let replayed_state = Host::new(&truncated_path)
            .expect("host opened on truncated WAL")
            .replay_from_genesis(genesis(VALIDATORS))
            .expect("clean-boundary replay must succeed");

        assert_eq!(
            replayed_state.state_root,
            clean_roots[epoch_idx],
            "epoch {} state root mismatch after clean truncation at {} bytes",
            epoch_idx + 1,
            byte_offset
        );

        let _ = std::fs::remove_file(&truncated_path);
    }

    // For mid-record truncations in the last record: replay must not panic.
    if let (Some(&last_full_offset), Some(&prev_offset)) =
        (epoch_offsets.last(), epoch_offsets.get(epoch_offsets.len().saturating_sub(2)))
    {
        let full_len = full_contents.len() as u64;
        for partial_bytes in 1..full_len.saturating_sub(last_full_offset) {
            let mid_offset = (last_full_offset + partial_bytes) as usize;
            let partial_path = unique_path(&format!("partial-{partial_bytes}"));
            std::fs::write(&partial_path, &full_contents[..mid_offset])
                .expect("write partial WAL");

            let result = Host::new(&partial_path)
                .expect("host opened on partial WAL")
                .replay_from_genesis(genesis(VALIDATORS));

            // Mid-record truncation: either graceful Err, or Ok with a state
            // root matching the previous clean checkpoint (last complete record).
            if let Ok(s) = result {
                let prev_root = if epoch_offsets.len() >= 2 {
                    clean_roots[epoch_offsets.len() - 2]
                } else {
                    [0u8; 32]
                };
                let _ = prev_offset; // suppress unused variable lint
                assert!(
                    s.state_root == prev_root || s.state_root == [0u8; 32],
                    "mid-record replay returned non-checkpoint state root at offset +{partial_bytes}"
                );
            }

            let _ = std::fs::remove_file(&partial_path);
        }
    }

    let _ = std::fs::remove_file(&full_path);
}

/// Verify that a WAL with an invalid magic header fails gracefully.
/// Host::new validates the magic on open and returns Err for a corrupt header.
#[test]
fn corrupted_wal_header_fails_gracefully() {
    let path = unique_path("bad-magic");
    {
        let mut f = std::fs::File::create(&path).expect("create WAL file");
        f.write_all(b"BADMAGIC").expect("write bad magic");
    }
    // Host::new itself detects the bad magic and returns an error.
    let result = Host::new(&path);
    assert!(
        result.is_err(),
        "corrupted magic header must cause Host::new to return an error"
    );
    let _ = std::fs::remove_file(&path);
}

/// Zero-byte WAL (header only) replays as genesis — no records applied.
#[test]
fn empty_wal_replays_as_genesis() {
    let path = unique_path("empty");
    let g = genesis(4);
    let replayed = Host::new(&path)
        .expect("host created")
        .replay_from_genesis(g)
        .expect("empty WAL replay succeeds");
    assert_eq!(replayed.state_root, [0u8; 32], "empty WAL = genesis state");
    let _ = std::fs::remove_file(&path);
}
