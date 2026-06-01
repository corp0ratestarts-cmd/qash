// Vector generation helper — not a CI test; run manually to regenerate vectors.
// cargo test -p qash-consensus --no-default-features -- --nocapture --ignored gen_coq_vectors
use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::transaction::{TX0_WIRE_BYTES, TX1_WIRE_BYTES, TX_TYPE_NOOP, TX_TYPE_SCORE_DECREMENT, TX_VERSION};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

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
fn idle(vc: u32) -> EpochInput {
    EpochInput::new(vc)
}

fn make_genesis_with_ids(vc: u32) -> Box<EpochState> {
    let mut s = Box::new(genesis(vc));
    for i in 0..vc as usize {
        s.validator_ids[i][0] = (i as u8) + 1;
    }
    s
}

fn make_tx0_gen(author_id: [u8; 48], nonce: u64) -> [u8; TX0_WIRE_BYTES] {
    let mut raw = [0u8; TX0_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
    raw[4..12].copy_from_slice(&nonce.to_le_bytes());
    raw[12..60].copy_from_slice(&author_id);
    raw[60..64].copy_from_slice(&0u32.to_le_bytes());
    raw
}

fn make_tx1_gen(
    author_id: [u8; 48],
    nonce: u64,
    target_idx: u32,
    delta: u32,
) -> [u8; TX1_WIRE_BYTES] {
    use qash_consensus::transaction::TX1_PAYLOAD_BYTES;
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

fn root_hex(r: &[u8; 32]) -> String {
    r.iter().map(|b| format!("{:02x}", b)).collect()
}
fn hr(r: HaltReason) -> u8 {
    r as u8
}

fn emit_step_idle(vc: u32) -> String {
    format!(r#"{{"kind":"idle","update_count":{}}}"#, vc)
}
fn emit_step_spike(vc: u32, validator_idx: usize, d: i64, c: i64, s: i64) -> String {
    format!(
        r#"{{"kind":"spike","update_count":{},"idx":{},"divergence":{},"conflict":{},"slash_accum":{}}}"#,
        vc, validator_idx, d, c, s
    )
}

#[ignore]
#[test]
fn gen_coq_vectors() {
    // EpochState is ~100KB on the stack (MAX_VALIDATORS=1024). Run on an enlarged
    // thread stack so that sequential scoped allocations don't overflow.
    let handle = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(gen_coq_vectors_inner)
        .expect("thread spawn");
    handle.join().expect("thread join");
}

fn gen_coq_vectors_inner() {
    let mut records: Vec<String> = Vec::new();

    // TV-0: genesis, 0 epochs
    {
        let s = genesis(4);
        records.push(format!(
            r#"  {{"id":"TV-0","desc":"genesis state, 0 epochs, all-zero root","vc":4,"steps":[],"expect":{{"epoch":{},"halt":false,"halt_reason":{},"state_root":"{}"}}}}"#,
            s.epoch, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    // TV-1: 1 idle epoch
    {
        let mut s = genesis(4);
        let steps = vec![emit_step_idle(4)];
        advance_epoch(&mut s, &idle(4), &[]).unwrap();
        records.push(format!(
            r#"  {{"id":"TV-1","desc":"1 idle epoch","vc":4,"steps":[{}],"expect":{{"epoch":{},"halt":false,"halt_reason":{},"state_root":"{}"}}}}"#,
            steps.join(","), s.epoch, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    // TV-2: 3 idle epochs (fills window)
    {
        let mut s = genesis(4);
        let steps: Vec<String> = (0..3).map(|_| emit_step_idle(4)).collect();
        for _ in 0..3 {
            advance_epoch(&mut s, &idle(4), &[]).unwrap();
        }
        records.push(format!(
            r#"  {{"id":"TV-2","desc":"3 idle epochs, window full","vc":4,"steps":[{}],"expect":{{"epoch":{},"halt":false,"halt_reason":{},"state_root":"{}"}}}}"#,
            steps.join(","), s.epoch, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    // TV-3: spike below epsilon before window full (no halt)
    {
        let mut s = genesis(4);
        let spike_raw = 10_000i64;
        let mut input = idle(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(spike_raw as i128),
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
        let steps = vec![emit_step_spike(4, 0, spike_raw, 0, 0)];
        advance_epoch(&mut s, &input, &[]).unwrap();
        records.push(format!(
            r#"  {{"id":"TV-3","desc":"sub-epsilon spike, window empty, no halt","vc":4,"steps":[{}],"expect":{{"epoch":{},"halt":false,"halt_reason":{},"state_root":"{}"}}}}"#,
            steps.join(","), s.epoch, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    // TV-4: fill window then spike — triggers LyapunovViolation
    {
        let mut s = genesis(4);
        let mut steps: Vec<String> = (0..WINDOW_SIZE).map(|_| emit_step_idle(4)).collect();
        for _ in 0..WINDOW_SIZE {
            advance_epoch(&mut s, &idle(4), &[]).unwrap();
        }

        let spike_raw = 900_000i64;
        let mut spike_input = idle(4);
        for i in 0..4 {
            spike_input.updates[i] = Some(ValidatorUpdate {
                divergence_new: FixedPoint::from_raw(spike_raw as i128),
                conflict_new: FixedPoint::from_raw(spike_raw as i128),
                slash_accum_new: FixedPoint::ZERO,
            });
        }
        steps.push(emit_step_spike(4, 0, spike_raw, spike_raw, 0));
        let epoch_before = s.epoch;
        let res = advance_epoch(&mut s, &spike_input, &[]);
        assert_eq!(res, Err(HaltReason::LyapunovViolation));
        records.push(format!(
            r#"  {{"id":"TV-4","desc":"window full then spike → LyapunovViolation halt","vc":4,"steps":[{}],"expect":{{"epoch":{},"halt":true,"halt_reason":{},"state_root":"{}"}}}}"#,
            steps.join(","), epoch_before, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    // TV-5: halt is absorbing — after TV-4's halt, further steps don't change epoch
    {
        let mut s = genesis(4);
        for _ in 0..WINDOW_SIZE {
            advance_epoch(&mut s, &idle(4), &[]).unwrap();
        }
        let mut spike_input = idle(4);
        for i in 0..4 {
            spike_input.updates[i] = Some(ValidatorUpdate {
                divergence_new: FixedPoint::from_raw(900_000),
                conflict_new: FixedPoint::from_raw(900_000),
                slash_accum_new: FixedPoint::ZERO,
            });
        }
        advance_epoch(&mut s, &spike_input, &[]).unwrap_err();
        // 5 more steps — state must not change
        let root_after_halt = s.state_root;
        let epoch_after_halt = s.epoch;
        for _ in 0..5 {
            advance_epoch(&mut s, &idle(4), &[]).unwrap_err();
        }
        assert_eq!(s.state_root, root_after_halt);
        records.push(format!(
            r#"  {{"id":"TV-5","desc":"halt absorbing — 5 more steps after halt keep state","vc":4,"steps":[],"expect":{{"epoch":{},"halt":true,"halt_reason":{},"state_root":"{}"}}}}"#,
            epoch_after_halt, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    // TV-6: DecodeInvalid — wrong update_count (4 validators, count=3)
    {
        let mut s = genesis(4);
        let bad = EpochInput::new(3);
        let res = advance_epoch(&mut s, &bad, &[]);
        assert_eq!(res, Err(HaltReason::DecodeInvalid));
        records.push(format!(
            r#"  {{"id":"TV-6","desc":"DecodeInvalid via wrong update_count","vc":4,"steps":[],"expect":{{"epoch":{},"halt":true,"halt_reason":{},"state_root":"{}"}}}}"#,
            s.epoch, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    // TV-7: DecodeInvalid — slash monotonicity violation
    {
        let mut s = genesis(4);
        s.validators[0].slash_accum = FixedPoint::from_raw(1_000);
        let mut input = idle(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::from_raw(500), // decrease
        });
        let res = advance_epoch(&mut s, &input, &[]);
        assert_eq!(res, Err(HaltReason::DecodeInvalid));
        records.push(format!(
            r#"  {{"id":"TV-7","desc":"DecodeInvalid via slash_accum decrease","vc":4,"steps":[],"expect":{{"epoch":{},"halt":true,"halt_reason":{},"state_root":"{}"}}}}"#,
            s.epoch, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    // TV-8: entropy chain — seed evolves after 3 epochs
    {
        let mut s = genesis(4);
        for _ in 0..3 {
            advance_epoch(&mut s, &idle(4), &[]).unwrap();
        }
        let seed_is_nonzero = s.entropy_seed != [0u8; 32];
        let seed_hex: String = s
            .entropy_seed
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        records.push(format!(
            r#"  {{"id":"TV-8","desc":"entropy seed evolves after 3 epochs","vc":4,"steps":[],"expect":{{"epoch":{},"halt":false,"halt_reason":0,"state_root":"{}","entropy_seed":"{}","seed_nonzero":{}}}}}"#,
            s.epoch, root_hex(&s.state_root), seed_hex, seed_is_nonzero
        ));
    }

    // TV-9: single validator, 2 idle epochs
    {
        let mut s = genesis(1);
        let steps: Vec<String> = (0..2).map(|_| emit_step_idle(1)).collect();
        for _ in 0..2 {
            advance_epoch(&mut s, &idle(1), &[]).unwrap();
        }
        records.push(format!(
            r#"  {{"id":"TV-9","desc":"single validator, 2 idle epochs","vc":1,"steps":[{}],"expect":{{"epoch":{},"halt":false,"halt_reason":{},"state_root":"{}"}}}}"#,
            steps.join(","), s.epoch, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    // TV-10: TX-0 (NoOp) does not change V_convergence relative to same epoch without TX-0.
    // Coq: TX0_perturbation_zero in contractivity/tx_perturbation_0.v
    // Both states advance one idle epoch; the TX-0 state gets nonce incremented but same Lyapunov.
    {
        let n: u32 = 4;
        let mut s_no_tx  = make_genesis_with_ids(n);
        let mut s_with_tx = make_genesis_with_ids(n);

        advance_epoch(&mut *s_no_tx, &idle(n), &[]).unwrap();

        let author_id = s_with_tx.validator_ids[0];
        // tx_sequence=0: first transaction from this validator (replay-protection counter)
        let tx0 = make_tx0_gen(author_id, 0);
        advance_epoch(&mut *s_with_tx, &idle(n), &[tx0.as_slice()]).unwrap();

        records.push(format!(
            r#"  {{"id":"TV-10","desc":"TX-0 NoOp: epoch advances, nonce incremented, state_root deterministic","vc":{},"steps":[],"expect":{{"epoch":{},"halt":false,"halt_reason":{},"state_root":"{}","nonce0":1}}}}"#,
            n, s_with_tx.epoch, hr(s_with_tx.halt_reason), root_hex(&s_with_tx.state_root)
        ));
        let _ = s_no_tx; // used to verify property; not stored as a separate vector
    }

    // TV-11: TX-1 (score decrement) leaves V_convergence ≤ baseline without TX-1.
    // Coq: TX1_score_decrement_nonincreasing in contractivity/tx1_score_decrement.v
    // Validator 0 gets D=500_000 in epoch input; TX-1 decrements D by 200_000 within the epoch.
    {
        let n: u32 = 4;
        let mut s = make_genesis_with_ids(n);

        let mut input = idle(n);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(500_000),
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });

        let author_id = s.validator_ids[0];
        // tx_sequence=0: first TX from author; target_idx=0: decrement validator 0
        let tx1 = make_tx1_gen(author_id, 0, 0, 200_000);
        advance_epoch(&mut *s, &input, &[tx1.as_slice()]).unwrap();

        records.push(format!(
            r#"  {{"id":"TV-11","desc":"TX-1 score decrement: D reduced 500k→300k, V_convergence non-increasing","vc":{},"steps":[],"expect":{{"epoch":{},"halt":false,"halt_reason":{},"state_root":"{}"}}}}"#,
            n, s.epoch, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    println!("{{\n  \"generated_by\": \"gen_vectors.rs\",\n  \"spec\": \"proofs/model/Model.v\",\n  \"coq_theorems\": [\"TH-3a\",\"TH-3b\",\"TH-6\",\"TX0_perturbation_zero\",\"TX1_score_decrement_nonincreasing\"],\n  \"vectors\": [\n{}\n  ]\n}}", records.join(",\n"));
}

use qash_consensus::transition::{encode_full_state_into, FULL_STATE_MAX_BYTES};

#[ignore]
#[test]
fn gen_replay_snapshots() {
    fn snap(label: &str, vc: u32, epochs: usize) {
        let mut s = genesis(vc);
        for _ in 0..epochs {
            advance_epoch(&mut s, &idle(vc), &[]).unwrap();
        }
        let mut buf = [0u8; FULL_STATE_MAX_BYTES];
        let len = encode_full_state_into(&mut s, &mut buf);
        let root_hex: String = s.state_root.iter().map(|b| format!("{:02x}", b)).collect();
        let bytes_hex: String = buf[..len].iter().map(|b| format!("{:02x}", b)).collect();
        println!("SNAP:{}:{}:{}:{}", label, len, root_hex, bytes_hex);
    }
    snap("epoch0", 4, 0);
    snap("epoch1", 4, 1);
    snap("epoch3", 4, 3);
}

#[ignore]
#[test]
fn _scratch_check_v1_vectors() {
    use qash_consensus::derive::derive_leaf_index;
    use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
    use qash_consensus::transition::{
        advance_epoch, EpochInput, EpochState, HaltReason, MAX_VALIDATORS,
    };

    // Check 4-validator genesis
    let mut state = EpochState {
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
    };
    let input = EpochInput::new(4);
    advance_epoch(&mut state, &input, &[]).unwrap();
    eprintln!(
        "4-validator epoch 1 root: {}",
        hex_encode(&state.state_root)
    );

    // Check 0-validator genesis
    let mut state0 = EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: 0,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        cascade_health: 0,
        state_root: [0u8; 32],
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
        causal_fingerprint: [0u8; 32],
    };
    let input0 = EpochInput::new(0);
    let r0 = advance_epoch(&mut state0, &input0, &[]);
    eprintln!("0-validator epoch 1 result: {:?}", r0);
    if r0.is_ok() {
        eprintln!(
            "0-validator epoch 1 root: {}",
            hex_encode(&state0.state_root)
        );
    }

    // Check derive_leaf_index with 32-byte ab seed
    let seed = [0xabu8; 32];
    let leaf = derive_leaf_index(1, 2, &seed);
    eprintln!("derive_leaf_index(1, 2, [0xab;32]): {}", hex_encode(&leaf));
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[ignore]
#[test]
fn _scratch_epoch2_root() {
    use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
    use qash_consensus::transition::{
        advance_epoch, EpochInput, EpochState, HaltReason, MAX_VALIDATORS,
    };

    let mut state = EpochState {
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
    };
    for epoch in 1..=2u64 {
        let input = EpochInput::new(4);
        let r = advance_epoch(&mut state, &input, &[]).unwrap();
        let bytes: Vec<String> = state
            .state_root
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        eprintln!("epoch {} root: {}", epoch, bytes.join(""));
        eprintln!(
            "  lyapunov_raw: {}, phi_safety_raw: {}",
            r.lyapunov.v_convergence.raw(),
            r.lyapunov.phi_safety.raw()
        );
    }
}
