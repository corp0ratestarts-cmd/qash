// Vector generation helper — not a CI test; run manually to regenerate vectors.
// cargo test -p qash-consensus --no-default-features -- --nocapture --ignored gen_coq_vectors
use qash_consensus::transition::{advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS};
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics, WINDOW_SIZE};
use qash_consensus::fixed_point::FixedPoint;

fn genesis(vc: u32) -> EpochState {
    EpochState {
        epoch: 0, halt_reason: HaltReason::None, entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: vc, convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS], validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        cascade_health: 0,
        state_root: [0u8; 32],
    }
}
fn idle(vc: u32) -> EpochInput { EpochInput::new(vc) }

fn root_hex(r: &[u8; 32]) -> String { r.iter().map(|b| format!("{:02x}", b)).collect() }
fn hr(r: HaltReason) -> u8 { r as u8 }

fn emit_step_idle(vc: u32) -> String {
    format!(r#"{{"kind":"idle","update_count":{}}}"#, vc)
}
fn emit_step_spike(vc: u32, validator_idx: usize, d: i64, c: i64, s: i64) -> String {
    format!(r#"{{"kind":"spike","update_count":{},"idx":{},"divergence":{},"conflict":{},"slash_accum":{}}}"#,
        vc, validator_idx, d, c, s)
}

#[ignore]
#[test]
fn gen_coq_vectors() {
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
        for _ in 0..3 { advance_epoch(&mut s, &idle(4), &[]).unwrap(); }
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
        for _ in 0..WINDOW_SIZE { advance_epoch(&mut s, &idle(4), &[]).unwrap(); }

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
        for _ in 0..WINDOW_SIZE { advance_epoch(&mut s, &idle(4), &[]).unwrap(); }
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
        for _ in 0..5 { advance_epoch(&mut s, &idle(4), &[]).unwrap_err(); }
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
        for _ in 0..3 { advance_epoch(&mut s, &idle(4), &[]).unwrap(); }
        let seed_is_nonzero = s.entropy_seed != [0u8; 32];
        let seed_hex: String = s.entropy_seed.iter().map(|b| format!("{:02x}", b)).collect();
        records.push(format!(
            r#"  {{"id":"TV-8","desc":"entropy seed evolves after 3 epochs","vc":4,"steps":[],"expect":{{"epoch":{},"halt":false,"halt_reason":0,"state_root":"{}","entropy_seed":"{}","seed_nonzero":{}}}}}"#,
            s.epoch, root_hex(&s.state_root), seed_hex, seed_is_nonzero
        ));
    }

    // TV-9: single validator, 2 idle epochs
    {
        let mut s = genesis(1);
        let steps: Vec<String> = (0..2).map(|_| emit_step_idle(1)).collect();
        for _ in 0..2 { advance_epoch(&mut s, &idle(1), &[]).unwrap(); }
        records.push(format!(
            r#"  {{"id":"TV-9","desc":"single validator, 2 idle epochs","vc":1,"steps":[{}],"expect":{{"epoch":{},"halt":false,"halt_reason":{},"state_root":"{}"}}}}"#,
            steps.join(","), s.epoch, hr(s.halt_reason), root_hex(&s.state_root)
        ));
    }

    println!("{{\n  \"generated_by\": \"gen_vectors.rs\",\n  \"spec\": \"proofs/model/Model.v\",\n  \"coq_theorems\": [\"TH-3a\",\"TH-3b\",\"TH-6\"],\n  \"vectors\": [\n{}\n  ]\n}}", records.join(",\n"));
}

use qash_consensus::transition::{encode_full_state_into, FULL_STATE_MAX_BYTES};

#[ignore]
#[test]
fn gen_replay_snapshots() {
    fn snap(label: &str, vc: u32, epochs: usize) {
        let mut s = genesis(vc);
        for _ in 0..epochs { advance_epoch(&mut s, &idle(vc), &[]).unwrap(); }
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
    use qash_consensus::transition::{advance_epoch, EpochState, EpochInput, HaltReason, MAX_VALIDATORS};
    use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
    use qash_consensus::derive::derive_leaf_index;
    
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
    };
    let input = EpochInput::new(4);
    advance_epoch(&mut state, &input, &[]).unwrap();
    eprintln!("4-validator epoch 1 root: {}", hex_encode(&state.state_root));
    
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
    };
    let input0 = EpochInput::new(0);
    let r0 = advance_epoch(&mut state0, &input0, &[]);
    eprintln!("0-validator epoch 1 result: {:?}", r0);
    if r0.is_ok() {
        eprintln!("0-validator epoch 1 root: {}", hex_encode(&state0.state_root));
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
    use qash_consensus::transition::{advance_epoch, EpochState, EpochInput, HaltReason, MAX_VALIDATORS};
    use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
    
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
    };
    for epoch in 1..=2u64 {
        let input = EpochInput::new(4);
        let r = advance_epoch(&mut state, &input, &[]).unwrap();
        let bytes: Vec<String> = state.state_root.iter().map(|b| format!("{:02x}", b)).collect();
        eprintln!("epoch {} root: {}", epoch, bytes.join(""));
        eprintln!("  lyapunov_raw: {}, phi_safety_raw: {}", r.lyapunov.v_convergence.raw(), r.lyapunov.phi_safety.raw());
    }
}
