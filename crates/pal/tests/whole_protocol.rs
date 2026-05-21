#![cfg(feature = "std")]

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
