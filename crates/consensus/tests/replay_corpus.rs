use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
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
    advance_epoch, decode_full_state, encode_full_state_into, EpochInput, EpochState, HaltReason,
    FULL_STATE_MAX_BYTES, MAX_VALIDATORS,
};

// ---------------------------------------------------------------------------
// Canonical snapshot hex strings (pinned — do not auto-regenerate)
// ---------------------------------------------------------------------------
//
// Format: encode_full_state_into output for (genesis, 4 validators) after N epochs.
// Length: 468 bytes each (4 validators × 80 + 120 fixed + 28 window).
// Wire format v1.1: FULL_STATE_FIXED_BYTES=120 (added cascade_health:u32 + 4-byte pad).

const SNAPSHOT_EPOCH_0_HEX: &str = "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

const SNAPSHOT_EPOCH_1_HEX: &str = "01000000000000005206dd7854901fa17203379be2e6a25329dcba9356ba6fd72dcab76a201c71a7000000000000000000000000000000000000000000000000000000000000000011d5dbd81e70d337c2e89eca4b926b9eaf6258efeef0b03b0d37f85cb7d8866600000000040000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000";

const SNAPSHOT_EPOCH_3_HEX: &str = "03000000000000005648ddd6dfe9ac7a8f151f8f21b7cf0c6e61030546643b769504ff79fd9c722d00000000000000000000000000000000000000000000000000000000000000008163f14ed023e105563316ac3e52de63f80d3346adc78d9ea024774c45b8178f00000000040000000300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000";

// Expected state roots (pinned; cross-reference proofs/model/vectors.json TV-0/TV-1/TV-2)
const EXPECTED_ROOT_EPOCH_0: [u8; 32] = [0u8; 32];

const EXPECTED_ROOT_EPOCH_1: [u8; 32] = [
    0x52, 0x06, 0xdd, 0x78, 0x54, 0x90, 0x1f, 0xa1, 0x72, 0x03, 0x37, 0x9b, 0xe2, 0xe6, 0xa2, 0x53,
    0x29, 0xdc, 0xba, 0x93, 0x56, 0xba, 0x6f, 0xd7, 0x2d, 0xca, 0xb7, 0x6a, 0x20, 0x1c, 0x71, 0xa7,
];

const EXPECTED_ROOT_EPOCH_3: [u8; 32] = [
    0x56, 0x48, 0xdd, 0xd6, 0xdf, 0xe9, 0xac, 0x7a, 0x8f, 0x15, 0x1f, 0x8f, 0x21, 0xb7, 0xcf, 0x0c,
    0x6e, 0x61, 0x03, 0x05, 0x46, 0x64, 0x3b, 0x76, 0x95, 0x04, 0xff, 0x79, 0xfd, 0x9c, 0x72, 0x2d,
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
    assert!(
        state.convergence_window.is_full(),
        "window must be full at epoch 3"
    );
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
        &buf0[..len0],
        pinned0.as_slice(),
        "epoch-0 encoding must be byte-identical to pinned snapshot"
    );

    // epoch 1 — after one idle epoch
    let mut state1 = genesis_4();
    advance_epoch(&mut state1, &idle(4), &[]).unwrap();
    let mut buf1 = [0u8; FULL_STATE_MAX_BYTES];
    let len1 = encode_full_state_into(&mut state1, &mut buf1);
    let pinned1 = hex_to_bytes(SNAPSHOT_EPOCH_1_HEX);
    assert_eq!(
        &buf1[..len1],
        pinned1.as_slice(),
        "epoch-1 encoding must be byte-identical to pinned snapshot"
    );

    // epoch 3 — after three idle epochs
    let mut state3 = genesis_4();
    for _ in 0..3 {
        advance_epoch(&mut state3, &idle(4), &[]).unwrap();
    }
    let mut buf3 = [0u8; FULL_STATE_MAX_BYTES];
    let len3 = encode_full_state_into(&mut state3, &mut buf3);
    let pinned3 = hex_to_bytes(SNAPSHOT_EPOCH_3_HEX);
    assert_eq!(
        &buf3[..len3],
        pinned3.as_slice(),
        "epoch-3 encoding must be byte-identical to pinned snapshot"
    );
}
