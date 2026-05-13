//! Epoch transition (atomic, infallible commit phase).

use crate::encoding;
use crate::fixed_point::{FixedPoint, OverflowError, SCALE};
use crate::hash::{h_domain, DomainTag};
use crate::lyapunov::{self, ConvergenceWindow, LyapunovEval, ValidatorMetrics, LyapunovError};

/// Protocol-facing limit (u32 per Domain A rules). Used in wire validation.
pub const MAX_VALIDATORS_WIRE: u32 = 1024;
/// Array-sizing alias (usize is required by Rust array syntax; not stored in state).
pub const MAX_VALIDATORS: usize = MAX_VALIDATORS_WIRE as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HaltReason {
    None                = 0x00,
    LyapunovViolation   = 0x01, // H1: δ_window > ε
    ArithOverflow       = 0x02, // H2
    EpochOverflow       = 0x03, // H3
    DecodeInvalid       = 0x04, // H4
    RoundtripFailure    = 0x05, // H5 (reserved)
    HaltFlagSet         = 0x06, // H6 (reserved)
    PhiSafetyViolation  = 0x07, // H7: Φ_safety ≥ PHI_MAX_SAFE (ADR-001)
}

#[derive(Debug, Clone, Copy)]
pub struct ValidatorUpdate {
    pub divergence_new: FixedPoint,
    pub conflict_new: FixedPoint,
    pub slash_accum_new: FixedPoint, // absolute, monotone
}

pub struct EpochInput {
    pub updates: [Option<ValidatorUpdate>; MAX_VALIDATORS],
    pub update_count: u32,
}

pub struct EpochState {
    pub epoch: u64,
    pub halt_reason: HaltReason,
    pub entropy_seed: [u8; 32],
    pub validators: [ValidatorMetrics; MAX_VALIDATORS],
    pub validator_count: u32,
    pub convergence_window: ConvergenceWindow,
}

impl EpochState {
    #[inline]
    pub fn is_halted(&self) -> bool {
        self.halt_reason != HaltReason::None
    }
}

#[derive(Debug)]
struct TransitionHalt {
    reason: HaltReason,
}

impl From<OverflowError> for TransitionHalt {
    fn from(_: OverflowError) -> Self {
        TransitionHalt { reason: HaltReason::ArithOverflow }
    }
}

impl From<LyapunovError> for TransitionHalt {
    fn from(e: LyapunovError) -> Self {
        match e {
            LyapunovError::Overflow => TransitionHalt { reason: HaltReason::ArithOverflow },
            LyapunovError::UnboundedMetric => TransitionHalt { reason: HaltReason::DecodeInvalid },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TransitionResult {
    pub state_root: [u8; 32],
    pub lyapunov: LyapunovEval,
}

pub fn advance_epoch(state: &mut EpochState, input: &EpochInput) -> Result<TransitionResult, HaltReason> {
    // Absorbing: if already halted, return the original reason; do not mutate.
    if state.is_halted() {
        return Err(state.halt_reason);
    }

    match run_pipeline(state, input) {
        Ok(r) => Ok(r),
        Err(h) => {
            // First halt only.
            state.halt_reason = h.reason;
            Err(h.reason)
        }
    }
}

fn run_pipeline(state: &mut EpochState, input: &EpochInput) -> Result<TransitionResult, TransitionHalt> {
    // ┌──────────────────────────────────────────────────┐
    // │ PRE-COMMIT PHASE: state is READ-ONLY             │
    // │ Any error returns without mutation.              │
    // └──────────────────────────────────────────────────┘

    step_1_validate(state, input)?;

    let lyap = evaluate_projected(state, input)?;
    if lyap.halt_triggered {
        return Err(TransitionHalt { reason: HaltReason::LyapunovViolation });
    }
    if lyap.phi_halt_triggered {
        return Err(TransitionHalt { reason: HaltReason::PhiSafetyViolation });
    }

    let next_epoch = state.epoch.checked_add(1).ok_or(TransitionHalt { reason: HaltReason::EpochOverflow })?;
    let next_entropy = h_domain(DomainTag::EntropyAdvance, &state.entropy_seed);

    // ╔══════════════════════════════════════════════════╗
    // ║ COMMIT POINT                                    ║
    // ║ Below: assignments only. No `?`. No checked ops. ║
    // ╚══════════════════════════════════════════════════╝

    for i in 0..state.validator_count as usize {
        if let Some(ref u) = input.updates[i] {
            state.validators[i].divergence = u.divergence_new;
            state.validators[i].conflict = u.conflict_new;
            state.validators[i].slash_accum = u.slash_accum_new;
        }
    }

    // check-then-push: push current V_convergence only after passing checks
    state.convergence_window.push(lyap.v_convergence);

    state.entropy_seed = next_entropy;
    state.epoch = next_epoch;

    let root = state_root_header_only(state);

    Ok(TransitionResult { state_root: root, lyapunov: lyap })
}

fn step_1_validate(state: &EpochState, input: &EpochInput) -> Result<(), TransitionHalt> {
    if state.validator_count > MAX_VALIDATORS_WIRE {
        return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
    }
    if input.update_count != state.validator_count {
        return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
    }

    let scale_raw = SCALE;

    for i in 0..state.validator_count as usize {
        if let Some(ref u) = input.updates[i] {
            let d = u.divergence_new.raw();
            let c = u.conflict_new.raw();

            if d < 0 || d > scale_raw || c < 0 || c > scale_raw {
                return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
            }

            if !u.slash_accum_new.is_non_negative() {
                return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
            }

            if u.slash_accum_new.raw() < state.validators[i].slash_accum.raw() {
                return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
            }

            // Implementation/serialization-domain bound (policy): keep Σ within i64.
            if u.slash_accum_new.to_i64().is_err() {
                return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
            }
        }
    }

    // trailing slots must be None
    for i in state.validator_count as usize..MAX_VALIDATORS {
        if input.updates[i].is_some() {
            return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
        }
    }

    Ok(())
}

/// Evaluate Lyapunov over the *effective* state (projected view) with zero allocation.
fn evaluate_projected(state: &EpochState, input: &EpochInput) -> Result<LyapunovEval, TransitionHalt> {
    let mut v_sum = FixedPoint::ZERO;
    let mut sum_slash = FixedPoint::ZERO;

    for i in 0..state.validator_count as usize {
        let (d, c, s) = match &input.updates[i] {
            Some(u) => (u.divergence_new, u.conflict_new, u.slash_accum_new),
            None => (
                state.validators[i].divergence,
                state.validators[i].conflict,
                state.validators[i].slash_accum,
            ),
        };

        let term_d = lyapunov::WEIGHT_D.checked_mul(d)?;
        let term_c = lyapunov::WEIGHT_C.checked_mul(c)?;
        let term = term_d.checked_add(term_c)?;
        v_sum = v_sum.checked_add(term)?;
        sum_slash = sum_slash.checked_add(s)?;
    }

    let phi = lyapunov::WEIGHT_S.checked_mul(sum_slash)?;
    let v_total = v_sum.checked_add(phi)?;

    // δ_window is checked against V_convergence, NOT V_total.
    let (delta_window, halt_triggered) = if state.convergence_window.is_full() {
        let delta = lyapunov::compute_delta_window(v_sum, &state.convergence_window)?;
        (delta, delta.raw() > lyapunov::EPSILON.raw())
    } else {
        (FixedPoint::ZERO, false)
    };

    let phi_halt_triggered = phi.raw() >= lyapunov::PHI_MAX_SAFE.raw();

    Ok(LyapunovEval {
        v_convergence: v_sum,
        phi_safety: phi,
        v_total,
        delta_window,
        halt_triggered,
        phi_halt_triggered,
    })
}

fn state_root_header_only(state: &EpochState) -> [u8; 32] {
    let mut header = [0u8; encoding::STATE_HEADER_SIZE as usize];
    encoding::encode_state_header(
        state.epoch,
        state.validator_count,
        state.halt_reason as u8,
        &state.entropy_seed,
        &mut header,
    );
    h_domain(DomainTag::StateRoot, &header)
}
