/// Binary replay corpus — Phase 1 of Issue #22 hardening.
///
/// Three canonical state snapshots (epoch 0, 1, 3) encoded via
/// `encode_full_state_into`. The expected state_root values are pinned.
/// Any bit-level change to the encoding or hash logic fails this test.
///
/// Cross-references:
///   proofs/model/vectors.json TV-0/TV-1/TV-2 — same state roots
///   EXPECTED_STATE_ROOT_3_EPOCHS in golden_replay.rs — same epoch-3 root
///
/// DO NOT regenerate these vectors automatically. If they change, verify
/// the new values on all three authorized ISAs (x86_64, aarch64, riscv64gc)
/// before updating. Use the `gen_replay_snapshots` test in gen_vectors.rs
/// with --ignored --nocapture to regenerate.

use qash_consensus::transition::{
    decode_full_state, encode_full_state_into, advance_epoch,
    EpochInput, EpochState, HaltReason, MAX_VALIDATORS, FULL_STATE_MAX_BYTES,
};
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};

// ---------------------------------------------------------------------------
// Canonical snapshot hex strings (pinned — do not auto-regenerate)
// ---------------------------------------------------------------------------
//
// Format: encode_full_state_into output for (genesis, 4 validators) after N epochs.
// Length: 460 bytes each (4 validators × 80 + 112 fixed + 28 window).

const SNAPSHOT_EPOCH_0_HEX: &str = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

const SNAPSHOT_EPOCH_1_HEX: &str = "0100000000000000513e26e067c3857289f0557adc7db9ea43cda6595cda4510cbf11af06196d352000000000000000000000000000000000000000000000000000000000000000011d5dbd81e70d337c2e89eca4b926b9eaf6258efeef0b03b0d37f85cb7d886660000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000";

const SNAPSHOT_EPOCH_3_HEX: &str = "03000000000000008adba4d30a361e2797dfef2abf8d0db579e04ff1044a312c8ae05dc567687ac600000000000000000000000000000000000000000000000000000000000000008163f14ed023e105563316ac3e52de63f80d3346adc78d9ea024774c45b8178f0000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000";

// Expected state roots (pinned; cross-reference proofs/model/vectors.json TV-0/TV-1/TV-2)
const EXPECTED_ROOT_EPOCH_0: [u8; 32] = [0u8; 32];

const EXPECTED_ROOT_EPOCH_1: [u8; 32] = [
    0x51, 0x3e, 0x26, 0xe0, 0x67, 0xc3, 0x85, 0x72,
    0x89, 0xf0, 0x55, 0x7a, 0xdc, 0x7d, 0xb9, 0xea,
    0x43, 0xcd, 0xa6, 0x59, 0x5c, 0xda, 0x45, 0x10,
    0xcb, 0xf1, 0x1a, 0xf0, 0x61, 0x96, 0xd3, 0x52,
];

const EXPECTED_ROOT_EPOCH_3: [u8; 32] = [
    0x8a, 0xdb, 0xa4, 0xd3, 0x0a, 0x36, 0x1e, 0x27,
    0x97, 0xdf, 0xef, 0x2a, 0xbf, 0x8d, 0x0d, 0xb5,
    0x79, 0xe0, 0x4f, 0xf1, 0x04, 0x4a, 0x31, 0x2c,
    0x8a, 0xe0, 0x5d, 0xc5, 0x67, 0x68, 0x7a, 0xc6,
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hex_to_bytes(hex: &str) -> std::vec::Vec<u8> {
    hex.as_bytes()
        .chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}

fn genesis_4() -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: 4,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        state_root: [0u8; 32],
    }
}

fn idle(vc: u32) -> EpochInput {
    EpochInput { updates: [None; MAX_VALIDATORS], update_count: vc }
}

// ---------------------------------------------------------------------------
// Corpus tests
// ---------------------------------------------------------------------------

/// Decode the pinned epoch-0 snapshot; verify state_root matches expected.
/// epoch-0 is genesis (no advance_epoch called), so state_root = [0;32].
#[test]
fn replay_corpus_epoch_0() {
    let bytes = hex_to_bytes(SNAPSHOT_EPOCH_0_HEX);
    let state = decode_full_state(&bytes).expect("epoch-0 snapshot must decode");
    assert_eq!(state.epoch, 0, "epoch-0 snapshot must have epoch=0");
    assert_eq!(
        state.state_root, EXPECTED_ROOT_EPOCH_0,
        "epoch-0 state_root must match pinned value"
    );
    assert_eq!(state.validator_count, 4);
    assert_eq!(state.halt_reason, HaltReason::None);
}

/// Decode the pinned epoch-1 snapshot; verify state_root matches expected.
#[test]
fn replay_corpus_epoch_1() {
    let bytes = hex_to_bytes(SNAPSHOT_EPOCH_1_HEX);
    let state = decode_full_state(&bytes).expect("epoch-1 snapshot must decode");
    assert_eq!(state.epoch, 1, "epoch-1 snapshot must have epoch=1");
    assert_eq!(
        state.state_root, EXPECTED_ROOT_EPOCH_1,
        "epoch-1 state_root must match pinned value"
    );
    assert_eq!(state.halt_reason, HaltReason::None);
}

/// Decode the pinned epoch-3 snapshot; verify state_root matches expected.
/// This is the same root as EXPECTED_STATE_ROOT_3_EPOCHS in golden_replay.rs
/// and TV-2 in proofs/model/vectors.json — three independent anchors.
#[test]
fn replay_corpus_epoch_3() {
    let bytes = hex_to_bytes(SNAPSHOT_EPOCH_3_HEX);
    let state = decode_full_state(&bytes).expect("epoch-3 snapshot must decode");
    assert_eq!(state.epoch, 3, "epoch-3 snapshot must have epoch=3");
    assert_eq!(
        state.state_root, EXPECTED_ROOT_EPOCH_3,
        "epoch-3 state_root must match pinned value"
    );
    assert_eq!(state.halt_reason, HaltReason::None);
    // Window must be full after 3 epochs.
    assert!(state.convergence_window.is_full(), "window must be full at epoch 3");
}

/// Replay forward from genesis and verify the state at each epoch produces
/// the same binary encoding as the pinned snapshot.
/// This is the key replay-determinism test: same input sequence → same bytes.
#[test]
fn replay_binary_corpus() {
    // epoch 0 — before any advance
    let state0 = genesis_4();
    let mut buf0 = [0u8; FULL_STATE_MAX_BYTES];
    let len0 = encode_full_state_into(&mut state0.clone(), &mut buf0);
    let pinned0 = hex_to_bytes(SNAPSHOT_EPOCH_0_HEX);
    assert_eq!(
        &buf0[..len0], pinned0.as_slice(),
        "epoch-0 encoding must be byte-identical to pinned snapshot"
    );

    // epoch 1 — after one idle epoch
    let mut state1 = genesis_4();
    advance_epoch(&mut state1, &idle(4), &[]).unwrap();
    let mut buf1 = [0u8; FULL_STATE_MAX_BYTES];
    let len1 = encode_full_state_into(&mut state1, &mut buf1);
    let pinned1 = hex_to_bytes(SNAPSHOT_EPOCH_1_HEX);
    assert_eq!(
        &buf1[..len1], pinned1.as_slice(),
        "epoch-1 encoding must be byte-identical to pinned snapshot"
    );

    // epoch 3 — after three idle epochs
    let mut state3 = genesis_4();
    for _ in 0..3 { advance_epoch(&mut state3, &idle(4), &[]).unwrap(); }
    let mut buf3 = [0u8; FULL_STATE_MAX_BYTES];
    let len3 = encode_full_state_into(&mut state3, &mut buf3);
    let pinned3 = hex_to_bytes(SNAPSHOT_EPOCH_3_HEX);
    assert_eq!(
        &buf3[..len3], pinned3.as_slice(),
        "epoch-3 encoding must be byte-identical to pinned snapshot"
    );
}
