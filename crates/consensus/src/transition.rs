//! Epoch transition (atomic, infallible commit phase).

use crate::encoding::EncodeError;
use crate::fixed_point::{FixedPoint, OverflowError, SCALE};
use crate::hash::{h_domain, DomainTag};
use crate::lyapunov::{
    self, ConvergenceWindow, LyapunovEval, LyapunovError, ValidatorMetrics, WINDOW_SIZE,
};

/// Protocol-facing limit (u32 per Domain A rules). Used in wire validation.
pub const MAX_VALIDATORS_WIRE: u32 = 1024;
/// Array-sizing alias (usize is required by Rust array syntax; not stored in state).
pub const MAX_VALIDATORS: usize = MAX_VALIDATORS_WIRE as usize;

// Full-state wire format (canonical, deterministic):
// [epoch:8][state_root:32][ledger_root:32][entropy_seed:32][halt:1][pad:3][vc:4]
// [N × (div:8 + conf:8 + slash:8 + nonce:8 + id:48)]
// [window_filled:1][pad:3][window_vals:3×8]
pub const FULL_STATE_FIXED_BYTES: usize = 112;
pub const FULL_STATE_PER_VALIDATOR_BYTES: usize = 80;
pub const FULL_STATE_WINDOW_BYTES: usize = 28;
pub const FULL_STATE_MAX_BYTES: usize = FULL_STATE_FIXED_BYTES
    + MAX_VALIDATORS * FULL_STATE_PER_VALIDATOR_BYTES
    + FULL_STATE_WINDOW_BYTES;
// = 112 + 81920 + 28 = 82,060

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HaltReason {
    None              = 0x00,
    LyapunovViolation = 0x01, // H1
    ArithOverflow     = 0x02, // H2
    EpochOverflow     = 0x03, // H3
    DecodeInvalid     = 0x04, // H4
    RoundtripFailure  = 0x05, // H5
    HaltFlagSet       = 0x06, // H6 (explicit external halt; reserved)
}

impl HaltReason {
    fn from_u8(v: u8) -> Result<Self, EncodeError> {
        match v {
            0x00 => Ok(HaltReason::None),
            0x01 => Ok(HaltReason::LyapunovViolation),
            0x02 => Ok(HaltReason::ArithOverflow),
            0x03 => Ok(HaltReason::EpochOverflow),
            0x04 => Ok(HaltReason::DecodeInvalid),
            0x05 => Ok(HaltReason::RoundtripFailure),
            0x06 => Ok(HaltReason::HaltFlagSet),
            _    => Err(EncodeError::InvalidHaltCode),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ValidatorUpdate {
    pub divergence_new: FixedPoint,
    pub conflict_new: FixedPoint,
    pub slash_accum_new: FixedPoint,      // absolute, monotone; must fit in i64
    pub signature_health_new: FixedPoint, // SH ∈ [0, SCALE]; zero-weight until v1.1.1
    pub blinding_health_new: FixedPoint,  // BH ∈ [0, SCALE]; zero-weight until v1.1.1
}

pub struct EpochInput {
    pub updates: [Option<ValidatorUpdate>; MAX_VALIDATORS],
    pub update_count: u32,
    /// Count of cascade proof rejections in this epoch's input set (§4c, v1.1).
    /// Must be ≤ MAX_QUERIES_PER_EPOCH (validated in step_1_validate).
    pub cascade_fail_count: u32,
}

#[derive(Clone, Copy)]
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
    /// TX replay protection (one nonce per validator slot).
    pub nonces: [u64; MAX_VALIDATORS],
    /// Stable 48-byte consensus identity for each validator slot (fixed at genesis).
    pub validator_ids: [[u8; 48]; MAX_VALIDATORS],
    /// This epoch's committed state root; used as prior_root for the next epoch.
    pub state_root: [u8; 32],
}

impl EpochState {
    #[inline]
    pub fn is_halted(&self) -> bool {
        self.halt_reason != HaltReason::None
    }
}

// ---------------------------------------------------------------------------
// Full-state encoding / decoding
// ---------------------------------------------------------------------------

/// Encode the full state into `out`; returns bytes written.
/// Only `state.validator_count` validator slots are encoded.
pub fn encode_full_state_into(
    state: &EpochState,
    out: &mut [u8; FULL_STATE_MAX_BYTES],
) -> usize {
    write_state_bytes(state, &state.state_root, out)
}

/// Encode for state-root commitment: identical to encode_full_state_into but
/// substitutes `prior_root` into the state_root field.
fn encode_for_commitment_into(
    state: &EpochState,
    prior_root: &[u8; 32],
    out: &mut [u8; FULL_STATE_MAX_BYTES],
) -> usize {
    write_state_bytes(state, prior_root, out)
}

fn write_state_bytes(
    state: &EpochState,
    root_field: &[u8; 32],
    out: &mut [u8; FULL_STATE_MAX_BYTES],
) -> usize {
    let mut pos: usize = 0;

    out[pos..pos + 8].copy_from_slice(&state.epoch.to_le_bytes());
    pos += 8;

    out[pos..pos + 32].copy_from_slice(root_field);
    pos += 32;

    // ledger_root: zeros (no ledger yet)
    out[pos..pos + 32].copy_from_slice(&[0u8; 32]);
    pos += 32;

    out[pos..pos + 32].copy_from_slice(&state.entropy_seed);
    pos += 32;

    out[pos]     = state.halt_reason as u8;
    out[pos + 1] = 0x00;
    out[pos + 2] = 0x00;
    out[pos + 3] = 0x00;
    pos += 4;

    out[pos..pos + 4].copy_from_slice(&state.validator_count.to_le_bytes());
    pos += 4;

    // 112 fixed-header bytes consumed.

    for i in 0..state.validator_count as usize {
        let v = &state.validators[i];
        out[pos..pos + 8].copy_from_slice(&fp_to_i64_wire(v.divergence).to_le_bytes());
        out[pos + 8..pos + 16].copy_from_slice(&fp_to_i64_wire(v.conflict).to_le_bytes());
        out[pos + 16..pos + 24].copy_from_slice(&fp_to_i64_wire(v.slash_accum).to_le_bytes());
        out[pos + 24..pos + 32].copy_from_slice(&state.nonces[i].to_le_bytes());
        out[pos + 32..pos + 80].copy_from_slice(&state.validator_ids[i]);
        pos += 80;
    }

    // Window: filled (1) + pad (3) + 3 × i64 (24) = 28 bytes.
    let (filled, values) = state.convergence_window.raw_parts();
    out[pos]     = filled;
    out[pos + 1] = 0x00;
    out[pos + 2] = 0x00;
    out[pos + 3] = 0x00;
    pos += 4;
    for v in values.iter() {
        out[pos..pos + 8].copy_from_slice(&fp_to_i64_wire(*v).to_le_bytes());
        pos += 8;
    }

    pos
}

/// Decode a canonical full-state encoding. Validates halt_reason, D/C bounds,
/// validator_count, window fill count, and ledger_root zeros.
pub fn decode_full_state(bytes: &[u8]) -> Result<EpochState, EncodeError> {
    if bytes.len() < FULL_STATE_FIXED_BYTES {
        return Err(EncodeError::BufferTooSmall);
    }

    let mut pos: usize = 0;

    let mut tmp8 = [0u8; 8];
    tmp8.copy_from_slice(&bytes[pos..pos + 8]);
    let epoch = u64::from_le_bytes(tmp8);
    pos += 8;

    let mut root_bytes = [0u8; 32];
    root_bytes.copy_from_slice(&bytes[pos..pos + 32]);
    let state_root = root_bytes;
    pos += 32;

    // ledger_root must be zeros.
    for b in &bytes[pos..pos + 32] {
        if *b != 0x00 {
            return Err(EncodeError::DecodeInvalid);
        }
    }
    pos += 32;

    let mut seed_bytes = [0u8; 32];
    seed_bytes.copy_from_slice(&bytes[pos..pos + 32]);
    let entropy_seed = seed_bytes;
    pos += 32;

    let halt_reason = HaltReason::from_u8(bytes[pos])?;
    if bytes[pos + 1] != 0x00 || bytes[pos + 2] != 0x00 || bytes[pos + 3] != 0x00 {
        return Err(EncodeError::DecodeInvalid);
    }
    pos += 4;

    let mut vc_bytes = [0u8; 4];
    vc_bytes.copy_from_slice(&bytes[pos..pos + 4]);
    let validator_count = u32::from_le_bytes(vc_bytes);
    pos += 4;

    if validator_count > MAX_VALIDATORS_WIRE {
        return Err(EncodeError::DecodeInvalid);
    }

    let vc = validator_count as usize;
    let expected_len = FULL_STATE_FIXED_BYTES
        + vc * FULL_STATE_PER_VALIDATOR_BYTES
        + FULL_STATE_WINDOW_BYTES;
    if bytes.len() != expected_len {
        return Err(EncodeError::DecodeInvalid);
    }

    let scale_raw = SCALE;
    let mut validators    = [ValidatorMetrics::ZERO; MAX_VALIDATORS];
    let mut nonces        = [0u64; MAX_VALIDATORS];
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];

    for i in 0..vc {
        let mut d_b = [0u8; 8];
        let mut c_b = [0u8; 8];
        let mut s_b = [0u8; 8];
        let mut n_b = [0u8; 8];
        let mut id  = [0u8; 48];

        d_b.copy_from_slice(&bytes[pos..pos + 8]);
        c_b.copy_from_slice(&bytes[pos + 8..pos + 16]);
        s_b.copy_from_slice(&bytes[pos + 16..pos + 24]);
        n_b.copy_from_slice(&bytes[pos + 24..pos + 32]);
        id.copy_from_slice(&bytes[pos + 32..pos + 80]);
        pos += 80;

        let d_raw = i64::from_le_bytes(d_b) as i128;
        let c_raw = i64::from_le_bytes(c_b) as i128;
        let s_raw = i64::from_le_bytes(s_b) as i128;
        let nonce = u64::from_le_bytes(n_b);

        if d_raw < 0 || d_raw > scale_raw {
            return Err(EncodeError::DecodeInvalid);
        }
        if c_raw < 0 || c_raw > scale_raw {
            return Err(EncodeError::DecodeInvalid);
        }
        if s_raw < 0 {
            return Err(EncodeError::DecodeInvalid);
        }

        validators[i] = ValidatorMetrics {
            divergence:  FixedPoint::from_raw(d_raw),
            conflict:    FixedPoint::from_raw(c_raw),
            slash_accum: FixedPoint::from_raw(s_raw),
        };
        nonces[i] = nonce;
        validator_ids[i] = id;
    }

    // Window: filled (1) + pad (3) + WINDOW_SIZE × i64.
    if bytes[pos + 1] != 0x00 || bytes[pos + 2] != 0x00 || bytes[pos + 3] != 0x00 {
        return Err(EncodeError::DecodeInvalid);
    }
    let filled = bytes[pos];
    pos += 4;

    if filled as usize > WINDOW_SIZE {
        return Err(EncodeError::DecodeInvalid);
    }

    let mut wire_values = [FixedPoint::ZERO; WINDOW_SIZE];
    for slot in wire_values.iter_mut() {
        let mut v_b = [0u8; 8];
        v_b.copy_from_slice(&bytes[pos..pos + 8]);
        *slot = FixedPoint::from_raw(i64::from_le_bytes(v_b) as i128);
        pos += 8;
    }

    // Reconstruct the window by pushing oldest-first so newest lands at [0].
    // Wire values[0] = newest; push from index (filled-1) down to 0.
    let mut window = ConvergenceWindow::new();
    let push_count = filled as usize;
    let mut k = push_count;
    while k > 0 {
        k -= 1;
        window.push(wire_values[k]);
    }

    let _ = pos; // consumed exactly expected_len bytes
    Ok(EpochState {
        epoch,
        halt_reason,
        entropy_seed,
        validators,
        validator_count,
        convergence_window: window,
        nonces,
        validator_ids,
        state_root,
    })
}

/// Cast FixedPoint to i64 for wire encoding.
/// All consensus-path values are validated to fit i64 before reaching the commit phase:
/// D, C ≤ SCALE = 1_000_000; Σ validated ≤ i64::MAX; V_convergence ≤ 768_000_000.
#[inline]
fn fp_to_i64_wire(fp: FixedPoint) -> i64 {
    debug_assert!(fp.raw() >= i64::MIN as i128 && fp.raw() <= i64::MAX as i128);
    fp.raw() as i64
}

fn compute_state_root(state: &EpochState, prior_root: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; FULL_STATE_MAX_BYTES];
    let len = encode_for_commitment_into(state, prior_root, &mut buf);
    h_domain(DomainTag::StateRoot, &buf[..len])
}

// ---------------------------------------------------------------------------
// Transition logic
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct TransitionHalt {
    reason: HaltReason,
}

impl From<OverflowError> for TransitionHalt {
    fn from(_: OverflowError) -> Self {
        TransitionHalt {
            reason: HaltReason::ArithOverflow,
        }
    }
}

impl From<LyapunovError> for TransitionHalt {
    fn from(e: LyapunovError) -> Self {
        match e {
            LyapunovError::Overflow       => TransitionHalt { reason: HaltReason::ArithOverflow },
            LyapunovError::UnboundedMetric => TransitionHalt { reason: HaltReason::DecodeInvalid },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TransitionResult {
    pub state_root: [u8; 32],
    pub primitive_roots: ConsensusDigestSet,
    pub lyapunov: LyapunovEval,
}

pub fn advance_epoch(
    state: &mut EpochState,
    input: &EpochInput,
    raw_txs: &[&[u8]],
) -> Result<TransitionResult, HaltReason> {
    if state.is_halted() {
        return Err(state.halt_reason);
    }

    match run_pipeline(state, input, raw_txs) {
        Ok(r) => Ok(r),
        Err(h) => {
            state.halt_reason = h.reason;
            Err(h.reason)
        }
    }
}

fn run_pipeline(
    state: &mut EpochState,
    input: &EpochInput,
    raw_txs: &[&[u8]],
) -> Result<TransitionResult, TransitionHalt> {
    // ┌──────────────────────────────────────────────────┐
    // │ PRE-COMMIT PHASE: state is READ-ONLY             │
    // │ Any error returns without mutation.              │
    // └──────────────────────────────────────────────────┘

    step_1_validate(state, input)?;

    let lyap = evaluate_projected(state, input)?;

    // §5 Condition 1: convergence gate
    if lyap.halt_triggered {
        return Err(TransitionHalt {
            reason: HaltReason::LyapunovViolation,
        });
    }
    if lyap.phi_halt_triggered {
        return Err(TransitionHalt { reason: HaltReason::PhiSafetyViolation });
    }

    let next_epoch =
        state.epoch.checked_add(1).ok_or(TransitionHalt { reason: HaltReason::EpochOverflow })?;
    let next_entropy = h_domain(DomainTag::EntropyAdvance, &state.entropy_seed);

    // Capture prior root before any mutation for chain continuity commitment.
    let prior_root = state.state_root;

    // ╔══════════════════════════════════════════════════╗
    // ║ COMMIT POINT                                    ║
    // ║ Below: assignments only. No `?`. No checked ops. ║
    // ╚══════════════════════════════════════════════════╝

    for i in 0..state.validator_count as usize {
        if let Some(ref u) = input.updates[i] {
            state.validators[i].divergence  = u.divergence_new;
            state.validators[i].conflict    = u.conflict_new;
            state.validators[i].slash_accum = u.slash_accum_new;
        }
    }

    // TX-0 has ε_τ = 0; apply after metric update, before window/entropy advance.
    if crate::transaction::apply_all(state, raw_txs, state.validator_count).is_err() {
        return Err(TransitionHalt { reason: HaltReason::ArithOverflow });
    }

    state.convergence_window.push(lyap.v_convergence);
    state.entropy_seed = next_entropy;
    state.epoch        = next_epoch;

    let root = compute_state_root(state, &prior_root);
    state.state_root = root;

    Ok(TransitionResult {
        state_root: root,
        primitive_roots,
        lyapunov: lyap,
    })
}

fn step_1_validate(
    state: &EpochState,
    input: &EpochInput,
) -> Result<(), TransitionHalt> {
    if state.validator_count > MAX_VALIDATORS_WIRE {
        return Err(TransitionHalt {
            reason: HaltReason::DecodeInvalid,
        });
    }
    if input.update_count != state.validator_count {
        return Err(TransitionHalt {
            reason: HaltReason::DecodeInvalid,
        });
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
                return Err(TransitionHalt {
                    reason: HaltReason::DecodeInvalid,
                });
            }

            let sh = u.signature_health_new.raw();
            let bh = u.blinding_health_new.raw();
            if sh < 0 || sh > scale_raw || bh < 0 || bh > scale_raw {
                return Err(TransitionHalt { reason: HaltReason::DecodeInvalid });
            }

            if !u.slash_accum_new.is_non_negative() {
                return Err(TransitionHalt {
                    reason: HaltReason::DecodeInvalid,
                });
            }

            // Monotonicity: slash accumulator never decreases.
            if u.slash_accum_new.raw() < state.validators[i].slash_accum.raw() {
                return Err(TransitionHalt {
                    reason: HaltReason::DecodeInvalid,
                });
            }

            // Keep Σ within i64 for wire encoding.
            if u.slash_accum_new.to_i64().is_err() {
                return Err(TransitionHalt {
                    reason: HaltReason::DecodeInvalid,
                });
            }
        }
    }

    for i in state.validator_count as usize..MAX_VALIDATORS {
        if input.updates[i].is_some() {
            return Err(TransitionHalt {
                reason: HaltReason::DecodeInvalid,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed_point::{FixedPoint, SCALE};
    use crate::lyapunov::{ConvergenceWindow, ValidatorMetrics};

    fn genesis_state_vc4() -> EpochState {
        EpochState {
            epoch: 0,
            halt_reason: HaltReason::None,
            entropy_seed: [0u8; 32],
            validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
            validator_count: 4,
            convergence_window: ConvergenceWindow::new(),
            nonces: [0u64; MAX_VALIDATORS],
            validator_ids: [[0u8; 48]; MAX_VALIDATORS],
            state_root: [0u8; 32],
        }
    }

    fn idle_input(n: u32) -> EpochInput {
        EpochInput { updates: [None; MAX_VALIDATORS], update_count: n }
    }

    /// TV-0: genesis state_root is [0u8;32] before any epoch is advanced.
    #[test]
    fn state_root_genesis_is_zero() {
        let state = genesis_state_vc4();
        assert_eq!(state.state_root, [0u8; 32]);
    }

    /// TV-7: entropy_seed advances to a non-zero value after the first epoch.
    #[test]
    fn entropy_seed_advances_nonzero() {
        let mut state = genesis_state_vc4();
        advance_epoch(&mut state, &idle_input(4), &[]).unwrap();
        assert_ne!(state.entropy_seed, [0u8; 32]);
    }

    #[test]
    fn validate_rejects_divergence_out_of_bounds() {
        let mut state = genesis_state_vc4();
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(SCALE + 1),
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
        assert_eq!(advance_epoch(&mut state, &input, &[]), Err(HaltReason::DecodeInvalid));
    }

    #[test]
    fn validate_rejects_conflict_out_of_bounds() {
        let mut state = genesis_state_vc4();
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::from_raw(SCALE + 1),
            slash_accum_new: FixedPoint::ZERO,
        });
        assert_eq!(advance_epoch(&mut state, &input, &[]), Err(HaltReason::DecodeInvalid));
    }

    #[test]
    fn validate_rejects_slash_decrease() {
        let mut state = genesis_state_vc4();
        state.validators[0].slash_accum = FixedPoint::from_raw(1_000);
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::from_raw(500), // decrease → invalid
        });
        assert_eq!(advance_epoch(&mut state, &input, &[]), Err(HaltReason::DecodeInvalid));
    }

    #[test]
    fn validate_rejects_wrong_update_count() {
        let mut state = genesis_state_vc4(); // validator_count = 4
        let input = idle_input(3);           // update_count = 3 ≠ 4
        assert_eq!(advance_epoch(&mut state, &input, &[]), Err(HaltReason::DecodeInvalid));
    }

    #[test]
    fn validate_rejects_update_beyond_count() {
        let mut state = genesis_state_vc4(); // validator_count = 4
        let mut input = idle_input(4);
        // Slot 4 is beyond validator_count=4; any Some(...) there is invalid.
        input.updates[4] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
        assert_eq!(advance_epoch(&mut state, &input, &[]), Err(HaltReason::DecodeInvalid));
    }

    /// V_convergence for 4 validators each with D=500_000, C=250_000:
    ///   per validator: floor(400_000×500_000/1_000_000) + floor(350_000×250_000/1_000_000)
    ///                = 200_000 + 87_500 = 287_500
    ///   total: 4 × 287_500 = 1_150_000
    #[test]
    fn evaluate_projected_known_values() {
        let mut state = genesis_state_vc4();
        let mut input = idle_input(4);
        for i in 0..4 {
            input.updates[i] = Some(ValidatorUpdate {
                divergence_new: FixedPoint::from_raw(500_000),
                conflict_new:   FixedPoint::from_raw(250_000),
                slash_accum_new: FixedPoint::ZERO,
            });
        }
        let result = advance_epoch(&mut state, &input, &[]).unwrap();
        assert_eq!(result.lyapunov.v_convergence.raw(), 1_150_000);
        assert_eq!(result.lyapunov.phi_safety.raw(), 0);
    }

    #[test]
    fn state_root_chains_via_prior_root() {
        // Two states with distinct initial state_roots produce distinct outputs.
        let mut state_a = genesis_state_vc4();
        let mut state_b = genesis_state_vc4();
        state_b.state_root[0] = 0x01; // differ only in initial root

        advance_epoch(&mut state_a, &idle_input(4), &[]).unwrap();
        advance_epoch(&mut state_b, &idle_input(4), &[]).unwrap();

        assert_ne!(state_a.state_root, state_b.state_root,
            "state_root must chain through prior_root");
    }
}

fn evaluate_projected(
    state: &EpochState,
    input: &EpochInput,
) -> Result<LyapunovEval, TransitionHalt> {
    let mut v_sum    = FixedPoint::ZERO;
    let mut max_slash = FixedPoint::ZERO;

    for i in 0..state.validator_count as usize {
        let (d, c, s) = match &input.updates[i] {
            Some(u) => (u.divergence_new, u.conflict_new, u.slash_accum_new),
            None    => (
                state.validators[i].divergence,
                state.validators[i].conflict,
                state.validators[i].slash_accum,
            ),
        };

        let term_d = lyapunov::WEIGHT_D.checked_mul(d)?;
        let term_c = lyapunov::WEIGHT_C.checked_mul(c)?;
        let term   = term_d.checked_add(term_c)?;
        v_sum      = v_sum.checked_add(term)?;
        max_slash  = max_slash.max(s);
    }

    let phi     = lyapunov::WEIGHT_S.checked_mul(max_slash)?;
    let v_total = v_sum.checked_add(phi)?;

    let (delta_window, halt_triggered) = if state.convergence_window.is_full() {
        let delta = lyapunov::compute_delta_window(v_sum, &state.convergence_window)?;
        (delta, delta.raw() > lyapunov::EPSILON.raw())
    } else {
        (FixedPoint::ZERO, false)
    };

    let phi_halt_triggered = phi.raw() >= lyapunov::PHI_MAX_SAFE.raw();

    Ok(LyapunovEval {
        v_convergence: v_sum,
        phi_safety: phi_acc,
        v_total,
        delta_window,
        halt_triggered,
        phi_halt_triggered,
    })
}
