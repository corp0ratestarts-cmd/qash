//! Epoch transition (atomic, infallible commit phase).

use crate::encoding::EncodeError;
use crate::envelope::{PROTOCOL_VERSION_V1_1, PROTOCOL_VERSION_V1_2};
use crate::fixed_point::{FixedPoint, OverflowError, SCALE};
use crate::hash::{h_domain, h_domain_finish, h_domain_start, DomainTag};
use sha3::Digest as _;
use crate::lyapunov::{
    self, ConvergenceWindow, LyapunovError, LyapunovEval, ValidatorMetrics, WINDOW_SIZE,
};
use crate::public::PublicTranscript;
use crate::sharding::{compute_efb, EpochFinalityBeacon, ShardCommitment, ShardingError};

/// Protocol-facing limit (u32 per Domain A rules). Used in wire validation.
pub const MAX_VALIDATORS_WIRE: u32 = 1024;
/// Array-sizing alias (usize is required by Rust array syntax; not stored in state).
pub const MAX_VALIDATORS: usize = MAX_VALIDATORS_WIRE as usize;

/// v1.1 cascade depth: number of consecutive healthy epochs required for finality.
pub const CASCADE_DEPTH: u32 = 8;
/// Epochs before v1.0 envelopes are rejected and cascade health is required.
pub const COMPATIBILITY_WINDOW: u64 = 100;

// Full-state wire format v1.1/v1.2 (canonical, deterministic):
// [epoch:8][state_root:32][ledger_root:32][entropy_seed:32][halt:1][pad:3][vc:4]
// [cascade_health:4][pad:4]                                ← v1.1 addition (8 bytes)
// [N x (div:8 + conf:8 + slash:8 + nonce:8 + id:48)]
// [window_filled:1][pad:3][window_vals:3x8]
// [receipt_root:32][efb_root:32] when either v1.2 sharding root is nonzero
pub const FULL_STATE_FIXED_BYTES: usize = 120;
pub const FULL_STATE_PER_VALIDATOR_BYTES: usize = 80;
pub const FULL_STATE_WINDOW_BYTES: usize = 28;
pub const FULL_STATE_ROOT_BYTES: usize = 64;
pub const FULL_STATE_BASE_MAX_BYTES: usize = FULL_STATE_FIXED_BYTES
    + MAX_VALIDATORS * FULL_STATE_PER_VALIDATOR_BYTES
    + FULL_STATE_WINDOW_BYTES;
pub const FULL_STATE_MAX_BYTES: usize = FULL_STATE_BASE_MAX_BYTES + FULL_STATE_ROOT_BYTES;
// = 120 + 81920 + 28 + 64 = 82,132

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HaltReason {
    None = 0x00,
    LyapunovViolation = 0x01,   // H1
    ArithOverflow = 0x02,       // H2
    EpochOverflow = 0x03,       // H3
    DecodeInvalid = 0x04,       // H4
    RoundtripFailure = 0x05,    // H5
    HaltFlagSet = 0x06,         // H6 (explicit external halt; reserved)
    PhiSafetyViolation = 0x07,  // H7
    IncompatibleVersion = 0x08, // H8: v1.0 envelope rejected after compatibility window
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
            0x07 => Ok(HaltReason::PhiSafetyViolation),
            0x08 => Ok(HaltReason::IncompatibleVersion),
            _ => Err(EncodeError::InvalidHaltCode),
        }
    }
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
    /// Protocol version of the originating envelope (PROTOCOL_VERSION_V1_0 or V1_1).
    /// After epoch COMPATIBILITY_WINDOW, V1_0 envelopes are rejected with IncompatibleVersion.
    /// Default: PROTOCOL_VERSION_V1_1.
    pub protocol_version: u32,
}

impl EpochInput {
    pub fn new(update_count: u32) -> Self {
        Self {
            updates: [None; MAX_VALIDATORS],
            update_count,
            protocol_version: PROTOCOL_VERSION_V1_1,
        }
    }
}

#[derive(Clone, Copy)]
pub struct EpochState {
    pub epoch: u64,
    pub halt_reason: HaltReason,
    pub entropy_seed: [u8; 32],
    pub validators: [ValidatorMetrics; MAX_VALIDATORS],
    pub validator_count: u32,
    pub convergence_window: ConvergenceWindow,
    /// TX replay protection (one nonce per validator slot).
    pub nonces: [u64; MAX_VALIDATORS],
    /// Stable 48-byte consensus identity for each validator slot (fixed at genesis).
    pub validator_ids: [[u8; 48]; MAX_VALIDATORS],
    /// v1.1 cascade health counter: increments each clean epoch, resets on gap, saturates at CASCADE_DEPTH.
    pub cascade_health: u32,
    /// This epoch's committed state root; used as prior_root for the next epoch.
    pub state_root: [u8; 32],
    /// This epoch's aggregate cross-shard receipt root. Zero before v1.2 sharding activates.
    pub receipt_root: [u8; 32],
    /// This epoch's Epoch Finality Beacon root. Zero before v1.2 sharding activates.
    pub efb_root: [u8; 32],
    /// v1.1 causal fingerprint: running H_domain chain over (prev_fingerprint || epoch || state_root).
    /// Tracks full causal history; equal fingerprints ⟹ bisimilar states (cf. proofs/safety/causal_fingerprint.v).
    /// Not included in the state_root commitment — parallel divergence-detection chain.
    pub causal_fingerprint: [u8; 32],
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
pub fn encode_full_state_into(state: &EpochState, out: &mut [u8; FULL_STATE_MAX_BYTES]) -> usize {
    let len = write_state_base_bytes(state, &state.state_root, out);
    append_sharding_roots_if_present(state, out, len)
}

fn write_state_base_bytes(state: &EpochState, root_field: &[u8; 32], out: &mut [u8]) -> usize {
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

    out[pos] = state.halt_reason as u8;
    out[pos + 1] = 0x00;
    out[pos + 2] = 0x00;
    out[pos + 3] = 0x00;
    pos += 4;

    out[pos..pos + 4].copy_from_slice(&state.validator_count.to_le_bytes());
    pos += 4;

    // v1.1: cascade_health (u32 LE) + 4 bytes pad = 8 bytes
    out[pos..pos + 4].copy_from_slice(&state.cascade_health.to_le_bytes());
    out[pos + 4] = 0x00;
    out[pos + 5] = 0x00;
    out[pos + 6] = 0x00;
    out[pos + 7] = 0x00;
    pos += 8;

    // 120 fixed-header bytes consumed.

    for i in 0..state.validator_count as usize {
        let v = &state.validators[i];
        out[pos..pos + 8].copy_from_slice(&fp_to_i64_wire(v.divergence).to_le_bytes());
        out[pos + 8..pos + 16].copy_from_slice(&fp_to_i64_wire(v.conflict).to_le_bytes());
        out[pos + 16..pos + 24].copy_from_slice(&fp_to_i64_wire(v.slash_accum).to_le_bytes());
        out[pos + 24..pos + 32].copy_from_slice(&state.nonces[i].to_le_bytes());
        out[pos + 32..pos + 80].copy_from_slice(&state.validator_ids[i]);
        pos += 80;
    }

    // Window: filled (1) + pad (3) + 3 x i64 (24) = 28 bytes.
    let (filled, values) = state.convergence_window.raw_parts();
    out[pos] = filled;
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

fn append_sharding_roots_if_present(state: &EpochState, out: &mut [u8], pos: usize) -> usize {
    if state.receipt_root == [0u8; 32] && state.efb_root == [0u8; 32] {
        return pos;
    }

    out[pos..pos + 32].copy_from_slice(&state.receipt_root);
    out[pos + 32..pos + 64].copy_from_slice(&state.efb_root);
    pos + FULL_STATE_ROOT_BYTES
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

    // v1.1: cascade_health (u32 LE) + 4 bytes pad
    let mut ch_bytes = [0u8; 4];
    ch_bytes.copy_from_slice(&bytes[pos..pos + 4]);
    let cascade_health = u32::from_le_bytes(ch_bytes);
    if bytes[pos + 4] != 0x00
        || bytes[pos + 5] != 0x00
        || bytes[pos + 6] != 0x00
        || bytes[pos + 7] != 0x00
    {
        return Err(EncodeError::DecodeInvalid);
    }
    pos += 8;

    if cascade_health > CASCADE_DEPTH {
        return Err(EncodeError::DecodeInvalid);
    }

    let vc = validator_count as usize;
    let expected_base_len =
        FULL_STATE_FIXED_BYTES + vc * FULL_STATE_PER_VALIDATOR_BYTES + FULL_STATE_WINDOW_BYTES;
    let expected_with_roots = expected_base_len + FULL_STATE_ROOT_BYTES;
    if bytes.len() != expected_base_len && bytes.len() != expected_with_roots {
        return Err(EncodeError::DecodeInvalid);
    }
    let has_sharding_roots = bytes.len() == expected_with_roots;

    let scale_raw = SCALE;
    let mut validators = [ValidatorMetrics::ZERO; MAX_VALIDATORS];
    let mut nonces = [0u64; MAX_VALIDATORS];
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];

    for i in 0..vc {
        let mut d_b = [0u8; 8];
        let mut c_b = [0u8; 8];
        let mut s_b = [0u8; 8];
        let mut n_b = [0u8; 8];
        let mut id = [0u8; 48];

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
            divergence: FixedPoint::from_raw(d_raw),
            conflict: FixedPoint::from_raw(c_raw),
            slash_accum: FixedPoint::from_raw(s_raw),
        };
        nonces[i] = nonce;
        validator_ids[i] = id;
    }

    // Window: filled (1) + pad (3) + WINDOW_SIZE x i64.
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

    let mut receipt_root = [0u8; 32];
    let mut efb_root = [0u8; 32];
    if has_sharding_roots {
        receipt_root.copy_from_slice(&bytes[pos..pos + 32]);
        efb_root.copy_from_slice(&bytes[pos + 32..pos + 64]);
        if receipt_root == [0u8; 32] && efb_root == [0u8; 32] {
            return Err(EncodeError::DecodeInvalid);
        }
        pos += FULL_STATE_ROOT_BYTES;
    }

    let _ = pos; // consumed exactly the validated byte length
    Ok(EpochState {
        epoch,
        halt_reason,
        entropy_seed,
        validators,
        validator_count,
        convergence_window: window,
        nonces,
        validator_ids,
        cascade_health,
        state_root,
        receipt_root,
        efb_root,
        causal_fingerprint: [0u8; 32], // not wire-encoded; resets on decode (runtime-only chain)
    })
}

/// Cast FixedPoint to i64 for wire encoding.
/// All consensus-path values are validated to fit i64 before reaching the commit phase:
/// D, C <= SCALE = 1_000_000; Sigma validated <= i64::MAX; V_convergence <= 768_000_000.
#[inline]
fn fp_to_i64_wire(fp: FixedPoint) -> i64 {
    debug_assert!(fp.raw() >= i64::MIN as i128 && fp.raw() <= i64::MAX as i128);
    fp.raw() as i64
}

/// Stream the canonical commitment encoding of `state` into `h`, substituting
/// `prior_root` for the `state_root` field. Produces the same byte sequence as
/// `encode_for_commitment_into` without allocating a full-sized stack buffer.
fn stream_state_for_commitment(state: &EpochState, prior_root: &[u8; 32], h: &mut sha3::Sha3_256) {
    h.update(state.epoch.to_le_bytes());
    h.update(prior_root);
    h.update([0u8; 32]); // ledger_root: zeros
    h.update(state.entropy_seed);
    h.update([state.halt_reason as u8, 0x00, 0x00, 0x00]);
    h.update(state.validator_count.to_le_bytes());
    // v1.1 cascade_health + 4 bytes pad
    h.update(state.cascade_health.to_le_bytes());
    h.update([0u8; 4]);

    for i in 0..state.validator_count as usize {
        let v = &state.validators[i];
        h.update(fp_to_i64_wire(v.divergence).to_le_bytes());
        h.update(fp_to_i64_wire(v.conflict).to_le_bytes());
        h.update(fp_to_i64_wire(v.slash_accum).to_le_bytes());
        h.update(state.nonces[i].to_le_bytes());
        h.update(state.validator_ids[i]);
    }

    let (filled, values) = state.convergence_window.raw_parts();
    h.update([filled, 0x00, 0x00, 0x00]);
    for v in values.iter() {
        h.update(fp_to_i64_wire(*v).to_le_bytes());
    }

    if state.receipt_root != [0u8; 32] || state.efb_root != [0u8; 32] {
        h.update(state.receipt_root);
        h.update(state.efb_root);
    }
}

fn compute_state_root(state: &EpochState, prior_root: &[u8; 32]) -> [u8; 32] {
    let mut h = h_domain_start(DomainTag::StateRoot);
    stream_state_for_commitment(state, prior_root, &mut h);
    h_domain_finish(h)
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
            LyapunovError::Overflow => TransitionHalt {
                reason: HaltReason::ArithOverflow,
            },
            LyapunovError::UnboundedMetric => TransitionHalt {
                reason: HaltReason::DecodeInvalid,
            },
        }
    }
}

impl From<ShardingError> for TransitionHalt {
    fn from(_: ShardingError) -> Self {
        TransitionHalt {
            reason: HaltReason::DecodeInvalid,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TransitionResult {
    pub state_root: [u8; 32],
    pub lyapunov: LyapunovEval,
    pub public_transcript: PublicTranscript,
    pub efb: Option<EpochFinalityBeacon>,
}

pub struct EpochShardingInput<'a> {
    pub shard_commitments: &'a [ShardCommitment],
    pub zk_batch_root: [u8; 32],
}

pub fn advance_epoch(
    state: &mut EpochState,
    input: &EpochInput,
    raw_txs: &[&[u8]],
) -> Result<TransitionResult, HaltReason> {
    if state.is_halted() {
        return Err(state.halt_reason);
    }

    match run_pipeline(state, input, raw_txs, None) {
        Ok(r) => Ok(r),
        Err(h) => {
            state.halt_reason = h.reason;
            Err(h.reason)
        }
    }
}

pub fn advance_epoch_sharded(
    state: &mut EpochState,
    input: &EpochInput,
    raw_txs: &[&[u8]],
    sharding: &EpochShardingInput<'_>,
) -> Result<TransitionResult, HaltReason> {
    if state.is_halted() {
        return Err(state.halt_reason);
    }

    match run_pipeline(state, input, raw_txs, Some(sharding)) {
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
    sharding: Option<&EpochShardingInput<'_>>,
) -> Result<TransitionResult, TransitionHalt> {
    // +--------------------------------------------------+
    // | PRE-COMMIT PHASE: state is READ-ONLY             |
    // | Any error returns without mutation.              |
    // +--------------------------------------------------+

    step_1_validate(state, input)?;

    let lyap = evaluate_projected(state, input)?;
    if lyap.halt_triggered {
        return Err(TransitionHalt {
            reason: HaltReason::LyapunovViolation,
        });
    }
    if lyap.phi_halt_triggered {
        return Err(TransitionHalt {
            reason: HaltReason::PhiSafetyViolation,
        });
    }

    let next_epoch = state.epoch.checked_add(1).ok_or(TransitionHalt {
        reason: HaltReason::EpochOverflow,
    })?;
    let next_entropy = h_domain(DomainTag::EntropyAdvance, &state.entropy_seed);

    let mut next_validators = state.validators;
    for (next_v, update) in next_validators[..state.validator_count as usize]
        .iter_mut()
        .zip(input.updates[..state.validator_count as usize].iter())
    {
        if let Some(ref u) = update {
            next_v.divergence = u.divergence_new;
            next_v.conflict = u.conflict_new;
            next_v.slash_accum = u.slash_accum_new;
        }
    }

    let mut tx_base = *state;
    tx_base.validators = next_validators;
    let tx_plan = crate::transaction::prevalidate_all(&tx_base, raw_txs, state.validator_count)
        .map_err(|_| TransitionHalt {
            reason: HaltReason::ArithOverflow,
        })?;
    tx_plan
        .apply_divergence_updates(&mut next_validators)
        .map_err(|_| TransitionHalt {
            reason: HaltReason::ArithOverflow,
        })?;

    let mut next_window = state.convergence_window;
    next_window.push(lyap.v_convergence);

    // Capture prior root before any mutation for chain continuity commitment.
    let prior_root = state.state_root;

    // v1.1: advance cascade health counter.
    // Condition: all active validators are fully idle (D == 0 AND C == 0) after updates.
    // Even a single unit of divergence or conflict resets cascade health to 0.
    let cascade_ok = (0..state.validator_count as usize).all(|i| {
        next_validators[i].divergence == FixedPoint::ZERO
            && next_validators[i].conflict == FixedPoint::ZERO
    });
    let new_cascade_health = if cascade_ok {
        state.cascade_health.saturating_add(1).min(CASCADE_DEPTH)
    } else {
        0
    };

    let efb = match sharding {
        Some(sharding) => {
            if input.protocol_version < PROTOCOL_VERSION_V1_2 {
                return Err(TransitionHalt {
                    reason: HaltReason::IncompatibleVersion,
                });
            }
            Some(compute_efb(
                next_epoch,
                state.efb_root,
                sharding.shard_commitments,
                sharding.zk_batch_root,
            )?)
        }
        None => None,
    };
    let next_receipt_root = match efb {
        Some(efb) => efb.aggregate_receipt_root,
        None => state.receipt_root,
    };
    let next_efb_root = match efb {
        Some(efb) => efb.efb_root,
        None => state.efb_root,
    };

    let mut projected = *state;
    projected.validators = next_validators;
    projected.nonces = tx_plan.next_nonces;
    projected.convergence_window = next_window;
    projected.entropy_seed = next_entropy;
    projected.epoch = next_epoch;
    projected.cascade_health = new_cascade_health;
    projected.receipt_root = next_receipt_root;
    projected.efb_root = next_efb_root;
    let root = compute_state_root(&projected, &prior_root);

    // v1.1 causal fingerprint: H_domain(CausalFingerprint, prev_fp || epoch_le || state_root).
    // Chains the full transition history; equal fingerprints ⟹ equal histories.
    let new_fingerprint = {
        let mut fp_input = [0u8; 72];
        fp_input[..32].copy_from_slice(&state.causal_fingerprint);
        fp_input[32..40].copy_from_slice(&next_epoch.to_le_bytes());
        fp_input[40..72].copy_from_slice(&root);
        h_domain(DomainTag::CausalFingerprint, &fp_input)
    };

    // +==================================================+
    // | COMMIT POINT                                     |
    // | Below: assignments only. No `?`. No checked ops. |
    // +==================================================+

    state.validators = next_validators;
    state.nonces = tx_plan.next_nonces;
    state.convergence_window = next_window;
    state.entropy_seed = next_entropy;
    state.epoch = next_epoch;
    state.cascade_health = new_cascade_health;
    state.receipt_root = next_receipt_root;
    state.efb_root = next_efb_root;
    state.state_root = root;
    state.causal_fingerprint = new_fingerprint;

    let public_transcript = PublicTranscript {
        state_root: root,
        receipt_root: next_receipt_root,
        efb_root: next_efb_root,
        epoch: next_epoch,
        halt_flag: false,
    };

    Ok(TransitionResult {
        state_root: root,
        lyapunov: lyap,
        public_transcript,
        efb,
    })
}

/// Validate that an envelope's epoch is within the accepted window.
///
/// Returns `Err(HaltReason::DecodeInvalid)` for pre-genesis epochs,
/// `Err(HaltReason::EpochOverflow)` on checked_add overflow,
/// and `Ok(())` when the epoch is within `[genesis_epoch, current_epoch + skew_bound]`.
///
/// Called in Domain B before forwarding an envelope to Domain A. Not called inside
/// `advance_epoch` itself because the epoch check belongs at the admission boundary.
pub fn validate_envelope_epoch(
    envelope_epoch: u64,
    genesis_epoch: u64,
    current_epoch: u64,
    skew_bound: u64,
) -> Result<(), HaltReason> {
    if envelope_epoch < genesis_epoch {
        return Err(HaltReason::DecodeInvalid);
    }
    let max_future = current_epoch
        .checked_add(skew_bound)
        .ok_or(HaltReason::EpochOverflow)?;
    if envelope_epoch > max_future {
        return Err(HaltReason::DecodeInvalid);
    }
    Ok(())
}

fn step_1_validate(state: &EpochState, input: &EpochInput) -> Result<(), TransitionHalt> {
    // H8: after the compatibility window, reject any v1.0 envelope.
    if state.epoch >= COMPATIBILITY_WINDOW && input.protocol_version < PROTOCOL_VERSION_V1_1 {
        return Err(TransitionHalt {
            reason: HaltReason::IncompatibleVersion,
        });
    }

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

    let scale_raw = SCALE;

    for i in 0..state.validator_count as usize {
        // §A4 sparse update semantics (normative):
        // None  = identity — validator metrics are unchanged; NOT an absence or liveness signal.
        // Some(u) = explicit update — all three fields (D, C, slash_accum) are replaced atomically.
        // Omission can never affect validator liveness. Future implementations MUST preserve this.
        if let Some(ref u) = input.updates[i] {
            let d = u.divergence_new.raw();
            let c = u.conflict_new.raw();

            if d < 0 || d > scale_raw || c < 0 || c > scale_raw {
                return Err(TransitionHalt {
                    reason: HaltReason::DecodeInvalid,
                });
            }

            if !u.slash_accum_new.is_non_negative() {
                return Err(TransitionHalt {
                    reason: HaltReason::DecodeInvalid,
                });
            }

            if u.slash_accum_new.raw() < state.validators[i].slash_accum.raw() {
                return Err(TransitionHalt {
                    reason: HaltReason::DecodeInvalid,
                });
            }

            // Keep Sigma within i64 for wire encoding.
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
            cascade_health: 0,
            state_root: [0u8; 32],
            receipt_root: [0u8; 32],
            efb_root: [0u8; 32],
            causal_fingerprint: [0u8; 32],
        }
    }

    fn idle_input(n: u32) -> EpochInput {
        EpochInput::new(n)
    }

    fn set_distinct_validator_ids(state: &mut EpochState) {
        for i in 0..state.validator_count as usize {
            state.validator_ids[i][0] = i as u8 + 1;
        }
    }

    fn make_tx0_raw(author_id: [u8; 48], nonce: u64) -> [u8; crate::transaction::TX0_WIRE_BYTES] {
        let mut raw = [0u8; crate::transaction::TX0_WIRE_BYTES];
        raw[0..2].copy_from_slice(&crate::transaction::TX_VERSION.to_le_bytes());
        raw[2..4].copy_from_slice(&crate::transaction::TX_TYPE_NOOP.to_le_bytes());
        raw[4..12].copy_from_slice(&nonce.to_le_bytes());
        raw[12..60].copy_from_slice(&author_id);
        raw[60..64].copy_from_slice(&0u32.to_le_bytes());
        raw
    }

    fn author_id(slot: u8) -> [u8; 48] {
        let mut id = [0u8; 48];
        id[0] = slot + 1;
        id
    }

    fn sort_key_for(raw: &[u8; crate::transaction::TX0_WIRE_BYTES]) -> [u8; 32] {
        crate::transaction::sort_key(&[0u8; 32], &crate::transaction::tx_id(raw))
    }

    fn ordered_two_validator_txs() -> (
        [u8; crate::transaction::TX0_WIRE_BYTES],
        [u8; crate::transaction::TX0_WIRE_BYTES],
    ) {
        let tx0 = make_tx0_raw(author_id(0), 0);
        let mut overflow_tx = make_tx0_raw(author_id(1), u64::MAX);
        for b in 0u16..=u8::MAX as u16 {
            overflow_tx[crate::transaction::TX_HEADER_BYTES] = b as u8;
            if sort_key_for(&tx0) < sort_key_for(&overflow_tx) {
                return (tx0, overflow_tx);
            }
        }
        panic!("could not construct ordered transaction pair");
    }

    fn ordered_same_author_txs() -> (
        [u8; crate::transaction::TX0_WIRE_BYTES],
        [u8; crate::transaction::TX0_WIRE_BYTES],
    ) {
        let tx0 = make_tx0_raw(author_id(0), 0);
        let mut tx1 = make_tx0_raw(author_id(0), 1);
        for b in 0u16..=u8::MAX as u16 {
            tx1[crate::transaction::TX_HEADER_BYTES] = b as u8;
            if sort_key_for(&tx0) < sort_key_for(&tx1) {
                return (tx0, tx1);
            }
        }
        panic!("could not construct ordered same-author transaction pair");
    }

    fn assert_state_unchanged_except_halt(
        before: &EpochState,
        after: &EpochState,
        halt: HaltReason,
    ) {
        assert_eq!(after.halt_reason, halt);
        assert_eq!(after.epoch, before.epoch);
        assert_eq!(after.entropy_seed, before.entropy_seed);
        assert_eq!(after.validators, before.validators);
        assert_eq!(after.validator_count, before.validator_count);
        assert_eq!(after.convergence_window, before.convergence_window);
        assert_eq!(after.nonces, before.nonces);
        assert_eq!(after.validator_ids, before.validator_ids);
        assert_eq!(after.state_root, before.state_root);
        assert_eq!(after.receipt_root, before.receipt_root);
        assert_eq!(after.efb_root, before.efb_root);
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
        assert_eq!(
            advance_epoch(&mut state, &input, &[]),
            Err(HaltReason::DecodeInvalid)
        );
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
        assert_eq!(
            advance_epoch(&mut state, &input, &[]),
            Err(HaltReason::DecodeInvalid)
        );
    }

    #[test]
    fn validate_rejects_slash_decrease() {
        let mut state = genesis_state_vc4();
        state.validators[0].slash_accum = FixedPoint::from_raw(1_000);
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::from_raw(500), // decrease -> invalid
        });
        assert_eq!(
            advance_epoch(&mut state, &input, &[]),
            Err(HaltReason::DecodeInvalid)
        );
    }

    #[test]
    fn validate_rejects_wrong_update_count() {
        let mut state = genesis_state_vc4(); // validator_count = 4
        let input = idle_input(3); // update_count = 3 != 4
        assert_eq!(
            advance_epoch(&mut state, &input, &[]),
            Err(HaltReason::DecodeInvalid)
        );
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
        assert_eq!(
            advance_epoch(&mut state, &input, &[]),
            Err(HaltReason::DecodeInvalid)
        );
    }

    /// V_convergence for 4 validators each with D=500_000, C=250_000:
    ///   per validator: floor(400_000x500_000/1_000_000) + floor(350_000x250_000/1_000_000)
    ///                = 200_000 + 87_500 = 287_500
    ///   total: 4 x 287_500 = 1_150_000
    #[test]
    fn evaluate_projected_known_values() {
        let mut state = genesis_state_vc4();
        let mut input = idle_input(4);
        for i in 0..4 {
            input.updates[i] = Some(ValidatorUpdate {
                divergence_new: FixedPoint::from_raw(500_000),
                conflict_new: FixedPoint::from_raw(250_000),
                slash_accum_new: FixedPoint::ZERO,
            });
        }
        let result = advance_epoch(&mut state, &input, &[]).unwrap();
        assert_eq!(result.lyapunov.v_convergence.raw(), 1_150_000);
        assert_eq!(result.lyapunov.phi_safety.raw(), 0);
    }

    #[test]
    fn evaluate_projected_phi_safety_sums_across_validators() {
        let mut state = genesis_state_vc4();
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::from_raw(400_000_000),
        });
        input.updates[1] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::from_raw(400_000_000),
        });

        let result = advance_epoch(&mut state, &input, &[]).unwrap();

        assert_eq!(result.lyapunov.phi_safety.raw(), 200_000_000);
        assert!(!result.lyapunov.phi_halt_triggered);
    }

    #[test]
    fn phi_safety_halts_at_threshold_before_commit() {
        let mut state = genesis_state_vc4();
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::from_raw(1_000_000_000),
        });
        input.updates[1] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::from_raw(1_000_000_000),
        });

        let result = advance_epoch(&mut state, &input, &[]);

        assert_eq!(result, Err(HaltReason::PhiSafetyViolation));
        assert_eq!(state.halt_reason, HaltReason::PhiSafetyViolation);
        assert_eq!(state.epoch, 0);
        assert_eq!(state.validators[0].slash_accum, FixedPoint::ZERO);
        assert_eq!(state.validators[1].slash_accum, FixedPoint::ZERO);
    }

    #[test]
    fn phi_safety_halts_one_over_threshold_before_commit() {
        let mut state = genesis_state_vc4();
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::ZERO,
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::from_raw(2_000_000_004),
        });

        let result = advance_epoch(&mut state, &input, &[]);

        assert_eq!(result, Err(HaltReason::PhiSafetyViolation));
        assert_eq!(state.halt_reason, HaltReason::PhiSafetyViolation);
        assert_eq!(state.epoch, 0);
    }

    #[test]
    fn lyapunov_delta_equal_epsilon_does_not_halt() {
        let mut state = genesis_state_vc4();
        state.convergence_window.push(FixedPoint::ZERO);
        state.convergence_window.push(FixedPoint::ZERO);
        state.convergence_window.push(FixedPoint::ZERO);
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(50_000),
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });

        let result = advance_epoch(&mut state, &input, &[]).unwrap();

        assert_eq!(result.lyapunov.delta_window.raw(), lyapunov::EPSILON.raw());
        assert!(!result.lyapunov.halt_triggered);
    }

    #[test]
    fn lyapunov_delta_one_over_epsilon_halts() {
        let mut state = genesis_state_vc4();
        state.convergence_window.push(FixedPoint::ZERO);
        state.convergence_window.push(FixedPoint::ZERO);
        state.convergence_window.push(FixedPoint::ZERO);
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(50_003),
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });

        let result = advance_epoch(&mut state, &input, &[]);

        assert_eq!(result, Err(HaltReason::LyapunovViolation));
        assert_eq!(state.halt_reason, HaltReason::LyapunovViolation);
    }

    #[test]
    fn tx_nonce_overflow_halts_without_partial_commit() {
        let mut state = genesis_state_vc4();
        set_distinct_validator_ids(&mut state);
        state.nonces[0] = u64::MAX;
        let before = state;
        let tx = make_tx0_raw(author_id(0), u64::MAX);

        assert_eq!(
            advance_epoch(&mut state, &idle_input(4), &[tx.as_slice()]),
            Err(HaltReason::ArithOverflow)
        );
        assert_state_unchanged_except_halt(&before, &state, HaltReason::ArithOverflow);
    }

    #[test]
    fn multiple_transactions_from_same_validator_commit_precomputed_nonce() {
        let mut state = genesis_state_vc4();
        set_distinct_validator_ids(&mut state);
        let (tx0, tx1) = ordered_same_author_txs();

        advance_epoch(
            &mut state,
            &idle_input(4),
            &[tx0.as_slice(), tx1.as_slice()],
        )
        .unwrap();

        assert_eq!(state.nonces[0], 2);
        assert_eq!(state.nonces[1], 0);
    }

    #[test]
    fn failure_after_first_transaction_halts_without_partial_commit() {
        let mut state = genesis_state_vc4();
        set_distinct_validator_ids(&mut state);
        state.nonces[1] = u64::MAX;
        let before = state;
        let (tx0, overflow_tx) = ordered_two_validator_txs();

        assert_eq!(
            advance_epoch(
                &mut state,
                &idle_input(4),
                &[tx0.as_slice(), overflow_tx.as_slice()]
            ),
            Err(HaltReason::ArithOverflow)
        );
        assert_state_unchanged_except_halt(&before, &state, HaltReason::ArithOverflow);
    }

    #[test]
    fn state_root_chains_via_prior_root() {
        // Two states with distinct initial state_roots produce distinct outputs.
        let mut state_a = genesis_state_vc4();
        let mut state_b = genesis_state_vc4();
        state_b.state_root[0] = 0x01; // differ only in initial root

        advance_epoch(&mut state_a, &idle_input(4), &[]).unwrap();
        advance_epoch(&mut state_b, &idle_input(4), &[]).unwrap();

        assert_ne!(
            state_a.state_root, state_b.state_root,
            "state_root must chain through prior_root"
        );
    }

    // ------------------------------------------------------------------
    // 2-C: validate_envelope_epoch
    // ------------------------------------------------------------------

    #[test]
    fn validate_epoch_accepts_within_window() {
        assert_eq!(validate_envelope_epoch(5, 0, 5, 1), Ok(()));
        assert_eq!(validate_envelope_epoch(6, 0, 5, 1), Ok(())); // exactly at skew
        assert_eq!(validate_envelope_epoch(0, 0, 10, 0), Ok(())); // at genesis
    }

    #[test]
    fn validate_epoch_rejects_pre_genesis() {
        // envelope_epoch < genesis_epoch
        assert_eq!(
            validate_envelope_epoch(0, 1, 10, 1),
            Err(HaltReason::DecodeInvalid)
        );
    }

    #[test]
    fn validate_epoch_rejects_too_far_future() {
        // envelope_epoch > current_epoch + skew_bound
        assert_eq!(
            validate_envelope_epoch(7, 0, 5, 1),
            Err(HaltReason::DecodeInvalid)
        );
    }

    #[test]
    fn validate_epoch_overflow_on_add() {
        // current_epoch + skew_bound overflows u64
        assert_eq!(
            validate_envelope_epoch(u64::MAX, 0, u64::MAX, 1),
            Err(HaltReason::EpochOverflow)
        );
    }

    #[test]
    fn validate_epoch_zero_skew_accepts_only_current() {
        assert_eq!(validate_envelope_epoch(5, 0, 5, 0), Ok(()));
        assert_eq!(
            validate_envelope_epoch(6, 0, 5, 0),
            Err(HaltReason::DecodeInvalid)
        );
    }

    // ------------------------------------------------------------------
    // 2-D: cascade health tracking
    // ------------------------------------------------------------------

    #[test]
    fn cascade_health_increments_on_clean_epochs() {
        let mut state = genesis_state_vc4(); // all divergence = 0
        for expected in 1..=CASCADE_DEPTH {
            advance_epoch(&mut state, &idle_input(4), &[]).unwrap();
            assert_eq!(state.cascade_health, expected, "epoch {}", state.epoch);
        }
    }

    #[test]
    fn cascade_health_saturates_at_depth() {
        let mut state = genesis_state_vc4();
        // advance past CASCADE_DEPTH epochs
        for _ in 0..=(CASCADE_DEPTH + 2) {
            advance_epoch(&mut state, &idle_input(4), &[]).unwrap();
        }
        assert_eq!(state.cascade_health, CASCADE_DEPTH);
    }

    #[test]
    fn cascade_health_resets_on_high_divergence() {
        let mut state = genesis_state_vc4();
        // Run 4 clean epochs to build health
        for _ in 0..4 {
            advance_epoch(&mut state, &idle_input(4), &[]).unwrap();
        }
        assert_eq!(state.cascade_health, 4);

        // Inject any non-zero divergence: cascade condition requires V_convergence == 0.
        let mut input = idle_input(4);
        input.updates[0] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(1),
            conflict_new: FixedPoint::ZERO,
            slash_accum_new: FixedPoint::ZERO,
        });
        advance_epoch(&mut state, &input, &[]).unwrap();
        assert_eq!(
            state.cascade_health, 0,
            "cascade health must reset on any divergence"
        );
    }

    #[test]
    fn cascade_health_in_state_root_commitment() {
        // Two states identical except cascade_health must produce different state roots.
        let mut state_a = genesis_state_vc4();
        let mut state_b = genesis_state_vc4();
        state_b.cascade_health = 1; // differs only in cascade_health

        advance_epoch(&mut state_a, &idle_input(4), &[]).unwrap();
        advance_epoch(&mut state_b, &idle_input(4), &[]).unwrap();

        assert_ne!(
            state_a.state_root, state_b.state_root,
            "cascade_health must be committed to state root"
        );
    }

    #[test]
    fn incompatible_version_halt_roundtrip() {
        // Ensure HaltReason::IncompatibleVersion (0x08) round-trips through from_u8.
        let r = HaltReason::IncompatibleVersion;
        assert_eq!(r as u8, 0x08);
        let rt = HaltReason::from_u8(0x08).expect("0x08 must decode");
        assert_eq!(rt, HaltReason::IncompatibleVersion);
    }

    #[test]
    fn version_gate_accepts_v1_1_after_window() {
        // A v1.1 envelope must be accepted even at epoch >= COMPATIBILITY_WINDOW.
        let mut state = genesis_state_vc4();
        state.epoch = COMPATIBILITY_WINDOW;
        let mut input = idle_input(4);
        // protocol_version defaults to V1_1 via EpochInput::new; explicit for clarity.
        input.protocol_version = crate::envelope::PROTOCOL_VERSION_V1_1;
        assert!(advance_epoch(&mut state, &input, &[]).is_ok());
    }

    #[test]
    fn version_gate_rejects_v1_0_after_window() {
        // A v1.0 envelope must be rejected at epoch >= COMPATIBILITY_WINDOW with H8.
        let mut state = genesis_state_vc4();
        state.epoch = COMPATIBILITY_WINDOW;
        let mut input = idle_input(4);
        input.protocol_version = crate::envelope::PROTOCOL_VERSION_V1_0;
        let result = advance_epoch(&mut state, &input, &[]);
        assert_eq!(result, Err(HaltReason::IncompatibleVersion));
        assert_eq!(state.halt_reason, HaltReason::IncompatibleVersion);
    }

    #[test]
    fn version_gate_accepts_v1_0_before_window() {
        // A v1.0 envelope must be accepted before the compatibility window closes.
        let mut state = genesis_state_vc4();
        state.epoch = COMPATIBILITY_WINDOW - 1;
        let mut input = idle_input(4);
        input.protocol_version = crate::envelope::PROTOCOL_VERSION_V1_0;
        assert!(advance_epoch(&mut state, &input, &[]).is_ok());
    }

    #[test]
    fn sharded_epoch_commits_efb_to_public_transcript() {
        let mut state = genesis_state_vc4();
        let mut input = idle_input(4);
        input.protocol_version = crate::envelope::PROTOCOL_VERSION_V1_2;
        let shards = [
            crate::sharding::ShardCommitment {
                shard_id: 0,
                state_root: [1u8; 32],
                receipt_root: [2u8; 32],
            },
            crate::sharding::ShardCommitment {
                shard_id: 1,
                state_root: [3u8; 32],
                receipt_root: [4u8; 32],
            },
        ];
        let sharding = EpochShardingInput {
            shard_commitments: &shards,
            zk_batch_root: [9u8; 32],
        };

        let result = advance_epoch_sharded(&mut state, &input, &[], &sharding).unwrap();
        let efb = result.efb.expect("v1.2 sharded transition must return EFB");

        assert_eq!(state.receipt_root, efb.aggregate_receipt_root);
        assert_eq!(state.efb_root, efb.efb_root);
        assert_eq!(result.public_transcript.state_root, state.state_root);
        assert_eq!(result.public_transcript.receipt_root, state.receipt_root);
        assert_eq!(result.public_transcript.efb_root, state.efb_root);
        assert_ne!(state.efb_root, [0u8; 32]);
    }

    #[test]
    fn full_state_roundtrip_preserves_sharding_roots() {
        let mut state = genesis_state_vc4();
        set_distinct_validator_ids(&mut state);
        state.receipt_root = [0xA5; 32];
        state.efb_root = [0x5A; 32];

        let mut encoded = [0u8; FULL_STATE_MAX_BYTES];
        let len = encode_full_state_into(&state, &mut encoded);
        assert_eq!(
            len,
            FULL_STATE_FIXED_BYTES
                + state.validator_count as usize * FULL_STATE_PER_VALIDATOR_BYTES
                + FULL_STATE_WINDOW_BYTES
                + FULL_STATE_ROOT_BYTES
        );

        let decoded = decode_full_state(&encoded[..len]).unwrap();

        assert_eq!(decoded.receipt_root, state.receipt_root);
        assert_eq!(decoded.efb_root, state.efb_root);
    }

    #[test]
    fn decode_rejects_extended_zero_sharding_roots() {
        let state = genesis_state_vc4();
        let mut encoded = [0u8; FULL_STATE_MAX_BYTES];
        let base_len = encode_full_state_into(&state, &mut encoded);
        let padded_len = base_len + FULL_STATE_ROOT_BYTES;

        assert!(matches!(
            decode_full_state(&encoded[..padded_len]),
            Err(EncodeError::DecodeInvalid)
        ));
    }

    #[test]
    fn sharded_epoch_requires_v1_2_protocol() {
        let mut state = genesis_state_vc4();
        let input = idle_input(4);
        let shards = [crate::sharding::ShardCommitment {
            shard_id: 0,
            state_root: [1u8; 32],
            receipt_root: [2u8; 32],
        }];
        let sharding = EpochShardingInput {
            shard_commitments: &shards,
            zk_batch_root: [0u8; 32],
        };

        assert_eq!(
            advance_epoch_sharded(&mut state, &input, &[], &sharding),
            Err(HaltReason::IncompatibleVersion)
        );
        assert_eq!(state.efb_root, [0u8; 32]);
    }
}

fn evaluate_projected(
    state: &EpochState,
    input: &EpochInput,
) -> Result<LyapunovEval, TransitionHalt> {
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
    let phi_halt_triggered = phi.raw() >= lyapunov::PHI_MAX_SAFE.raw();

    let (delta_window, halt_triggered) = if state.convergence_window.is_full() {
        let delta = lyapunov::compute_delta_window(v_sum, &state.convergence_window)?;
        (delta, delta.raw() > lyapunov::EPSILON.raw())
    } else {
        (FixedPoint::ZERO, false)
    };

    Ok(LyapunovEval {
        v_convergence: v_sum,
        phi_safety: phi,
        v_total,
        delta_window,
        halt_triggered,
        phi_halt_triggered,
    })
}
