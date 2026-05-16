use std::fs;
use std::path::Path;

use serde::Deserialize;

use qash_consensus::fixed_point::{FixedPoint, SCALE};
use qash_consensus::hash::{h_domain, DomainTag};
use qash_consensus::lyapunov::{self, ConvergenceWindow, ValidatorMetrics};
use qash_consensus::transition::{
    advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
};
use sha3::{Digest, Sha3_256};

#[derive(Debug, Deserialize)]
struct ReplayVector {
    name: String,
    validator_count: u32,
    steps: Vec<StepVector>,
}

#[derive(Debug, Deserialize)]
struct StepVector {
    updates: Vec<UpdateVector>,
    expected_halt_reason: String,
}

#[derive(Debug, Deserialize)]
struct UpdateVector {
    divergence_raw: i64,
    conflict_raw: i64,
    slash_accum_raw: i64,
}

#[derive(Clone)]
struct ContractModelState {
    runtime: EpochState,
}

impl ContractModelState {
    fn new(validator_count: u32) -> Self {
        Self {
            runtime: EpochState {
                epoch: 0,
                halt_reason: HaltReason::None,
                entropy_seed: [0u8; 32],
                validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
                validator_count,
                convergence_window: ConvergenceWindow::new(),
            },
        }
    }

    fn apply(&mut self, input: &EpochInput) -> Result<[u8; 32], HaltReason> {
        if self.runtime.halt_reason != HaltReason::None {
            return Err(self.runtime.halt_reason);
        }

        if self.runtime.validator_count as usize > MAX_VALIDATORS || input.update_count != self.runtime.validator_count {
            self.runtime.halt_reason = HaltReason::DecodeInvalid;
            return Err(HaltReason::DecodeInvalid);
        }

        let mut projected = self.runtime.validators;
        for i in 0..self.runtime.validator_count as usize {
            if let Some(update) = input.updates[i] {
                if update.divergence_new.raw() < 0
                    || update.divergence_new.raw() > SCALE
                    || update.conflict_new.raw() < 0
                    || update.conflict_new.raw() > SCALE
                    || !update.slash_accum_new.is_non_negative()
                    || update.slash_accum_new.raw() < self.runtime.validators[i].slash_accum.raw()
                    || update.slash_accum_new.to_i64().is_err()
                {
                    self.runtime.halt_reason = HaltReason::DecodeInvalid;
                    return Err(HaltReason::DecodeInvalid);
                }

                projected[i] = ValidatorMetrics {
                    divergence: update.divergence_new,
                    conflict: update.conflict_new,
                    slash_accum: update.slash_accum_new,
                };
            }
        }

        for i in self.runtime.validator_count as usize..MAX_VALIDATORS {
            if input.updates[i].is_some() {
                self.runtime.halt_reason = HaltReason::DecodeInvalid;
                return Err(HaltReason::DecodeInvalid);
            }
        }

        let lyap = match lyapunov::evaluate(
            &projected[..self.runtime.validator_count as usize],
            &self.runtime.convergence_window,
        ) {
            Ok(eval) => eval,
            Err(lyapunov::LyapunovError::Overflow) => {
                self.runtime.halt_reason = HaltReason::ArithOverflow;
                return Err(HaltReason::ArithOverflow);
            }
            Err(lyapunov::LyapunovError::UnboundedMetric) => {
                self.runtime.halt_reason = HaltReason::DecodeInvalid;
                return Err(HaltReason::DecodeInvalid);
            }
        };

        if lyap.halt_triggered {
            self.runtime.halt_reason = HaltReason::LyapunovViolation;
            return Err(HaltReason::LyapunovViolation);
        }

        let next_epoch = match self.runtime.epoch.checked_add(1) {
            Some(v) => v,
            None => {
                self.runtime.halt_reason = HaltReason::EpochOverflow;
                return Err(HaltReason::EpochOverflow);
            }
        };

        self.runtime.validators = projected;
        self.runtime.convergence_window.push(lyap.v_convergence);
        self.runtime.entropy_seed = h_domain(DomainTag::EntropyAdvance, &self.runtime.entropy_seed);
        self.runtime.epoch = next_epoch;

        Ok(state_fingerprint(&self.runtime))
    }
}

fn parse_halt_reason(s: &str) -> HaltReason {
    match s {
        "None" => HaltReason::None,
        "LyapunovViolation" => HaltReason::LyapunovViolation,
        "ArithOverflow" => HaltReason::ArithOverflow,
        "EpochOverflow" => HaltReason::EpochOverflow,
        "DecodeInvalid" => HaltReason::DecodeInvalid,
        "RoundtripFailure" => HaltReason::RoundtripFailure,
        "HaltFlagSet" => HaltReason::HaltFlagSet,
        other => panic!("unknown halt reason: {other}"),
    }
}

fn state_fingerprint(state: &EpochState) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update((DomainTag::InternalHash as u32).to_le_bytes());
    hasher.update(state.epoch.to_le_bytes());
    hasher.update(state.validator_count.to_le_bytes());
    hasher.update([state.halt_reason as u8, 0x00, 0x00, 0x00]);
    hasher.update(state.entropy_seed);

    for i in 0..state.validator_count as usize {
        let v = &state.validators[i];
        hasher.update(v.divergence.raw().to_le_bytes());
        hasher.update(v.conflict.raw().to_le_bytes());
        hasher.update(v.slash_accum.raw().to_le_bytes());
    }

    let (filled, values) = state.convergence_window.raw_parts();
    hasher.update([filled, 0x00, 0x00, 0x00]);
    for v in values.iter() {
        hasher.update(v.raw().to_le_bytes());
    }

    let out = hasher.finalize();
    let mut res = [0u8; 32];
    res.copy_from_slice(&out);
    res
}

fn build_input(step: &StepVector, validator_count: u32) -> EpochInput {
    assert_eq!(step.updates.len(), validator_count as usize);
    let mut input = EpochInput {
        updates: [None; MAX_VALIDATORS],
        update_count: validator_count,
    };

    for (i, u) in step.updates.iter().enumerate() {
        input.updates[i] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(u.divergence_raw),
            conflict_new: FixedPoint::from_raw(u.conflict_raw),
            slash_accum_new: FixedPoint::from_raw(u.slash_accum_raw),
        });
    }

    input
}

#[test]
fn replay_vectors_match_contract_model_and_runtime() {
    let root = Path::new("tests/vectors/replay");
    let entries = fs::read_dir(root).expect("failed to read replay vector directory");

    let mut any = false;
    for entry in entries {
        let path = entry.expect("invalid entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        any = true;

        let raw = fs::read_to_string(&path).expect("failed to read vector file");
        let vector: ReplayVector = serde_json::from_str(&raw).expect("invalid vector JSON");

        let mut runtime_state = EpochState {
            epoch: 0,
            halt_reason: HaltReason::None,
            entropy_seed: [0u8; 32],
            validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
            validator_count: vector.validator_count,
            convergence_window: ConvergenceWindow::new(),
        };
        let mut model_state = ContractModelState::new(vector.validator_count);

        for (idx, step) in vector.steps.iter().enumerate() {
            let input = build_input(step, vector.validator_count);
            let expected_halt = parse_halt_reason(&step.expected_halt_reason);

            let runtime_res = advance_epoch(&mut runtime_state, &input).map(|_| state_fingerprint(&runtime_state));
            let model_res = model_state.apply(&input);

            match (runtime_res, model_res, expected_halt) {
                (Ok(runtime_fp), Ok(model_fp), HaltReason::None) => {
                    assert_eq!(runtime_fp, model_fp, "{} step {} fingerprint mismatch", vector.name, idx);
                    assert_eq!(runtime_state.halt_reason, HaltReason::None);
                    assert_eq!(model_state.runtime.halt_reason, HaltReason::None);
                }
                (Err(runtime_h), Err(model_h), expected) => {
                    assert_eq!(runtime_h, expected, "{} step {} runtime halt mismatch", vector.name, idx);
                    assert_eq!(model_h, expected, "{} step {} model halt mismatch", vector.name, idx);
                    assert_eq!(runtime_state.halt_reason, expected);
                    assert_eq!(model_state.runtime.halt_reason, expected);
                    assert_eq!(
                        state_fingerprint(&runtime_state),
                        state_fingerprint(&model_state.runtime),
                        "{} step {} halted-state fingerprint mismatch",
                        vector.name,
                        idx
                    );
                }
                (a, b, expected) => panic!(
                    "{} step {} expected {:?}, got runtime={:?}, model={:?}",
                    vector.name, idx, expected, a, b
                ),
            }

            assert_eq!(runtime_state.epoch, model_state.runtime.epoch);
            assert_eq!(
                runtime_state.convergence_window.raw_parts(),
                model_state.runtime.convergence_window.raw_parts()
            );
        }
    }

    assert!(any, "no replay vectors found");
}
