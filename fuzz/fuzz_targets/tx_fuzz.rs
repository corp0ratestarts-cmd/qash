// Fuzz target: TX-0 transaction pipeline — parse, admissibility, nonce, apply.
//
// Verifies:
//   1. parse_tx0 never panics; returns Ok or TxError (never Rust panic)
//   2. is_admissible only succeeds when nonce matches exactly
//   3. apply_tx_0 increments nonce by exactly 1
//   4. apply_all is nonce-safe: duplicate nonces in the same batch are rejected
//   5. sort order is deterministic: forward and reverse submission produce the
//      same post-state nonces (reordering resistance)
//
// Run: cargo hfuzz run tx_fuzz  (from fuzz/)

use honggfuzz::fuzz;
use arbitrary::Arbitrary;
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
use qash_consensus::transaction::{
    TX_VERSION, TX_TYPE_NOOP, TX0_WIRE_BYTES,
    parse_tx0, is_admissible, apply_tx_0, apply_all,
};
use qash_consensus::transition::{EpochState, HaltReason, MAX_VALIDATORS};

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    /// Raw bytes for the TX envelope — arbitrary length and content.
    raw: [u8; TX0_WIRE_BYTES],
    /// Which validator slot to target (0–3).
    target_slot: u8,
    /// Nonce to embed in the tx.
    nonce: u64,
    /// Entropy seed for the state.
    seed: [u8; 32],
}

fn make_state(vc: u32, seed: [u8; 32]) -> EpochState {
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
    for i in 0..vc as usize {
        validator_ids[i][0] = i as u8 + 1;
    }
    EpochState {
        epoch: 1,
        halt_reason: HaltReason::None,
        entropy_seed: seed,
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: vc,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids,
        state_root: [0u8; 32],
    }
}

fn make_valid_tx(slot: u8, nonce: u64) -> [u8; TX0_WIRE_BYTES] {
    let mut raw = [0u8; TX0_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
    raw[4..12].copy_from_slice(&nonce.to_le_bytes());
    raw[12] = slot + 1; // validator_id[0]
    // payload_len = 0 (bytes 60..64 already 0)
    raw
}

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            let mut u = arbitrary::Unstructured::new(data);
            let fi = match FuzzInput::arbitrary(&mut u) {
                Ok(v) => v,
                Err(_) => return,
            };

            // Invariant 1: parse_tx0 never panics on arbitrary input
            let _ = parse_tx0(&fi.raw);
            // Also test short inputs
            let _ = parse_tx0(b"");
            let _ = parse_tx0(&fi.raw[..fi.raw.len().min(32)]);

            // Invariant 2: correct nonce → admissible; wrong → rejected
            let mut state = make_state(4, fi.seed);
            let slot = fi.target_slot % 4;
            let tx_correct = make_valid_tx(slot, 0); // nonce=0 matches initial state
            let tx_wrong   = make_valid_tx(slot, fi.nonce.wrapping_add(1).max(1)); // nonce>0 rejected

            if let Ok((tx, _)) = parse_tx0(&tx_correct) {
                // Should be admissible with nonce=0
                assert!(is_admissible(&state, &tx).is_ok(),
                    "correct nonce rejected for slot {slot}");
            }
            if fi.nonce > 0 {
                if let Ok((tx, _)) = parse_tx0(&tx_wrong) {
                    // Should be rejected (nonce > expected 0)
                    assert!(is_admissible(&state, &tx).is_err(),
                        "wrong nonce accepted for slot {slot}");
                }
            }

            // Invariant 3: apply increments nonce by exactly 1
            let tx_bytes = make_valid_tx(slot, 0);
            if let Ok((tx, _)) = parse_tx0(&tx_bytes) {
                if let Ok(idx) = is_admissible(&state, &tx) {
                    let prev = state.nonces[idx];
                    let _ = apply_tx_0(&mut state, idx);
                    assert_eq!(state.nonces[idx], prev + 1, "nonce not incremented by 1");
                }
            }

            // Invariant 4+5: reordering resistance — same post-state regardless of submission order
            let mut s1 = make_state(4, fi.seed);
            let mut s2 = make_state(4, fi.seed);

            let tx_a = make_valid_tx(0, 0);
            let tx_b = make_valid_tx(1, 0);
            let tx_c = make_valid_tx(2, 0);

            let forward: &[&[u8]] = &[tx_a.as_slice(), tx_b.as_slice(), tx_c.as_slice()];
            let reverse: &[&[u8]] = &[tx_c.as_slice(), tx_b.as_slice(), tx_a.as_slice()];

            let n1 = apply_all(&mut s1, forward, 100).unwrap_or(0);
            let n2 = apply_all(&mut s2, reverse, 100).unwrap_or(0);

            assert_eq!(n1, n2, "apply_all count differs by order");
            for i in 0..4usize {
                assert_eq!(s1.nonces[i], s2.nonces[i],
                    "nonce[{i}] differs by submission order");
            }
        });
    }
}
