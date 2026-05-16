use std::fs;
use std::path::Path;

use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

const VECTOR_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/vectors/deterministic_vectors.json"
);

fn genesis_state_with_count(count: u32) -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: count,
        convergence_window: ConvergenceWindow::new(),
    }
}

fn idle_input(n: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        update_count: n,
    }
}

fn normalize_input(n: u32, items: &[(usize, i128, i128, i128)]) -> EpochInput {
    let mut input = idle_input(n);
    for (idx, d, c, s) in items {
        input.updates[*idx] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(*d),
            conflict_new: FixedPoint::from_raw(*c),
            slash_accum_new: FixedPoint::from_raw(*s),
        });
    }
    input
}

fn to_hex(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn deterministic_vector_payload() -> String {
    // 1) epoch transitions at edges
    let mut min_state = genesis_state_with_count(1);
    let min_root = advance_epoch(&mut min_state, &idle_input(1))
        .unwrap()
        .state_root;

    let mut max_state = genesis_state_with_count(MAX_VALIDATORS as u32);
    let max_root = advance_epoch(&mut max_state, &idle_input(MAX_VALIDATORS as u32))
        .unwrap()
        .state_root;

    let mut slash_state = genesis_state_with_count(2);
    let ok = normalize_input(2, &[(0, 0, 0, 2000), (1, 0, 0, 1000)]);
    let _ = advance_epoch(&mut slash_state, &ok).unwrap();
    let bad = normalize_input(2, &[(0, 0, 0, 1500), (1, 0, 0, 1200)]);
    let slash_halt = advance_epoch(&mut slash_state, &bad).unwrap_err();

    let mut empty_window_state = genesis_state_with_count(1);
    let empty_window_pass = advance_epoch(
        &mut empty_window_state,
        &normalize_input(1, &[(0, 1_000_000, 0, 0)]),
    )
    .is_ok();

    let mut full_window_state = genesis_state_with_count(1);
    for _ in 0..WINDOW_SIZE {
        let _ = advance_epoch(&mut full_window_state, &idle_input(1)).unwrap();
    }
    let full_window_halt = advance_epoch(
        &mut full_window_state,
        &normalize_input(1, &[(0, 1_000_000, 0, 0)]),
    )
    .unwrap_err();

    // 2) halt-path determinism
    let mut lyap_state = genesis_state_with_count(1);
    for _ in 0..WINDOW_SIZE {
        let _ = advance_epoch(&mut lyap_state, &idle_input(1)).unwrap();
    }
    let lyap_halt = advance_epoch(
        &mut lyap_state,
        &normalize_input(1, &[(0, 1_000_000, 0, 0)]),
    )
    .unwrap_err();

    let mut overflow_state = genesis_state_with_count(1);
    let overflow_input = normalize_input(1, &[(0, i128::from(i64::MAX), 0, 0)]);
    let overflow_halt = advance_epoch(&mut overflow_state, &overflow_input).unwrap_err();

    let mut decode_invalid_state = genesis_state_with_count(1);
    let mut decode_invalid_input = idle_input(1);
    decode_invalid_input.updates[1] = Some(ValidatorUpdate {
        divergence_new: FixedPoint::ZERO,
        conflict_new: FixedPoint::ZERO,
        slash_accum_new: FixedPoint::ZERO,
    });
    let decode_invalid_halt =
        advance_epoch(&mut decode_invalid_state, &decode_invalid_input).unwrap_err();

    // 3) canonical root across permutations that normalize identically
    let mut perm_a_state = genesis_state_with_count(3);
    let mut perm_b_state = genesis_state_with_count(3);

    let perm_a = normalize_input(3, &[(0, 7000, 1000, 10), (2, 3000, 4000, 40)]);
    let perm_b = normalize_input(3, &[(2, 3000, 4000, 40), (0, 7000, 1000, 10)]);

    let perm_a_root = advance_epoch(&mut perm_a_state, &perm_a)
        .unwrap()
        .state_root;
    let perm_b_root = advance_epoch(&mut perm_b_state, &perm_b)
        .unwrap()
        .state_root;

    format!(
        "{{
  \"schema\": \"qash-consensus-deterministic-v1\",
  \"edge_bounds\": {{
    \"min_validators_root\": \"{}\",
    \"max_validators_root\": \"{}\",
    \"slash_monotonicity_halt\": \"{:?}\",
    \"empty_window_allows_spike\": {},
    \"full_window_spike_halt\": \"{:?}\"
  }},
  \"halt_path\": {{
    \"lyapunov_violation\": \"{:?}\",
    \"arith_overflow\": \"{:?}\",
    \"decode_invalid\": \"{:?}\"
  }},
  \"canonical_permutations\": {{
    \"perm_a_root\": \"{}\",
    \"perm_b_root\": \"{}\"
  }}
}}\n",
        to_hex(&min_root),
        to_hex(&max_root),
        slash_halt,
        empty_window_pass,
        full_window_halt,
        lyap_halt,
        overflow_halt,
        decode_invalid_halt,
        to_hex(&perm_a_root),
        to_hex(&perm_b_root),
    )
}

#[test]
fn deterministic_vectors_match_golden() {
    let expected = fs::read_to_string(VECTOR_PATH).expect("missing golden vector file");
    let generated = deterministic_vector_payload();
    assert_eq!(
        generated, expected,
        "golden vectors are stale; regenerate with scripts/regenerate_consensus_vectors.sh"
    );
}

#[test]
fn regenerate_vectors_when_requested() {
    if std::env::var("QASH_REGENERATE_VECTORS").ok().as_deref() != Some("1") {
        return;
    }

    let payload = deterministic_vector_payload();
    let path = Path::new(VECTOR_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, payload).unwrap();
}
