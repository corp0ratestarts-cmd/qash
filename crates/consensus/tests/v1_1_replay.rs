/// v1.1 Replay Corpus — 50-epoch cross-ISA determinism gate (2-K).
///
/// This file contains two tests:
///
/// 1. `v1_1_corpus_matches_pinned` — the main gate.  Runs the 50-epoch
///    sequence defined in `tests/vectors/vectors.v1.1.json` and asserts
///    every state_root matches the pinned value.  CI runs this on all
///    three authorized ISAs (x86_64, aarch64, riscv64gc); bit-identical
///    roots on all three confirm cross-ISA determinism.
///
/// 2. `gen_v1_1_corpus` — regeneration helper (#[ignore]).  Runs the same
///    sequence and prints a fresh `vectors.v1.1.json` to stdout.  Invoke
///    with:
///      cargo test -p qash-consensus --test v1_1_replay gen_v1_1_corpus \
///          -- --ignored --nocapture
///    then copy stdout into tests/vectors/vectors.v1.1.json.
///    VERIFY the new values on all three authorized ISAs before pinning.

use qash_consensus::envelope::{PROTOCOL_VERSION_V1_0, PROTOCOL_VERSION_V1_1};
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, MAX_VALIDATORS,
};

// ---------------------------------------------------------------------------
// Corpus definition
// ---------------------------------------------------------------------------

const CORPUS_EPOCHS: u64 = 50;

fn genesis() -> EpochState {
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
        causal_fingerprint: [0u8; 32],
    }
}

/// Epochs 0, 2, 4, 6, 8 use v1.0 (accepted before compat window epoch 100).
/// All others use v1.1.
fn protocol_version_for_step(step: u64) -> u32 {
    if step < 10 && step % 2 == 0 {
        PROTOCOL_VERSION_V1_0
    } else {
        PROTOCOL_VERSION_V1_1
    }
}

fn corpus_input(step: u64) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: protocol_version_for_step(step),
        update_count: 4,
    }
}

/// Run the corpus and collect (epoch, state_root) after each transition.
fn run_corpus() -> std::vec::Vec<(u64, [u8; 32])> {
    let mut state = genesis();
    let mut out = std::vec::Vec::with_capacity(CORPUS_EPOCHS as usize);

    for step in 0..CORPUS_EPOCHS {
        let input = corpus_input(step);
        let _ = advance_epoch(&mut state, &input, &[]);
        out.push((state.epoch, state.state_root));
    }

    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hex_to_bytes32(hex: &str) -> [u8; 32] {
    let bytes: std::vec::Vec<u8> = hex
        .as_bytes()
        .chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(bytes.len(), 32, "expected 32-byte root, got {}", hex);
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

fn bytes_to_hex(b: &[u8; 32]) -> std::string::String {
    b.iter().map(|byte| format!("{:02x}", byte)).collect()
}

// ---------------------------------------------------------------------------
// Main determinism gate
// ---------------------------------------------------------------------------

const PINNED_JSON: &[u8] = include_bytes!("../../../tests/vectors/vectors.v1.1.json");

fn load_pinned() -> std::vec::Vec<[u8; 32]> {
    let s = std::str::from_utf8(PINNED_JSON).expect("valid UTF-8");
    let v: serde_json::Value = serde_json::from_str(s).expect("valid JSON");
    v["state_roots"]
        .as_array()
        .expect("state_roots array")
        .iter()
        .map(|entry| hex_to_bytes32(entry["root"].as_str().expect("root string")))
        .collect()
}

#[test]
fn v1_1_corpus_matches_pinned() {
    let pinned = load_pinned();
    assert_eq!(
        pinned.len(),
        CORPUS_EPOCHS as usize,
        "vectors.v1.1.json has {} roots, expected {}; run gen_v1_1_corpus to regenerate",
        pinned.len(),
        CORPUS_EPOCHS,
    );

    let actual = run_corpus();
    for (i, ((epoch, root), expected)) in actual.iter().zip(pinned.iter()).enumerate() {
        assert_eq!(
            root, expected,
            "state_root mismatch at step {} (epoch {})\n  got:      {}\n  expected: {}",
            i,
            epoch,
            bytes_to_hex(root),
            bytes_to_hex(expected),
        );
    }
}

// ---------------------------------------------------------------------------
// Regeneration helper (#[ignore] — not run in normal CI)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn gen_v1_1_corpus() {
    let roots = run_corpus();

    println!("{{");
    println!("  \"version\": \"1.1\",");
    println!("  \"description\": \"50-epoch v1.1 replay corpus. Steps 0,2,4,6,8 use v1.0 protocol_version (exercising compatibility-window acceptance before epoch 100); steps 1,3,5,7,9 and 10-49 use v1.1. Generated on x86_64; CI verifies on aarch64 and riscv64gc.\",");
    println!("  \"corpus_epochs\": {},", CORPUS_EPOCHS);
    println!("  \"validator_count\": 4,");
    println!("  \"state_roots\": [");
    let n = roots.len();
    for (i, (epoch, root)) in roots.iter().enumerate() {
        let comma = if i + 1 < n { "," } else { "" };
        println!(
            "    {{\"step\": {}, \"epoch\": {}, \"protocol_version\": \"{}\", \"root\": \"{}\"}}{}",
            i,
            epoch,
            if protocol_version_for_step(i as u64) == PROTOCOL_VERSION_V1_0 { "v1.0" } else { "v1.1" },
            bytes_to_hex(root),
            comma,
        );
    }
    println!("  ]");
    println!("}}");
}
