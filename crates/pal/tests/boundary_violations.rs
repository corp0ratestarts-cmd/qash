//! Integration tests for Domain B → Domain A boundary isolation.
//!
//! Verifies that:
//! - Domain B observations (network frames, attestation, reset flag) cannot
//!   influence Domain A state roots.
//! - Malformed or mismatched inputs are rejected before touching Domain A state.
//! - Corrupted and truncated WAL records fail gracefully on replay.
//! - Consensus halts propagate correctly and are NOT persisted to the log.
//! - Empty logs replay to the genesis state unchanged.

#![cfg(feature = "std")]

use qash_consensus::PublicTranscript;
use qash_consensus::{
    lyapunov::{ConvergenceWindow, ValidatorMetrics},
    EpochState, HaltReason, MAX_VALIDATORS,
};
use qash_pal::hosted::{
    AttestationReport, AttestationVerdict, AttestationVerifier, CanonicalInput,
    CanonicalValidatorUpdate, CommitmentFrame, CommitmentTransport, Host, HostedError,
    InMemoryCommitmentTransport, StaticAttestationVerifier,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
        "qash-pal-boundary-{tag}-{}-{nanos}.log",
        std::process::id()
    ))
}

/// Build a CanonicalInput with a single non-idle validator update.
fn spike_input(epoch: u64, validator_count: u32, divergence_raw: i64) -> CanonicalInput {
    let mut input =
        CanonicalInput::idle(epoch, validator_count).expect("idle input construction must succeed");
    input.updates[0] = Some(CanonicalValidatorUpdate {
        divergence_raw,
        conflict_raw: divergence_raw / 4,
        slash_accum_raw: divergence_raw / 20,
    });
    input
}

// ---------------------------------------------------------------------------
// 1. Domain B isolation
// ---------------------------------------------------------------------------

/// Domain B observations (enqueued frames, sent frames, attestation bytes,
/// reset flag) must have zero effect on Domain A state roots.
#[test]
fn domain_b_observations_do_not_affect_domain_a_state_root() {
    let genesis = genesis_state(3);
    const D: i64 = 50_000;

    // Run A: Domain B noise injected between every Domain A input.
    let path_a = unique_log_path("b-isolation-noisy");
    let mut state_a = genesis;
    {
        let mut host = Host::new(&path_a).expect("host A created");
        host.enqueue_network_frame(b"pre-epoch-1 gossip noise".to_vec());
        host.set_attestation_quote([0xAB; 256]);
        host.send_network_frame(b"outbound peer announcement");
        host.request_reset();

        let i0 = spike_input(state_a.epoch, state_a.validator_count, D);
        host.apply_canonical_input(&mut state_a, &i0)
            .expect("epoch 0 applies");

        host.enqueue_network_frame(b"inter-epoch noise frame".to_vec());
        host.set_attestation_quote([0xCD; 256]);
        host.send_network_frame(b"another outbound frame");

        let i1 = spike_input(state_a.epoch, state_a.validator_count, D * 2);
        host.apply_canonical_input(&mut state_a, &i1)
            .expect("epoch 1 applies");
    }

    // Run B: identical Domain A inputs with no Domain B observations.
    let path_b = unique_log_path("b-isolation-clean");
    let mut state_b = genesis;
    {
        let mut host = Host::new(&path_b).expect("host B created");
        let i0 = spike_input(state_b.epoch, state_b.validator_count, D);
        host.apply_canonical_input(&mut state_b, &i0)
            .expect("epoch 0 applies");
        let i1 = spike_input(state_b.epoch, state_b.validator_count, D * 2);
        host.apply_canonical_input(&mut state_b, &i1)
            .expect("epoch 1 applies");
    }

    assert_eq!(
        state_a.state_root, state_b.state_root,
        "Domain B observations must not affect Domain A state root"
    );
    assert_eq!(state_a.epoch, state_b.epoch);

    let _ = std::fs::remove_file(path_a);
    let _ = std::fs::remove_file(path_b);
}

#[test]
fn commitment_transport_round_trips_public_transcript_without_raw_txs() {
    let transcript = PublicTranscript {
        state_root: [1u8; 32],
        receipt_root: [2u8; 32],
        efb_root: [3u8; 32],
        epoch: 42,
        halt_flag: false,
    };
    let validator_id = [4u8; 48];
    let attestation_quote = [5u8; 256];
    let frame = CommitmentFrame::from_transcript(&transcript, validator_id, attestation_quote);

    let mut transport = InMemoryCommitmentTransport::new();
    transport
        .send_commitment(&frame)
        .expect("commitment send succeeds");
    let received = transport
        .recv_commitment()
        .expect("commitment receive succeeds")
        .expect("one frame queued");

    assert_eq!(received, frame);
    assert_eq!(transport.recv_commitment().unwrap(), None);
}

#[test]
fn static_attestation_verifier_rejects_quote_mismatch() {
    let verifier = StaticAttestationVerifier {
        trusted_quote: [9u8; 256],
    };
    let trusted = AttestationReport {
        validator_id: [1u8; 48],
        quote: [9u8; 256],
    };
    let rejected = AttestationReport {
        validator_id: [1u8; 48],
        quote: [8u8; 256],
    };

    assert_eq!(verifier.verify(&trusted), AttestationVerdict::Trusted);
    assert_eq!(verifier.verify(&rejected), AttestationVerdict::Rejected);
}

// ---------------------------------------------------------------------------
// 2. Input validation — epoch mismatch
// ---------------------------------------------------------------------------

/// apply_canonical_input must reject an input whose epoch does not match the
/// current state epoch, and must leave the state unchanged.
#[test]
fn apply_rejects_epoch_mismatch_and_leaves_state_unchanged() {
    let path = unique_log_path("epoch-mismatch");
    let genesis = genesis_state(2);
    let mut state = genesis;
    let mut host = Host::new(&path).expect("host created");

    // Advance to epoch 1.
    let i0 = CanonicalInput::idle(0, state.validator_count).expect("idle epoch 0");
    host.apply_canonical_input(&mut state, &i0)
        .expect("epoch 0 applies");
    assert_eq!(state.epoch, 1);

    // Try to apply an input claiming epoch 5 (wrong).
    let wrong = CanonicalInput::idle(5, state.validator_count).expect("idle epoch 5");
    let result = host.apply_canonical_input(&mut state, &wrong);
    assert!(
        matches!(result, Err(HostedError::InvalidInput(_))),
        "epoch mismatch must return InvalidInput, got {result:?}"
    );
    // State must be frozen — epoch, root, halt_reason unchanged.
    assert_eq!(state.epoch, 1, "epoch must not advance after rejection");
    assert_eq!(state.halt_reason, HaltReason::None);

    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// 3. Input validation — validator count mismatch
// ---------------------------------------------------------------------------

/// apply_canonical_input must reject an input whose update count does not
/// match the state's validator_count.
#[test]
fn apply_rejects_validator_count_mismatch_and_leaves_state_unchanged() {
    let path = unique_log_path("vc-mismatch");
    let genesis = genesis_state(4);
    let mut state = genesis;
    let mut host = Host::new(&path).expect("host created");

    // Input sized for 2 validators, but state has 4.
    let wrong = CanonicalInput::idle(state.epoch, 2).expect("idle 2-validator input");
    let result = host.apply_canonical_input(&mut state, &wrong);
    assert!(
        matches!(result, Err(HostedError::InvalidInput(_))),
        "validator count mismatch must return InvalidInput"
    );
    assert_eq!(state.epoch, 0, "epoch must not advance after rejection");

    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// 4. WAL corruption — invalid log-file magic
// ---------------------------------------------------------------------------

/// Opening a file whose first 8 bytes are not the QASH log magic must fail.
#[test]
fn host_new_rejects_file_with_invalid_magic() {
    let path = unique_log_path("bad-magic");
    std::fs::write(&path, b"BADMAGIC").expect("write bad magic file");
    let result = Host::new(&path);
    assert!(
        matches!(result, Err(HostedError::InvalidLog(_))),
        "invalid log magic must return InvalidLog"
    );
    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// 5. WAL corruption — truncated record
// ---------------------------------------------------------------------------

/// If the last record in the log is truncated (e.g. mid-write crash after
/// the log header was written but before the record payload completed),
/// replay_from_genesis must return an error rather than silently skip data.
#[test]
fn replay_rejects_truncated_record() {
    let path = unique_log_path("truncated-record");
    let genesis = genesis_state(2);
    let mut state = genesis;

    {
        let mut host = Host::new(&path).expect("host created");
        let i0 = spike_input(state.epoch, state.validator_count, 10_000);
        host.apply_canonical_input(&mut state, &i0)
            .expect("epoch 0 applies");
    }

    // Lop the last 8 bytes off the file, simulating a mid-write crash.
    let mut content = std::fs::read(&path).expect("read log file");
    assert!(
        content.len() > 16,
        "log file must be longer than 16 bytes for this test to be meaningful"
    );
    let keep = content.len() - 8;
    content.truncate(keep);
    std::fs::write(&path, &content).expect("write truncated log");

    // Host::new only checks the 8-byte file header — it still succeeds.
    let host = Host::new(&path).expect("host opened on truncated log");
    let result = host.replay_from_genesis(genesis);
    assert!(
        result.is_err(),
        "replay of truncated log must return an error"
    );

    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// 6. Empty log replay
// ---------------------------------------------------------------------------

/// Replaying a freshly-created (empty) log must return a state equal to
/// the supplied genesis state.
#[test]
fn empty_log_replay_returns_genesis_state() {
    let path = unique_log_path("empty-replay");
    let genesis = genesis_state(2);

    let host = Host::new(&path).expect("host created");
    let replayed = host
        .replay_from_genesis(genesis)
        .expect("empty log replay must succeed");

    assert_eq!(replayed.epoch, genesis.epoch);
    assert_eq!(replayed.state_root, genesis.state_root);
    assert_eq!(replayed.halt_reason, genesis.halt_reason);

    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// 7. Consensus halt propagation + WAL isolation
// ---------------------------------------------------------------------------

/// A Lyapunov halt triggered inside advance_epoch must:
///   a) be returned as Err(ConsensusHalt) from apply_canonical_input.
///   b) NOT be written to the WAL (the halting transition is not committed
///      because the PAL runs advance_epoch on a scratch copy and only commits
///      on Ok — the halt-triggering input is atomically rejected).
///   c) Leave the in-memory state unchanged (epoch, root, halt_reason).
///
/// Halt trigger: fill the convergence window (3 epochs of V=0), then spike
/// divergence = SCALE (1_000_000). evaluate_projected computes:
///   V_spike = WEIGHT_D * SCALE / SCALE = 400_000
///   delta_window = |400_000 - 0| = 400_000 >> EPSILON (20_000)
/// → halt_triggered = true in pre-commit phase → Err returned, no mutation.
#[test]
fn consensus_halt_propagates_and_is_not_written_to_wal() {
    let path = unique_log_path("halt-propagation");
    let genesis = genesis_state(1);
    let mut state = genesis;
    let mut host = Host::new(&path).expect("host created");

    // Fill convergence window with 3 idle epochs (V=0 each).
    for _ in 0..3 {
        let idle = CanonicalInput::idle(state.epoch, state.validator_count).expect("idle input");
        host.apply_canonical_input(&mut state, &idle)
            .expect("idle epoch applies");
    }
    assert_eq!(state.epoch, 3);
    let pre_halt_root = state.state_root;

    // Spike: divergence = SCALE → projected V = 400_000 >> EPSILON → halt rejected.
    let spike = spike_input(state.epoch, state.validator_count, 1_000_000);
    let halt_result = host.apply_canonical_input(&mut state, &spike);

    // (a) Err(ConsensusHalt) must be returned.
    assert!(
        matches!(halt_result, Err(HostedError::ConsensusHalt(_))),
        "divergence spike must trigger ConsensusHalt, got {halt_result:?}"
    );

    // (c) In-memory state must be completely unchanged (scratch-copy atomicity).
    assert_eq!(
        state.epoch, 3,
        "epoch must be unchanged after rejected halt"
    );
    assert_eq!(
        state.state_root, pre_halt_root,
        "state root must be unchanged"
    );
    assert_eq!(
        state.halt_reason,
        HaltReason::None,
        "in-memory halt_reason must stay None — advance_epoch ran on scratch copy"
    );

    // (b) WAL must contain only the 3 committed idle epochs.
    let replayed = host
        .replay_from_genesis(genesis)
        .expect("WAL replay must succeed");
    assert_eq!(
        replayed.epoch, 3,
        "WAL must contain exactly 3 committed epochs; halting transition not persisted"
    );
    assert_eq!(
        replayed.state_root, pre_halt_root,
        "replayed root must match pre-halt root"
    );

    let _ = std::fs::remove_file(path);
}
