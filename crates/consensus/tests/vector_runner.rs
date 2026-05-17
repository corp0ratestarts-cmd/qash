// vector_runner.rs — Golden-vector runner for fixed-point, leaf-index, and epoch replay.
//
// Reads tests/vectors/vectors.v1.json (embedded at compile time). Verifies that the Rust
// implementation produces bit-for-bit identical outputs to the stored expected values.
// Cross-ISA CI runs this on x86_64, aarch64, and riscv64gc; divergence means a
// Domain A determinism violation.
//
// To regenerate expected values after a deliberate code change:
//   cargo test -p qash-consensus --test vector_runner regen -- --ignored --nocapture
// Then inspect the output, verify on ALL three authorized ISAs, and update vectors.v1.json.
//
// Coverage: proofs/COVERAGE.md — "Cross-ISA replay invariance" (CI-VERIFIED)

use qash_consensus::derive::derive_leaf_index;
use qash_consensus::fixed_point::{FixedPoint, SCALE};
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, MAX_VALIDATORS,
};

const VECTORS_V1_JSON: &str = include_str!("../../../tests/vectors/vectors.v1.json");

// ---------------------------------------------------------------------------
// Hex helpers
// ---------------------------------------------------------------------------

fn hex_decode(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd-length hex string: '{}'", s);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("invalid hex"))
        .collect()
}

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

// ---------------------------------------------------------------------------
// Genesis helpers (mirrors replay_corpus.rs / golden_replay.rs conventions)
// ---------------------------------------------------------------------------

fn genesis_state(validator_count: u32) -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        state_root: [0u8; 32],
    }
}

fn idle_input(n: u32) -> EpochInput {
    EpochInput { updates: [None; MAX_VALIDATORS], update_count: n }
}

// ---------------------------------------------------------------------------
// Vector dispatcher
// ---------------------------------------------------------------------------

#[test]
fn vector_runner_all() {
    let root: serde_json::Value =
        serde_json::from_str(VECTORS_V1_JSON).expect("vectors.v1.json must be valid JSON");

    let vectors = root["vectors"].as_array().expect("root.vectors must be an array");
    assert!(!vectors.is_empty(), "no vectors found");

    let mut ran = 0usize;

    for v in vectors {
        let name = v["name"].as_str().unwrap_or("<unnamed>");

        if name.starts_with("fixed_point") {
            run_fixed_point(name, v);
            ran += 1;
        } else if name.starts_with("leaf_index") {
            run_leaf_index(name, v);
            ran += 1;
        } else if name.starts_with("genesis_noop_epochs") {
            run_genesis_noop_epochs(name, v);
            ran += 1;
        } else {
            // Unknown vector kind — fail loudly so stale entries are caught.
            panic!("vector_runner: unknown vector kind '{}' — add a dispatcher case", name);
        }
    }

    assert!(ran >= 3, "expected at least 3 vectors, ran {}", ran);
}

// ---------------------------------------------------------------------------
// fixed_point: verify checked_mul matches expected_raw
// ---------------------------------------------------------------------------

fn run_fixed_point(name: &str, v: &serde_json::Value) {
    let left_raw = v["inputs"]["left_raw"]
        .as_i64()
        .expect("left_raw must be i64") as i128;
    let right_raw = v["inputs"]["right_raw"]
        .as_i64()
        .expect("right_raw must be i64") as i128;
    let expected_raw = v["expected_raw"]
        .as_i64()
        .expect("expected_raw must be i64") as i128;

    let left = FixedPoint::from_raw(left_raw);
    let right = FixedPoint::from_raw(right_raw);

    let got = left
        .checked_mul(right)
        .unwrap_or_else(|e| panic!("[{}] checked_mul({}, {}) overflowed: {:?}", name, left_raw, right_raw, e));

    assert_eq!(
        got.raw(),
        expected_raw,
        "[{}] fixed_point mul: expected {} got {}",
        name,
        expected_raw,
        got.raw()
    );

    // Sanity: result fits within SCALE bounds (result of multiplying two numbers in [0, SCALE]).
    assert!(
        got.raw() >= 0 && got.raw() <= SCALE,
        "[{}] result {} out of [0, SCALE]",
        name,
        got.raw()
    );
}

// ---------------------------------------------------------------------------
// leaf_index: verify derive_leaf_index output matches expected_leaf_index_hex
// ---------------------------------------------------------------------------

fn run_leaf_index(name: &str, v: &serde_json::Value) {
    let validator_id = v["inputs"]["validator_id"]
        .as_u64()
        .expect("validator_id must be u64");
    let epoch = v["inputs"]["epoch"].as_u64().expect("epoch must be u64");
    let seed_hex = v["inputs"]["epoch_seed_hex"]
        .as_str()
        .expect("epoch_seed_hex must be a string");

    let seed_bytes = hex_decode(seed_hex);
    assert_eq!(
        seed_bytes.len(),
        32,
        "[{}] epoch_seed_hex must decode to exactly 32 bytes (got {})",
        name,
        seed_bytes.len()
    );
    let mut epoch_seed = [0u8; 32];
    epoch_seed.copy_from_slice(&seed_bytes);

    let expected_hex = v["expected_leaf_index_hex"]
        .as_str()
        .expect("expected_leaf_index_hex must be a string");
    let expected_bytes = hex_decode(expected_hex);
    assert_eq!(
        expected_bytes.len(),
        48,
        "[{}] expected_leaf_index_hex must decode to exactly 48 bytes (got {})",
        name,
        expected_bytes.len()
    );

    let got = derive_leaf_index(validator_id, epoch, &epoch_seed);

    assert_eq!(
        got.as_slice(),
        expected_bytes.as_slice(),
        "[{}] derive_leaf_index({}, {}, {}) MISMATCH\n  expected: {}\n  got:      {}",
        name,
        validator_id,
        epoch,
        seed_hex,
        expected_hex,
        hex_encode(&got)
    );

    // Also exercise verify_leaf_index.
    assert!(
        qash_consensus::derive::verify_leaf_index(validator_id, epoch, &epoch_seed, &got),
        "[{}] verify_leaf_index returned false for its own output",
        name
    );
}

// ---------------------------------------------------------------------------
// genesis_noop_epochs: replay noop epochs from all-zero genesis and verify roots
// ---------------------------------------------------------------------------

fn run_genesis_noop_epochs(name: &str, v: &serde_json::Value) {
    let validator_count = v["validator_count"]
        .as_u64()
        .unwrap_or(4) as u32;

    let epochs = v["epochs"].as_array().expect("epochs must be an array");
    assert!(!epochs.is_empty(), "[{}] epochs array must not be empty", name);

    // Epochs must be contiguous starting at 1.
    for (i, ep) in epochs.iter().enumerate() {
        let expected_epoch = (i + 1) as u64;
        let stored_epoch = ep["epoch"].as_u64().expect("epoch must be u64");
        assert_eq!(
            stored_epoch, expected_epoch,
            "[{}] epoch[{}] expected epoch number {}, got {}",
            name, i, expected_epoch, stored_epoch
        );
    }

    let mut state = genesis_state(validator_count);

    for ep in epochs {
        let expected_root_hex = ep["expected_state_root_hex"]
            .as_str()
            .expect("expected_state_root_hex must be a string");
        let expected_root = hex_decode(expected_root_hex);
        assert_eq!(
            expected_root.len(),
            32,
            "[{}] expected_state_root_hex must be 32 bytes",
            name
        );

        let expected_lyapunov = ep["expected_lyapunov_raw"].as_i64().unwrap_or(0) as i128;
        let expected_phi = ep["expected_phi_safety_raw"].as_i64().unwrap_or(0) as i128;
        let expected_halted = ep["expected_halted"].as_bool().unwrap_or(false);

        // inputs array: currently only empty-input epochs are supported.
        let inputs = ep["inputs"].as_array().expect("inputs must be an array");
        assert!(
            inputs.is_empty(),
            "[{}] non-empty epoch inputs are not yet supported by vector_runner",
            name
        );

        let input = idle_input(state.validator_count);
        let result = advance_epoch(&mut state, &input, &[])
            .unwrap_or_else(|h| panic!("[{}] advance_epoch failed: {:?}", name, h));

        assert_eq!(
            state.state_root.as_slice(),
            expected_root.as_slice(),
            "[{}] epoch {} state_root MISMATCH\n  expected: {}\n  got:      {}",
            name,
            state.epoch,
            expected_root_hex,
            hex_encode(&state.state_root)
        );

        assert_eq!(
            result.lyapunov.v_convergence.raw(),
            expected_lyapunov,
            "[{}] epoch {} v_convergence: expected {} got {}",
            name,
            state.epoch,
            expected_lyapunov,
            result.lyapunov.v_convergence.raw()
        );

        assert_eq!(
            result.lyapunov.phi_safety.raw(),
            expected_phi,
            "[{}] epoch {} phi_safety: expected {} got {}",
            name,
            state.epoch,
            expected_phi,
            result.lyapunov.phi_safety.raw()
        );

        assert_eq!(
            state.is_halted(),
            expected_halted,
            "[{}] epoch {} halted: expected {} got {}",
            name,
            state.epoch,
            expected_halted,
            state.is_halted()
        );
    }
}

// ---------------------------------------------------------------------------
// Regeneration helper — run with: cargo test --test vector_runner regen -- --ignored
// ---------------------------------------------------------------------------

/// Print current code-derived outputs so vectors.v1.json can be updated after
/// deliberate spec changes. Verify on ALL three authorized ISAs before committing.
#[ignore]
#[test]
fn regen() {
    // fixed_point
    let left = FixedPoint::from_raw(400_000);
    let right = FixedPoint::from_raw(350_000);
    let fp = left.checked_mul(right).unwrap();
    println!("fixed_point_0_4_times_0_35.expected_raw = {}", fp.raw());

    // leaf_index
    let seed = [0xabu8; 32];
    let leaf = derive_leaf_index(1, 2, &seed);
    println!("leaf_index.expected_leaf_index_hex = {}", hex_encode(&leaf));

    // genesis_noop_epochs (4 validators)
    let mut state = genesis_state(4);
    for epoch in 1..=2u64 {
        let r = advance_epoch(&mut state, &idle_input(4), &[]).unwrap();
        println!(
            "genesis_noop_epochs epoch {}: root={}, lyapunov={}, phi={}",
            epoch,
            hex_encode(&state.state_root),
            r.lyapunov.v_convergence.raw(),
            r.lyapunov.phi_safety.raw()
        );
    }
}
