use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
/// Coq ↔ Rust parity test: validates advance_epoch() against proofs/model/vectors.json.
///
/// Issue #19 — Practical Coq ↔ Rust Integration.
///
/// The JSON file is the bridge between:
///   proofs/model/Model.v  (Coq executable spec, theorems TH-3a/TH-3b/TH-6)
///   advance_epoch()        (Rust implementation in transition.rs)
///
/// Any divergence between the Rust output and the stored vectors signals that the
/// Rust implementation has drifted from the formal model. Regenerate vectors using
/// the `gen_coq_vectors` test in gen_vectors.rs (pass --ignored --nocapture).
///
/// TV-10/TV-11 cover TX-0 (NoOp) and TX-1 (score decrement) cases — see
/// Coq theorems TX0_perturbation_zero and TX1_score_decrement_nonincreasing.
use qash_consensus::transaction::{TX0_WIRE_BYTES, TX1_WIRE_BYTES, TX1_PAYLOAD_BYTES, TX_TYPE_NOOP, TX_TYPE_SCORE_DECREMENT, TX_VERSION};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

// ---------------------------------------------------------------------------
// Minimal JSON field extractor — no external dep, controls trusted format.
// ---------------------------------------------------------------------------

fn extract_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\":\"", key);
    let start = json.find(&needle)? + needle.len();
    let end = json[start..].find('"')? + start;
    Some(&json[start..end])
}

fn extract_num(json: &str, key: &str) -> Option<i64> {
    let needle_q = format!("\"{}\":", key);
    let start = json.find(&needle_q)? + needle_q.len();
    let rest = json[start..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn extract_bool(json: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{}\":", key);
    let start = json.find(&needle)? + needle.len();
    let rest = json[start..].trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn hex_to_32(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate().take(32) {
        out[i] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap();
    }
    out
}

// ---------------------------------------------------------------------------
// State helpers
// ---------------------------------------------------------------------------

fn genesis(vc: u32) -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: vc,
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

fn idle_input(vc: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: qash_consensus::envelope::PROTOCOL_VERSION_V1_1,
        update_count: vc,
    }
}

fn halt_reason_from_u8(b: u8) -> HaltReason {
    match b {
        1 => HaltReason::LyapunovViolation,
        2 => HaltReason::ArithOverflow,
        3 => HaltReason::EpochOverflow,
        4 => HaltReason::DecodeInvalid,
        5 => HaltReason::RoundtripFailure,
        6 => HaltReason::HaltFlagSet,
        7 => HaltReason::PhiSafetyViolation,
        _ => HaltReason::None,
    }
}

// ---------------------------------------------------------------------------
// Per-TV execution helpers
// ---------------------------------------------------------------------------

fn make_tx0_bytes(author_id: [u8; 48], nonce: u64) -> [u8; TX0_WIRE_BYTES] {
    let mut raw = [0u8; TX0_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
    raw[4..12].copy_from_slice(&nonce.to_le_bytes());
    raw[12..60].copy_from_slice(&author_id);
    raw[60..64].copy_from_slice(&0u32.to_le_bytes());
    raw
}

fn make_tx1_bytes(author_id: [u8; 48], nonce: u64, target_idx: u32, delta: u32) -> [u8; TX1_WIRE_BYTES] {
    let mut raw = [0u8; TX1_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_SCORE_DECREMENT.to_le_bytes());
    raw[4..12].copy_from_slice(&nonce.to_le_bytes());
    raw[12..60].copy_from_slice(&author_id);
    raw[60..64].copy_from_slice(&(TX1_PAYLOAD_BYTES as u32).to_le_bytes());
    raw[64..68].copy_from_slice(&target_idx.to_le_bytes());
    raw[68..72].copy_from_slice(&delta.to_le_bytes());
    raw
}

fn genesis_with_ids(vc: u32) -> Box<EpochState> {
    let mut s = Box::new(genesis(vc));
    for i in 0..vc as usize {
        s.validator_ids[i][0] = (i as u8) + 1;
    }
    s
}

fn run_tv0(vc: u32) -> EpochState {
    genesis(vc)
}

fn run_idle_epochs(vc: u32, n: usize) -> EpochState {
    let mut s = genesis(vc);
    for _ in 0..n {
        advance_epoch(&mut s, &idle_input(vc), &[]).unwrap();
    }
    s
}

fn run_sub_epsilon_spike(vc: u32, divergence: i128) -> EpochState {
    let mut s = genesis(vc);
    let mut input = idle_input(vc);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(divergence),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    advance_epoch(&mut s, &input, &[]).unwrap();
    s
}

fn run_window_fill_then_spike(vc: u32) -> EpochState {
    use qash_consensus::lyapunov::WINDOW_SIZE;
    let mut s = genesis(vc);
    for _ in 0..WINDOW_SIZE {
        advance_epoch(&mut s, &idle_input(vc), &[]).unwrap();
    }
    let mut spike = idle_input(vc);
    for i in 0..vc as usize {
        spike.updates[i] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(900_000),
            conflict_new: FixedPoint::from_raw(900_000),
            slash_accum_new: FixedPoint::ZERO,
        });
    }
    let _ = advance_epoch(&mut s, &spike, &[]);
    s
}

fn run_halt_absorbing(vc: u32) -> EpochState {
    let mut s = run_window_fill_then_spike(vc);
    for _ in 0..5 {
        let _ = advance_epoch(&mut s, &idle_input(vc), &[]);
    }
    s
}

fn run_decode_invalid_update_count(vc: u32) -> EpochState {
    let mut s = genesis(vc);
    let bad = EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: qash_consensus::envelope::PROTOCOL_VERSION_V1_1,
        update_count: vc - 1,
    };
    let _ = advance_epoch(&mut s, &bad, &[]);
    s
}

fn run_decode_invalid_slash_decrease(vc: u32) -> EpochState {
    let mut s = genesis(vc);
    s.validators[0].slash_accum = FixedPoint::from_raw(1_000);
    let mut input = idle_input(vc);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::ZERO,
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::from_raw(500),
    });
    let _ = advance_epoch(&mut s, &input, &[]);
    s
}

// TX sequence for the first transaction from each validator in a fresh genesis state.
// This is a replay-protection counter, not a cryptographic nonce.
const GENESIS_TX_SEQ: u64 = 0;

fn run_tx0_noop(vc: u32) -> Box<EpochState> {
    let mut s = genesis_with_ids(vc);
    let author_id = s.validator_ids[0];
    let tx0 = make_tx0_bytes(author_id, GENESIS_TX_SEQ);
    advance_epoch(&mut *s, &idle_input(vc), &[tx0.as_slice()]).unwrap();
    s
}

fn run_tx1_score_decrement(vc: u32) -> Box<EpochState> {
    let mut s = genesis_with_ids(vc);
    let mut input = idle_input(vc);
    input.updates[0] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::from_raw(500_000),
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    let author_id = s.validator_ids[0];
    // target_idx=0: TX-1 decrements validator 0's score; GENESIS_TX_SEQ: first TX from author
    let tx1 = make_tx1_bytes(author_id, GENESIS_TX_SEQ, 0, 200_000);
    advance_epoch(&mut *s, &input, &[tx1.as_slice()]).unwrap();
    s
}

// ---------------------------------------------------------------------------
// The parity test — runs each TV and asserts against the stored vector.
// ---------------------------------------------------------------------------

struct Expected {
    id: String,
    desc: String,
    epoch: u64,
    halt: bool,
    halt_reason: u8,
    state_root: [u8; 32],
    entropy_seed: Option<[u8; 32]>,
    seed_nonzero: Option<bool>,
    nonce0: Option<u64>,
}

fn assert_state(state: &EpochState, exp: &Expected) {
    assert_eq!(
        state.epoch, exp.epoch,
        "{}: epoch mismatch (got {}, want {})",
        exp.id, state.epoch, exp.epoch
    );
    let halted = state.halt_reason != HaltReason::None;
    assert_eq!(halted, exp.halt, "{}: halt flag mismatch", exp.id);
    assert_eq!(
        state.halt_reason as u8, exp.halt_reason,
        "{}: halt_reason mismatch",
        exp.id
    );
    assert_eq!(
        state.state_root, exp.state_root,
        "{}: state_root mismatch\n  got:  {:?}\n  want: {:?}",
        exp.id, state.state_root, exp.state_root
    );
    if let Some(seed) = exp.entropy_seed {
        assert_eq!(
            state.entropy_seed, seed,
            "{}: entropy_seed mismatch",
            exp.id
        );
    }
    if let Some(nonzero) = exp.seed_nonzero {
        assert_eq!(
            state.entropy_seed != [0u8; 32],
            nonzero,
            "{}: seed_nonzero mismatch",
            exp.id
        );
    }
    if let Some(nonce0) = exp.nonce0 {
        assert_eq!(
            state.nonces[0], nonce0,
            "{}: nonce0 mismatch (got {}, want {})",
            exp.id, state.nonces[0], nonce0
        );
    }
}

#[test]
fn coq_model_parity() {
    let json = include_str!("../../../proofs/model/vectors.json");

    // Split into individual vector objects by finding each {"id": block.
    let parts: Vec<&str> = json.split(r#"{"id":"#).skip(1).collect();

    assert!(!parts.is_empty(), "no vectors found in vectors.json");

    for raw in &parts {
        let id_end = raw.find('"').unwrap_or(raw.len());
        let id = format!(
            "TV-{}",
            &raw[..id_end]
                .trim_matches(|c: char| c == 'T' || c == 'V' || c == '-' || c.is_ascii_digit())
        );
        let full = format!("{{\"id\":{}", raw);

        let id_val = extract_str(&full, "id").unwrap_or("").to_string();
        let desc = extract_str(&full, "desc").unwrap_or("").to_string();
        let vc = extract_num(&full, "vc").unwrap_or(4) as u32;

        // Parse expect block
        let expect_start = full.find("\"expect\":").unwrap_or(0);
        let expect_block = &full[expect_start..];

        let epoch = extract_num(expect_block, "epoch").unwrap_or(0) as u64;
        let halt = extract_bool(expect_block, "halt").unwrap_or(false);
        let halt_reason = extract_num(expect_block, "halt_reason").unwrap_or(0) as u8;
        let root_hex = extract_str(expect_block, "state_root").unwrap_or("");
        let state_root = hex_to_32(root_hex);

        let entropy_seed = extract_str(expect_block, "entropy_seed").map(hex_to_32);
        let seed_nonzero = extract_bool(expect_block, "seed_nonzero");
        let nonce0 = extract_num(expect_block, "nonce0").map(|n| n as u64);

        let exp = Expected {
            id: id_val.clone(),
            desc,
            epoch,
            halt,
            halt_reason: halt_reason_from_u8(halt_reason) as u8,
            state_root,
            entropy_seed,
            seed_nonzero,
            nonce0,
        };

        let state: EpochState = match id_val.as_str() {
            "TV-0" => run_tv0(vc),
            "TV-1" => run_idle_epochs(vc, 1),
            "TV-2" => run_idle_epochs(vc, 3),
            "TV-3" => run_sub_epsilon_spike(vc, 10_000),
            "TV-4" => run_window_fill_then_spike(vc),
            "TV-5" => run_halt_absorbing(vc),
            "TV-6" => run_decode_invalid_update_count(vc),
            "TV-7" => run_decode_invalid_slash_decrease(vc),
            "TV-8" => run_idle_epochs(vc, 3),
            "TV-9" => run_idle_epochs(vc, 2),
            "TV-10" => *run_tx0_noop(vc),
            "TV-11" => *run_tx1_score_decrement(vc),
            other => panic!("unknown vector id: {}", other),
        };

        assert_state(&state, &exp);
        println!("  {} [{}]: OK — {}", id_val, id, exp.desc);
    }

    println!("\ncoq_model_parity: all {} vectors passed", parts.len());
}
