use std::env;
use std::fs;

use serde::{Deserialize, Serialize};

use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};

// ── Vector file format ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct VectorFile {
    vectors: Vec<TestVector>,
}

#[derive(Deserialize)]
struct TestVector {
    name: String,
    #[serde(default = "default_vc")]
    initial_validator_count: u32,
    epochs: Vec<EpochSpec>,
}

fn default_vc() -> u32 { 4 }

#[derive(Deserialize)]
struct PerSlotUpdate {
    slot: usize,
    divergence_new: i64,
    conflict_new: i64,
    slash_accum_new: i64,
}

#[derive(Deserialize)]
struct AllValidatorsUpdate {
    divergence_new: i64,
    conflict_new: i64,
    slash_accum_new: i64,
}

#[derive(Deserialize)]
struct EpochSpec {
    #[serde(default)]
    validator_updates: Vec<PerSlotUpdate>,
    all_validators_update: Option<AllValidatorsUpdate>,
    #[serde(default)]
    cascade_fail_count: u32,
    expected_state_root_hex: Option<String>,
    expected_halt_reason: Option<String>,
}

// ── Output format ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OutputFile {
    version: u32,
    results: Vec<VectorResult>,
}

#[derive(Serialize)]
struct VectorResult {
    name: String,
    pass: bool,
    epochs: Vec<EpochResult>,
}

#[derive(Serialize)]
struct EpochResult {
    epoch: u64,
    state_root_hex: String,
    halt_reason: String,
    pass: bool,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn halt_str(r: HaltReason) -> &'static str {
    match r {
        HaltReason::None               => "None",
        HaltReason::LyapunovViolation  => "LyapunovViolation",
        HaltReason::ArithOverflow      => "ArithOverflow",
        HaltReason::EpochOverflow      => "EpochOverflow",
        HaltReason::DecodeInvalid      => "DecodeInvalid",
        HaltReason::RoundtripFailure   => "RoundtripFailure",
        HaltReason::HaltFlagSet        => "HaltFlagSet",
        HaltReason::PhiSafetyViolation => "PhiSafetyViolation",
    }
}

fn fp(v: i64) -> FixedPoint { FixedPoint::from_raw(v as i128) }

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut vectors_path = "tests/vectors/vectors.v1.json".to_string();
    let mut out_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--vectors" => { vectors_path = args[i + 1].clone(); i += 2; }
            "--out"     => { out_path = Some(args[i + 1].clone()); i += 2; }
            _           => { i += 1; }
        }
    }

    let json = match fs::read_to_string(&vectors_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("ERROR: cannot read {vectors_path}: {e}"); std::process::exit(1); }
    };
    let vf: VectorFile = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(e) => { eprintln!("ERROR: JSON parse error: {e}"); std::process::exit(1); }
    };

    let mut all_pass = true;
    let mut output = OutputFile { version: 1, results: Vec::new() };

    for vec in &vf.vectors {
        let vc = vec.initial_validator_count;
        if vc as usize > MAX_VALIDATORS {
            eprintln!("ERROR: vector '{}': initial_validator_count {vc} > MAX_VALIDATORS {MAX_VALIDATORS}", vec.name);
            std::process::exit(1);
        }

        let mut state = EpochState {
            epoch: 0,
            halt_reason: HaltReason::None,
            entropy_seed: [0u8; 32],
            cascade_health: 0,
            state_root: [0u8; 32],
            ledger_root: [0u8; 32],
            validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
            validator_count: vc,
            convergence_window: ConvergenceWindow::new(),
        };

        let mut epoch_results = Vec::new();
        let mut vec_pass = true;

        for (idx, spec) in vec.epochs.iter().enumerate() {
            let mut updates = [None::<ValidatorUpdate>; MAX_VALIDATORS];

            for u in &spec.validator_updates {
                if u.slot >= vc as usize {
                    eprintln!("ERROR: vector '{}' epoch {idx}: slot {} >= validator_count {vc}", vec.name, u.slot);
                    std::process::exit(1);
                }
                updates[u.slot] = Some(ValidatorUpdate {
                    divergence_new:       fp(u.divergence_new),
                    conflict_new:         fp(u.conflict_new),
                    slash_accum_new:      fp(u.slash_accum_new),
                    signature_health_new: FixedPoint::ZERO,
                    blinding_health_new:  FixedPoint::ZERO,
                });
            }

            if let Some(ref all) = spec.all_validators_update {
                for slot in updates.iter_mut().take(vc as usize) {
                    if slot.is_none() {
                        *slot = Some(ValidatorUpdate {
                            divergence_new:       fp(all.divergence_new),
                            conflict_new:         fp(all.conflict_new),
                            slash_accum_new:      fp(all.slash_accum_new),
                            signature_health_new: FixedPoint::ZERO,
                            blinding_health_new:  FixedPoint::ZERO,
                        });
                    }
                }
            }

            let input = EpochInput { updates, update_count: vc, cascade_fail_count: spec.cascade_fail_count };

            advance_epoch(&mut state, &input).ok();

            let root_hex  = hex::encode(state.state_root);
            let halt_name = halt_str(state.halt_reason).to_string();

            let mut ok = true;

            if let Some(ref exp) = spec.expected_state_root_hex {
                if exp != "TBD" && exp != &root_hex {
                    eprintln!("FAIL  vector '{}' epoch {idx}: state_root\n  expected {exp}\n  actual   {root_hex}", vec.name);
                    ok = false;
                }
            }
            if let Some(ref exp) = spec.expected_halt_reason {
                if exp != "TBD" && exp != &halt_name {
                    eprintln!("FAIL  vector '{}' epoch {idx}: halt_reason\n  expected {exp}\n  actual   {halt_name}", vec.name);
                    ok = false;
                }
            }

            if ok {
                eprintln!("pass  vector '{}' epoch {idx}: root={}... halt={halt_name}", vec.name, &root_hex[..8]);
            } else {
                vec_pass = false;
                all_pass = false;
            }

            epoch_results.push(EpochResult {
                epoch: idx as u64,
                state_root_hex: root_hex,
                halt_reason: halt_name,
                pass: ok,
            });
        }

        output.results.push(VectorResult { name: vec.name.clone(), pass: vec_pass, epochs: epoch_results });
    }

    let out_json = serde_json::to_string_pretty(&output).expect("serialize output");

    match out_path {
        Some(ref p) => fs::write(p, &out_json).unwrap_or_else(|e| { eprintln!("ERROR: write {p}: {e}"); std::process::exit(1); }),
        None        => println!("{out_json}"),
    }

    if !all_pass {
        std::process::exit(1);
    }
}
