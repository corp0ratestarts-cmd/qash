#![cfg(feature = "std")]

// ── TH-7 cross-ISA anchors ────────────────────────────────────────────────────
// These values are the canonical outputs of a 5-epoch deterministic sharded
// replay on x86_64. The cross-ISA CI matrix (platform-determinism.yml) verifies
// aarch64 and riscv64gc produce identical values.
//
// To re-derive after a legitimate protocol change:
//   cargo test -p qash-pal --features std --test whole_protocol \
//     -- whole_protocol_sharded_canonical_roots_print --nocapture
// Verify all three ISAs agree before updating.
const EXPECTED_SHARDED_STATE_ROOT_5_EPOCHS: [u8; 32] = [
    224, 47, 3, 182, 189, 223, 252, 149, 128, 54, 33, 251, 163, 249, 27, 70,
    217, 248, 15, 95, 193, 72, 175, 247, 170, 95, 219, 201, 251, 9, 177, 154,
];
const EXPECTED_SHARDED_EFB_ROOT_5_EPOCHS: [u8; 32] = [
    70, 143, 66, 216, 86, 168, 204, 73, 65, 202, 238, 145, 186, 167, 234, 72,
    248, 190, 39, 233, 6, 198, 187, 53, 47, 235, 168, 197, 236, 178, 127, 255,
];
const EXPECTED_SHARDED_RECEIPT_ROOT_5_EPOCHS: [u8; 32] = [
    16, 206, 249, 147, 35, 160, 250, 185, 1, 149, 120, 18, 59, 155, 254, 238,
    6, 23, 244, 203, 129, 152, 223, 252, 197, 32, 137, 13, 114, 18, 158, 48,
];

use qash_consensus::lyapunov::ConvergenceWindow;
use qash_consensus::{
    h_domain, DomainTag, EpochState, HaltReason, PublicTranscript, ValidatorMetrics, MAX_VALIDATORS,
};
use qash_pal::hosted::{
    CanonicalInput, CanonicalShardCommitment, CanonicalShardingInput, CanonicalZkProfile, Host,
    HostedError, StaticZkProofVerifier, ZkProofBundle, ZkProofVerifier,
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
        state_root: [0u8; 32],
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
        causal_fingerprint: [0u8; 32],
    }
}

fn unique_log_path(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qash-pal-whole-protocol-{tag}-{}-{nanos}.log",
        std::process::id()
    ))
}

fn commitment_root(epoch: u64, shard_id: u32, kind: u8) -> [u8; 32] {
    let mut buf = [0u8; 13];
    buf[0..8].copy_from_slice(&epoch.to_be_bytes());
    buf[8..12].copy_from_slice(&shard_id.to_be_bytes());
    buf[12] = kind;
    h_domain(DomainTag::EpochFinalityBeacon, &buf)
}

fn mock_abcr_frame(epoch: u64) -> Vec<u8> {
    let mut frame = b"QASH-ABCR-MOCK\0".to_vec();
    frame.extend_from_slice(&epoch.to_le_bytes());
    frame.extend_from_slice(&commitment_root(epoch, 0, 9));
    frame
}

fn sharded_input(epoch: u64, validator_count: u32) -> CanonicalInput {
    let mut input = CanonicalInput::idle(epoch, validator_count).expect("valid idle input");
    input.sharding = Some(CanonicalShardingInput {
        shard_commitments: vec![
            CanonicalShardCommitment {
                shard_id: 0,
                state_root: commitment_root(epoch, 0, 0),
                receipt_root: commitment_root(epoch, 0, 1),
            },
            CanonicalShardCommitment {
                shard_id: 1,
                state_root: commitment_root(epoch, 1, 0),
                receipt_root: commitment_root(epoch, 1, 1),
            },
        ],
        zk_batch_root: if epoch % 2 == 0 {
            [0u8; 32]
        } else {
            commitment_root(epoch, 0, 2)
        },
        zk_profile: if epoch % 2 == 0 {
            None
        } else {
            Some(CanonicalZkProfile::pr93_plonky3_fri_poseidon_qash())
        },
    });
    input
}

#[test]
fn hosted_zk_bundle_accepts_only_pr93_profile_shape() {
    let profile = CanonicalZkProfile::pr93_plonky3_fri_poseidon_qash();
    let batch_root = commitment_root(9, 0, 2);
    let verifier = StaticZkProofVerifier {
        accepted_profile: profile,
        accepted_batch_root: batch_root,
    };
    let bundle = ZkProofBundle {
        profile,
        shard_proof_count: 16,
        aggregation_proof_count: 1,
        batch_root,
    };
    assert!(matches!(
        verifier.verify_bundle(&bundle),
        Ok(root) if root == batch_root
    ));

    let bad_bundle = ZkProofBundle {
        profile: CanonicalZkProfile {
            recursion_depth: 3,
            ..profile
        },
        ..bundle
    };
    assert!(matches!(
        verifier.verify_bundle(&bad_bundle),
        Err(HostedError::InvalidInput(_))
    ));
}

fn transcript_from_state(state: &EpochState) -> PublicTranscript {
    PublicTranscript {
        state_root: state.state_root,
        receipt_root: state.receipt_root,
        efb_root: state.efb_root,
        epoch: state.epoch,
        halt_flag: state.halt_reason != HaltReason::None,
    }
}

#[test]
fn hosted_whole_protocol_sharded_replay_is_deterministic() {
    let path = unique_log_path("happy-path");
    let genesis = genesis_state(4);
    let mut state = genesis;
    let mut host = Host::new(&path).expect("host created");
    let mut transcripts = Vec::new();

    for _ in 0..5 {
        host.enqueue_network_frame(mock_abcr_frame(state.epoch));
        host.enqueue_network_frame(b"domain-b-noise-frame".to_vec());
        host.send_network_frame(b"outbound-domain-b-observation");

        let mut saw_mock_abcr = false;
        while let Some(frame) = host.recv_network_frame() {
            if frame.starts_with(b"QASH-ABCR-MOCK\0") {
                saw_mock_abcr = true;
                break;
            }
        }
        assert!(
            saw_mock_abcr,
            "mock ABCR frame must be normalized before admission"
        );
        let input = sharded_input(state.epoch, state.validator_count);
        let result = host
            .apply_canonical_input(&mut state, &input)
            .expect("sharded canonical input applies");

        assert_eq!(result.public_transcript, transcript_from_state(&state));
        assert_ne!(state.receipt_root, [0u8; 32]);
        assert_ne!(state.efb_root, [0u8; 32]);
        transcripts.push(result.public_transcript);
    }

    assert_eq!(state.epoch, 5);
    assert_eq!(host.sent_frames().len(), 5);

    let replayed = host
        .replay_from_genesis(genesis)
        .expect("persisted sharded log replays");
    assert_eq!(replayed.state_root, state.state_root);
    assert_eq!(replayed.receipt_root, state.receipt_root);
    assert_eq!(replayed.efb_root, state.efb_root);
    assert_eq!(
        transcripts.last().copied(),
        Some(transcript_from_state(&replayed))
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn hosted_whole_protocol_rejects_malformed_sharded_input_without_persistence() {
    let path = unique_log_path("malformed-sharding");
    let genesis = genesis_state(4);
    let mut state = genesis;
    let mut host = Host::new(&path).expect("host created");

    let mut bad = sharded_input(state.epoch, state.validator_count);
    bad.sharding
        .as_mut()
        .expect("sharding present")
        .shard_commitments
        .swap(0, 1);

    let result = host.apply_canonical_input(&mut state, &bad);
    assert!(matches!(
        result,
        Err(HostedError::ConsensusHalt(HaltReason::DecodeInvalid))
    ));
    assert_eq!(state.epoch, genesis.epoch);
    assert_eq!(state.state_root, genesis.state_root);
    assert_eq!(state.receipt_root, genesis.receipt_root);
    assert_eq!(state.efb_root, genesis.efb_root);
    assert_eq!(state.halt_reason, genesis.halt_reason);

    let replayed = host
        .replay_from_genesis(genesis)
        .expect("empty log still replays");
    assert_eq!(replayed.epoch, genesis.epoch);
    assert_eq!(replayed.state_root, genesis.state_root);
    assert_eq!(replayed.receipt_root, genesis.receipt_root);
    assert_eq!(replayed.efb_root, genesis.efb_root);
    assert_eq!(replayed.halt_reason, genesis.halt_reason);

    let _ = std::fs::remove_file(path);
}

// ── Cross-ISA canonical root tests (TH-7) ────────────────────────────────────

fn run_canonical_sharded_5epoch() -> (EpochState, std::path::PathBuf) {
    let path = unique_log_path("canonical-isa");
    let genesis = genesis_state(4);
    let mut state = genesis;
    let mut host = Host::new(&path).expect("host created");
    for _ in 0..5 {
        host.enqueue_network_frame(mock_abcr_frame(state.epoch));
        while let Some(frame) = host.recv_network_frame() {
            if frame.starts_with(b"QASH-ABCR-MOCK\0") {
                break;
            }
        }
        let input = sharded_input(state.epoch, state.validator_count);
        host.apply_canonical_input(&mut state, &input)
            .expect("canonical sharded input applies");
    }
    (state, path)
}

/// Print canonical sharded roots (used to bootstrap pinned constants above).
/// Run with --nocapture to capture values for a new ISA or after a protocol change.
#[test]
fn whole_protocol_sharded_canonical_roots_print() {
    let (state, path) = run_canonical_sharded_5epoch();
    println!(
        "CANONICAL_SHARDED_STATE_ROOT_5_EPOCHS   = {:?}",
        state.state_root
    );
    println!(
        "CANONICAL_SHARDED_EFB_ROOT_5_EPOCHS     = {:?}",
        state.efb_root
    );
    println!(
        "CANONICAL_SHARDED_RECEIPT_ROOT_5_EPOCHS = {:?}",
        state.receipt_root
    );
    let _ = std::fs::remove_file(path);
}

/// TH-7 empirical anchor: 5-epoch hosted whole-protocol sharded replay roots
/// MUST be identical across x86_64, aarch64, and riscv64gc.
///
/// Update EXPECTED_SHARDED_* constants only after verifying all three ISA
/// targets produce the new value in the cross-ISA CI matrix.
#[test]
fn whole_protocol_sharded_canonical_roots_golden() {
    let (state, path) = run_canonical_sharded_5epoch();
    assert_eq!(
        state.state_root,
        EXPECTED_SHARDED_STATE_ROOT_5_EPOCHS,
        "hosted sharded state_root changed — update EXPECTED_SHARDED_STATE_ROOT_5_EPOCHS \
         only after verifying all three ISA targets produce the new value"
    );
    assert_eq!(
        state.efb_root,
        EXPECTED_SHARDED_EFB_ROOT_5_EPOCHS,
        "hosted sharded efb_root changed — update EXPECTED_SHARDED_EFB_ROOT_5_EPOCHS \
         only after verifying all three ISA targets produce the new value"
    );
    assert_eq!(
        state.receipt_root,
        EXPECTED_SHARDED_RECEIPT_ROOT_5_EPOCHS,
        "hosted sharded receipt_root changed — update EXPECTED_SHARDED_RECEIPT_ROOT_5_EPOCHS \
         only after verifying all three ISA targets produce the new value"
    );
    let _ = std::fs::remove_file(path);
}
