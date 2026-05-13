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
    None              = 0x00,
    LyapunovViolation = 0x01, // H1: δ_window > ε
    ArithOverflow     = 0x02, // H2: i128 overflow
    EpochOverflow     = 0x03, // H3: epoch counter overflow
    DecodeInvalid     = 0x04, // H4: decode / admissibility failure
    RoundtripFailure  = 0x05, // H5: state root round-trip failure (reserved)
    HaltFlagSet       = 0x06, // H6: explicit external halt (reserved)
    PhiSafetyViolation= 0x07, // §5 Condition 2: Φ_safety ≥ Φ_max_safe (ADR-001)
}

#[derive(Debug, Clone, Copy)]
pub struct ValidatorUpdate {
    pub divergence_new: FixedPoint,
    pub conflict_new: FixedPoint,
    pub slash_accum_new: FixedPoint, // absolute, monotone; must fit in i64
}

pub struct EpochInput {
    pub updates: [Option<ValidatorUpdate>; MAX_VALIDATORS],
    pub update_count: u32,
    /// Count of cascade proof rejections in this epoch's input set (§4c, v1.1).
    /// Must be ≤ MAX_QUERIES_PER_EPOCH (validated in step_1_validate).
    pub cascade_fail_count: u32,
}

pub struct EpochState {
    pub epoch: u64,
    pub halt_reason: HaltReason,
    pub entropy_seed: [u8; 32],
    /// Current state root (commitment chain anchor).
    /// Genesis: [0u8; 32]. Updated at the end of every successful epoch.
    pub state_root: [u8; 32],
    /// Ledger (sparse Merkle accumulator) root. Stub [0u8; 32] until SM tree is implemented.
    pub ledger_root: [u8; 32],
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
            // First halt only — state is NOT mutated except for halt_reason.
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

    // §5 Condition 1: convergence gate
    if lyap.halt_triggered {
        return Err(TransitionHalt { reason: HaltReason::LyapunovViolation });
    }

    // §5 Condition 2: safety admissibility gate (ADR-001 accepted)
    if lyap.phi_safety.raw() >= lyapunov::PHI_MAX_SAFE {
        return Err(TransitionHalt { reason: HaltReason::PhiSafetyViolation });
    }

    let next_epoch = state.epoch.checked_add(1)
        .ok_or(TransitionHalt { reason: HaltReason::EpochOverflow })?;
    let next_entropy = h_domain(DomainTag::EntropyAdvance, &state.entropy_seed);

    // Capture prior root before any mutation (needed for commitment encoding).
    let prior_root = state.state_root;

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

    // Push current V_convergence only after passing checks (prevents mutation on halt).
    state.convergence_window.push(lyap.v_convergence);

    state.entropy_seed = next_entropy;
    state.epoch = next_epoch;

    // Compute new state root with prior-root substitution (ADR-003 accepted, §2).
    // Step 9 per spec: LAST operation in the commit phase.
    let root = compute_state_root(state, &prior_root);
    state.state_root = root;

    Ok(TransitionResult { state_root: root, lyapunov: lyap })
}

fn step_1_validate(state: &EpochState, input: &EpochInput) -> Result<(), TransitionHalt> {
    if state.validator_count > MAX_VALIDATORS_WIRE {
        return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
    }
    if input.update_count != state.validator_count {
        return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
    }

    // Admissibility: entropy_seed must not be all-zero post-genesis (§1 constraint 5).
    if state.epoch > 0 && state.entropy_seed == [0u8; 32] {
        return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
    }

    // Cascade fail count bounded by max_queries (§4c).
    if (input.cascade_fail_count as i128) > lyapunov::MAX_QUERIES_PER_EPOCH {
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

            // Monotonicity: slash accumulator never decreases.
            if u.slash_accum_new.raw() < state.validators[i].slash_accum.raw() {
                return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
            }

            // Bound slash to i64::MAX (wire format and Φ_safety arithmetic contract).
            if u.slash_accum_new.to_i64().is_err() {
                return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
            }
        }
    }

    // Trailing slots must be None.
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
    let mut phi_acc = FixedPoint::ZERO;

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

        // Φ_safety: sum of γ·Σ_i over all validators (ADR-001 accepted — sum, not max).
        let phi_term = lyapunov::WEIGHT_S.checked_mul(s)?;
        phi_acc = phi_acc.checked_add(phi_term)?;
    }

    // CH term: cascade health factor, epoch-level (§4c, v1.1).
    let ch_raw = (input.cascade_fail_count as i128) * SCALE
        / lyapunov::MAX_QUERIES_PER_EPOCH;
    let ch_term = lyapunov::WEIGHT_CH.checked_mul(FixedPoint::from_raw(ch_raw))?;
    v_sum = v_sum.checked_add(ch_term)?;

    let v_total = v_sum.checked_add(phi_acc)?;

    let (delta_window, halt_triggered) = if state.convergence_window.is_full() {
        let delta = lyapunov::compute_delta_window(v_sum, &state.convergence_window)?;
        (delta, delta.raw() > lyapunov::EPSILON.raw())
    } else {
        (FixedPoint::ZERO, false)
    };

    Ok(LyapunovEval {
        v_convergence: v_sum,
        phi_safety: phi_acc,
        v_total,
        delta_window,
        halt_triggered,
    })
}

/// Compute the state root using prior-root substitution (§2, ADR-003 accepted).
///
/// Encodes the full committed state with `state_root` field replaced by `prior_root`,
/// then hashes with H_domain(STATE_ROOT, ...). This binds each root to the preceding
/// root, creating a verifiable chain without storing chain history in consensus state.
fn compute_state_root(state: &EpochState, prior_root: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; encoding::MAX_COMMITMENT_PREIMAGE];

    // Extract validator i64 values.
    // Invariants guaranteed by step_1_validate:
    //   D, C ∈ [0, SCALE] ⊂ [0, 1_000_000] → fits in i64.
    //   slash_accum.to_i64() validated → fits in i64.
    let mut d_arr = [0i64; MAX_VALIDATORS];
    let mut c_arr = [0i64; MAX_VALIDATORS];
    let mut s_arr = [0i64; MAX_VALIDATORS];

    let vc = state.validator_count as usize;
    let mut i = 0usize;
    while i < vc {
        d_arr[i] = state.validators[i].divergence.raw() as i64;
        c_arr[i] = state.validators[i].conflict.raw() as i64;
        // Slash was validated to fit in i64; cast is safe.
        s_arr[i] = state.validators[i].slash_accum.raw() as i64;
        i += 1;
    }

    let (filled, wvals) = state.convergence_window.raw_parts();
    // V_convergence ∈ [0, ~666M] → fits in i64 (window stores these values).
    let w0 = wvals[0].raw() as i64;
    let w1 = wvals[1].raw() as i64;
    let w2 = wvals[2].raw() as i64;

    let n = encoding::encode_commitment_preimage(
        state.epoch,
        prior_root,
        &state.ledger_root,
        &state.entropy_seed,
        (state.halt_reason != HaltReason::None) as u8,
        state.validator_count,
        &d_arr[..vc],
        &c_arr[..vc],
        &s_arr[..vc],
        filled,
        w0, w1, w2,
        &mut buf,
    );

    h_domain(DomainTag::StateRoot, &buf[..n as usize])
}
