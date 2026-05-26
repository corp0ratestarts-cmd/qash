//! TX-0/TX-1 transaction pipeline for Domain A.
//!
//! Signature bytes are carried opaquely; verification is Domain B (PAL).
//! Domain A checks deterministic admissibility, replay nonces, canonical
//! ordering, and bounded state effects.

use crate::fixed_point::FixedPoint;
use crate::hash::{h_domain, DomainTag};
use crate::lyapunov::ValidatorMetrics;
use crate::transition::{EpochState, MAX_VALIDATORS};

// ---------------------------------------------------------------------------
// Wire constants
// ---------------------------------------------------------------------------

/// Envelope version field value.
pub const TX_VERSION: u16 = 0x0001;
/// TX type for no-op.
pub const TX_TYPE_NOOP: u16 = 0x0000;
/// TX type for bounded validator divergence decrement.
pub const TX_TYPE_SCORE_DECREMENT: u16 = 0x0001;
/// Dilithium5 signature size (opaque in Domain A).
pub const PQ_SIG_BYTES: usize = 2420;

/// Envelope header layout (64 bytes):
/// [version:2][tx_type:2][nonce:8][author_id:48][payload_len:4]
pub const TX_HEADER_BYTES: usize = 64;

/// Total wire size of a TX-0 envelope (no payload).
pub const TX0_WIRE_BYTES: usize = TX_HEADER_BYTES + PQ_SIG_BYTES;
/// TX-1 payload: [target_idx:u32][delta:u32].
pub const TX1_PAYLOAD_BYTES: usize = 8;
/// Total wire size of a TX-1 envelope.
pub const TX1_WIRE_BYTES: usize = TX_HEADER_BYTES + TX1_PAYLOAD_BYTES + PQ_SIG_BYTES;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxError {
    InvalidVersion,
    UnknownType,
    NonceMismatch { expected: u64, got: u64 },
    AuthorNotFound,
    MalformedEnvelope,
    BudgetExceeded,
    TargetOutOfBounds,
    DeltaExceedsDivergence,
}

// ---------------------------------------------------------------------------
// Parsed transaction
// ---------------------------------------------------------------------------

/// A decoded TX-0 envelope. The signature is kept as a reference to the
/// original raw bytes to avoid copying 2420 bytes onto the stack.
#[derive(Debug)]
pub struct Tx0<'a> {
    pub author_id: [u8; 48],
    pub nonce: u64,
    /// Raw signature bytes (opaque in Domain A).
    pub signature: &'a [u8; PQ_SIG_BYTES],
}

/// A decoded TX-1 BoundedValidatorScoreDecrement envelope.
#[derive(Debug)]
pub struct Tx1<'a> {
    pub author_id: [u8; 48],
    pub nonce: u64,
    pub target_idx: u32,
    pub delta: u32,
    /// Raw signature bytes (opaque in Domain A).
    pub signature: &'a [u8; PQ_SIG_BYTES],
}

#[derive(Debug)]
enum ParsedTx<'a> {
    Tx0(Tx0<'a>),
    Tx1(Tx1<'a>),
}

impl ParsedTx<'_> {
    fn author_id(&self) -> &[u8; 48] {
        match self {
            ParsedTx::Tx0(tx) => &tx.author_id,
            ParsedTx::Tx1(tx) => &tx.author_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Parse
// ---------------------------------------------------------------------------

/// Decode a TX-0 envelope from `raw`. Returns the parsed tx and total bytes
/// consumed (always TX0_WIRE_BYTES on success).
pub fn parse_tx0(raw: &[u8]) -> Result<(Tx0<'_>, usize), TxError> {
    if raw.len() < TX0_WIRE_BYTES {
        return Err(TxError::MalformedEnvelope);
    }

    let mut ver_b = [0u8; 2];
    ver_b.copy_from_slice(&raw[0..2]);
    let version = u16::from_le_bytes(ver_b);
    if version != TX_VERSION {
        return Err(TxError::InvalidVersion);
    }

    let mut typ_b = [0u8; 2];
    typ_b.copy_from_slice(&raw[2..4]);
    let tx_type = u16::from_le_bytes(typ_b);
    if tx_type != TX_TYPE_NOOP {
        return Err(TxError::UnknownType);
    }

    let mut nonce_b = [0u8; 8];
    nonce_b.copy_from_slice(&raw[4..12]);
    let nonce = u64::from_le_bytes(nonce_b);

    let mut author_id = [0u8; 48];
    author_id.copy_from_slice(&raw[12..60]);

    let mut plen_b = [0u8; 4];
    plen_b.copy_from_slice(&raw[60..64]);
    let payload_len = u32::from_le_bytes(plen_b);
    if payload_len != 0 {
        return Err(TxError::MalformedEnvelope);
    }

    let sig_slice = &raw[TX_HEADER_BYTES..TX0_WIRE_BYTES];
    let sig_arr: &[u8; PQ_SIG_BYTES] = match sig_slice.try_into() {
        Ok(a) => a,
        Err(_) => return Err(TxError::MalformedEnvelope),
    };

    Ok((
        Tx0 {
            author_id,
            nonce,
            signature: sig_arr,
        },
        TX0_WIRE_BYTES,
    ))
}

/// Decode a TX-1 envelope from `raw`.
pub fn parse_tx1(raw: &[u8]) -> Result<(Tx1<'_>, usize), TxError> {
    if raw.len() < TX1_WIRE_BYTES {
        return Err(TxError::MalformedEnvelope);
    }

    let mut ver_b = [0u8; 2];
    ver_b.copy_from_slice(&raw[0..2]);
    let version = u16::from_le_bytes(ver_b);
    if version != TX_VERSION {
        return Err(TxError::InvalidVersion);
    }

    let mut typ_b = [0u8; 2];
    typ_b.copy_from_slice(&raw[2..4]);
    let tx_type = u16::from_le_bytes(typ_b);
    if tx_type != TX_TYPE_SCORE_DECREMENT {
        return Err(TxError::UnknownType);
    }

    let mut nonce_b = [0u8; 8];
    nonce_b.copy_from_slice(&raw[4..12]);
    let nonce = u64::from_le_bytes(nonce_b);

    let mut author_id = [0u8; 48];
    author_id.copy_from_slice(&raw[12..60]);

    let mut plen_b = [0u8; 4];
    plen_b.copy_from_slice(&raw[60..64]);
    let payload_len = u32::from_le_bytes(plen_b);
    if payload_len != TX1_PAYLOAD_BYTES as u32 {
        return Err(TxError::MalformedEnvelope);
    }

    let mut target_b = [0u8; 4];
    target_b.copy_from_slice(&raw[64..68]);
    let target_idx = u32::from_le_bytes(target_b);

    let mut delta_b = [0u8; 4];
    delta_b.copy_from_slice(&raw[68..72]);
    let delta = u32::from_le_bytes(delta_b);

    let sig_slice = &raw[TX_HEADER_BYTES + TX1_PAYLOAD_BYTES..TX1_WIRE_BYTES];
    let sig_arr: &[u8; PQ_SIG_BYTES] = match sig_slice.try_into() {
        Ok(a) => a,
        Err(_) => return Err(TxError::MalformedEnvelope),
    };

    Ok((
        Tx1 {
            author_id,
            nonce,
            target_idx,
            delta,
            signature: sig_arr,
        },
        TX1_WIRE_BYTES,
    ))
}

fn parse_tx(raw: &[u8]) -> Result<(ParsedTx<'_>, usize), TxError> {
    if raw.len() < TX_HEADER_BYTES {
        return Err(TxError::MalformedEnvelope);
    }
    let mut typ_b = [0u8; 2];
    typ_b.copy_from_slice(&raw[2..4]);
    match u16::from_le_bytes(typ_b) {
        TX_TYPE_NOOP => {
            let (tx, consumed) = parse_tx0(raw)?;
            Ok((ParsedTx::Tx0(tx), consumed))
        }
        TX_TYPE_SCORE_DECREMENT => {
            let (tx, consumed) = parse_tx1(raw)?;
            Ok((ParsedTx::Tx1(tx), consumed))
        }
        _ => Err(TxError::UnknownType),
    }
}

// ---------------------------------------------------------------------------
// tx_id: canonical identifier (used for sort key computation)
// ---------------------------------------------------------------------------

/// tx_id = H_domain(TxId, canonical tx bytes)
/// Commits to all envelope fields including the opaque signature.
pub fn tx_id(raw: &[u8; TX0_WIRE_BYTES]) -> [u8; 32] {
    h_domain(DomainTag::TxId, raw.as_slice())
}

fn tx_id_bytes(raw: &[u8], consumed: usize) -> Result<[u8; 32], TxError> {
    if raw.len() < consumed {
        return Err(TxError::MalformedEnvelope);
    }
    Ok(h_domain(DomainTag::TxId, &raw[..consumed]))
}

// ---------------------------------------------------------------------------
// Sort key
// ---------------------------------------------------------------------------

/// Sort key = H_domain(EntropyAdvance, entropy_seed || tx_id_bytes)
/// Deterministic canonical ordering for an epoch's transaction set.
pub fn sort_key(entropy_seed: &[u8; 32], tx_id_bytes: &[u8; 32]) -> [u8; 32] {
    let mut input = [0u8; 64];
    input[..32].copy_from_slice(entropy_seed);
    input[32..].copy_from_slice(tx_id_bytes);
    h_domain(DomainTag::EntropyAdvance, &input)
}

// ---------------------------------------------------------------------------
// Admissibility
// ---------------------------------------------------------------------------

/// Find the validator slot index whose id matches `author_id`. O(N) scan.
///
/// Uses `==` on 48-byte arrays (not constant-time). This is safe because
/// `author_id` and all `validator_ids` are public consensus data.
pub(crate) fn index_of_validator(state: &EpochState, author_id: &[u8; 48]) -> Option<usize> {
    (0..state.validator_count as usize).find(|&i| &state.validator_ids[i] == author_id)
}

/// Check that the tx is admissible against the current state.
/// For TX-0: author_id found in validator set and nonce matches exactly.
pub fn is_admissible(state: &EpochState, tx: &Tx0<'_>) -> Result<usize, TxError> {
    let idx = index_of_validator(state, &tx.author_id).ok_or(TxError::AuthorNotFound)?;
    let expected = state.nonces[idx];
    if tx.nonce != expected {
        return Err(TxError::NonceMismatch {
            expected,
            got: tx.nonce,
        });
    }
    Ok(idx)
}

fn check_nonce(next_nonces: &[u64; MAX_VALIDATORS], idx: usize, nonce: u64) -> Result<(), TxError> {
    let expected = next_nonces[idx];
    if nonce != expected {
        return Err(TxError::NonceMismatch {
            expected,
            got: nonce,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Apply TX-0
// ---------------------------------------------------------------------------

/// Apply TX-0: increment the author's nonce. No other state change.
/// Caller must pass the slot index returned by `is_admissible`.
pub fn apply_tx_0(state: &mut EpochState, idx: usize) -> Result<(), TxError> {
    if idx >= state.validator_count as usize {
        return Err(TxError::MalformedEnvelope);
    }
    state.nonces[idx] = state.nonces[idx]
        .checked_add(1)
        .ok_or(TxError::MalformedEnvelope)?;
    Ok(())
}

/// Apply TX-1: decrement target divergence and increment author nonce.
pub fn apply_tx_1(state: &mut EpochState, author_idx: usize, tx: &Tx1<'_>) -> Result<(), TxError> {
    apply_tx_1_projected(
        &mut state.validators,
        &mut state.nonces,
        state.validator_count,
        author_idx,
        tx,
    )
}

pub fn tx1_project_divergence(current: FixedPoint, delta: u32) -> Result<FixedPoint, TxError> {
    let current_raw = current.raw();
    let delta_raw = i128::from(delta);
    if delta_raw > current_raw {
        return Err(TxError::DeltaExceedsDivergence);
    }
    Ok(FixedPoint::from_raw(current_raw - delta_raw))
}

fn apply_tx_1_projected(
    validators: &mut [ValidatorMetrics; MAX_VALIDATORS],
    next_nonces: &mut [u64; MAX_VALIDATORS],
    validator_count: u32,
    author_idx: usize,
    tx: &Tx1<'_>,
) -> Result<(), TxError> {
    if author_idx >= validator_count as usize {
        return Err(TxError::MalformedEnvelope);
    }
    let target_idx = usize::try_from(tx.target_idx).map_err(|_| TxError::TargetOutOfBounds)?;
    if target_idx >= validator_count as usize {
        return Err(TxError::TargetOutOfBounds);
    }

    validators[target_idx].divergence =
        tx1_project_divergence(validators[target_idx].divergence, tx.delta)?;
    next_nonces[author_idx] = next_nonces[author_idx]
        .checked_add(1)
        .ok_or(TxError::MalformedEnvelope)?;
    Ok(())
}

fn apply_tx_1_cached(
    projected_divergences: &mut [FixedPoint; MAX_VALIDATORS],
    next_nonces: &mut [u64; MAX_VALIDATORS],
    validator_count: u32,
    author_idx: usize,
    target_idx_raw: u32,
    delta: u32,
) -> Result<(u32, FixedPoint), TxError> {
    if author_idx >= validator_count as usize {
        return Err(TxError::MalformedEnvelope);
    }
    let target_idx = usize::try_from(target_idx_raw).map_err(|_| TxError::TargetOutOfBounds)?;
    if target_idx >= validator_count as usize {
        return Err(TxError::TargetOutOfBounds);
    }

    let next_divergence = tx1_project_divergence(projected_divergences[target_idx], delta)?;
    projected_divergences[target_idx] = next_divergence;
    next_nonces[author_idx] = next_nonces[author_idx]
        .checked_add(1)
        .ok_or(TxError::MalformedEnvelope)?;
    Ok((target_idx_raw, next_divergence))
}

// ---------------------------------------------------------------------------
// TxPrevalidation: result of stateless prevalidation pass
// ---------------------------------------------------------------------------

/// Result of transaction prevalidation.
///
/// `next_nonces` is the complete nonce array to assign during transition commit.
/// All parsing, filtering, sorting, admissibility checks, and nonce overflow
/// checks needed to compute it have already completed before this value exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TxPrevalidation {
    pub next_nonces: [u64; MAX_VALIDATORS],
    pub divergence_targets: [u32; MAX_VALIDATORS],
    pub divergence_values: [FixedPoint; MAX_VALIDATORS],
    pub divergence_update_count: u32,
    pub applied_count: u32,
}

impl TxPrevalidation {
    pub fn apply_divergence_updates(
        &self,
        validators: &mut [ValidatorMetrics; MAX_VALIDATORS],
    ) -> Result<(), TxError> {
        let count = usize::try_from(self.divergence_update_count)
            .map_err(|_| TxError::MalformedEnvelope)?;
        if count > MAX_VALIDATORS {
            return Err(TxError::MalformedEnvelope);
        }
        for i in 0..count {
            let target = usize::try_from(self.divergence_targets[i])
                .map_err(|_| TxError::MalformedEnvelope)?;
            if target >= MAX_VALIDATORS {
                return Err(TxError::MalformedEnvelope);
            }
            validators[target].divergence = self.divergence_values[i];
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// prevalidate_all: decode -> sort -> validate against projected nonces
// ---------------------------------------------------------------------------

/// Per-entry for sorting. `author_slot` is pre-resolved in pass 1 to avoid a
/// second O(N) `index_of_validator` scan in the admission pass. `raw_idx`
/// retains the position so pass 2 can read nonce and type with cheap
/// fixed-offset reads (no full re-validation; all checks done in pass 1).
#[derive(Clone, Copy)]
struct CandidateTx {
    key: [u8; 32],
    id: [u8; 32],
    raw_idx: u32,
    author_slot: u32,
}

impl CandidateTx {
    const ZERO: CandidateTx = CandidateTx {
        key: [0u8; 32],
        id: [0u8; 32],
        raw_idx: 0,
        author_slot: 0,
    };
}

fn candidate_after(left: &CandidateTx, right: &CandidateTx) -> bool {
    left.key > right.key || (left.key == right.key && left.id > right.id)
}

/// Prevalidate all transactions in `raw_txs` without mutating `state`.
///
/// Steps:
/// 1. Parse each envelope (skip malformed).
/// 2. Compute sort keys and sort by key (insertion sort).
/// 3. In sorted order, check admissibility against a local nonce projection.
/// 4. Compute each accepted author's next nonce with `checked_add`; stop at
///    `max_count`.
///
/// Malformed or inadmissible transactions are filtered out. A nonce overflow
/// is returned as an error because the next state cannot represent it.
pub fn prevalidate_all(
    state: &EpochState,
    raw_txs: &[&[u8]],
    max_count: u32,
) -> Result<TxPrevalidation, TxError> {
    const MAX_TX_PER_EPOCH: usize = 1024;

    let n = if raw_txs.len() > MAX_TX_PER_EPOCH {
        MAX_TX_PER_EPOCH
    } else {
        raw_txs.len()
    };

    let mut entries = [CandidateTx::ZERO; MAX_TX_PER_EPOCH];
    let mut valid: usize = 0;

    // Pass 1: parse each envelope, resolve author slot, build CandidateTx entries.
    // Caching `author_slot` avoids a second O(N) `index_of_validator` scan in
    // the admission pass. `raw_idx` is retained for cheap field reads in pass 2.
    for (raw_idx, raw) in raw_txs.iter().enumerate().take(n) {
        let (tx, consumed) = match parse_tx(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let slot = match index_of_validator(state, tx.author_id()) {
            Some(s) => s,
            None => continue,
        };
        let id = match tx_id_bytes(raw, consumed) {
            Ok(id) => id,
            Err(_) => continue,
        };
        let key = sort_key(&state.entropy_seed, &id);
        entries[valid] = CandidateTx {
            key,
            id,
            raw_idx: raw_idx as u32,
            author_slot: slot as u32,
        };
        valid += 1;
    }

    // Insertion sort (deterministic, constant-size), ordered by (sort_key, tx_id).
    let mut i: usize = 1;
    while i < valid {
        let mut j = i;
        while j > 0 && candidate_after(&entries[j - 1], &entries[j]) {
            entries.swap(j - 1, j);
            j -= 1;
        }
        i += 1;
    }

    let limit = if (max_count as usize) < valid {
        max_count as usize
    } else {
        valid
    };
    let mut next_nonces = state.nonces;
    let mut projected_divergences = [FixedPoint::ZERO; MAX_VALIDATORS];
    for (dst, validator) in projected_divergences[..state.validator_count as usize]
        .iter_mut()
        .zip(state.validators[..state.validator_count as usize].iter())
    {
        *dst = validator.divergence;
    }
    let mut divergence_targets = [0u32; MAX_VALIDATORS];
    let mut divergence_values = [FixedPoint::ZERO; MAX_VALIDATORS];
    let mut divergence_update_count: u32 = 0;
    let mut applied: u32 = 0;

    // Pass 2: admit in sorted order.
    // Reads nonce and type from fixed envelope offsets — no full re-parse.
    // All validity checks (version, author_id, payload_len, envelope size) were
    // already performed in pass 1; `raw_idx` points to a known-valid slice.
    for e in &entries[..valid] {
        if applied as usize >= limit {
            break;
        }
        let raw = raw_txs[e.raw_idx as usize];
        let mut nonce_b = [0u8; 8];
        nonce_b.copy_from_slice(&raw[4..12]);
        let nonce = u64::from_le_bytes(nonce_b);
        let idx = e.author_slot as usize;
        if check_nonce(&next_nonces, idx, nonce).is_err() {
            continue;
        }
        let mut typ_b = [0u8; 2];
        typ_b.copy_from_slice(&raw[2..4]);
        let tx_type = u16::from_le_bytes(typ_b);
        let applied_tx = match tx_type {
            TX_TYPE_NOOP => {
                next_nonces[idx] = next_nonces[idx]
                    .checked_add(1)
                    .ok_or(TxError::MalformedEnvelope)?;
                true
            }
            TX_TYPE_SCORE_DECREMENT => {
                let mut target_b = [0u8; 4];
                target_b.copy_from_slice(&raw[64..68]);
                let target_idx = u32::from_le_bytes(target_b);
                let mut delta_b = [0u8; 4];
                delta_b.copy_from_slice(&raw[68..72]);
                let delta = u32::from_le_bytes(delta_b);
                match apply_tx_1_cached(
                    &mut projected_divergences,
                    &mut next_nonces,
                    state.validator_count,
                    idx,
                    target_idx,
                    delta,
                ) {
                    Ok((tidx, new_divergence)) => {
                        let effect_idx = divergence_update_count as usize;
                        if effect_idx >= MAX_VALIDATORS {
                            return Err(TxError::BudgetExceeded);
                        }
                        divergence_targets[effect_idx] = tidx;
                        divergence_values[effect_idx] = new_divergence;
                        divergence_update_count += 1;
                        true
                    }
                    Err(_) => false,
                }
            }
            _ => continue, // unreachable: pass 1 accepted only known types
        };
        if applied_tx {
            applied += 1;
        }
    }

    Ok(TxPrevalidation {
        next_nonces,
        divergence_targets,
        divergence_values,
        divergence_update_count,
        applied_count: applied,
    })
}

/// Apply all transactions in `raw_txs` to `state`.
///
/// Delegates all fallible work to `prevalidate_all`, then commits the
/// already-computed nonce array with a single infallible assignment.
pub fn apply_all(
    state: &mut EpochState,
    raw_txs: &[&[u8]],
    max_count: u32,
) -> Result<u32, TxError> {
    let plan = prevalidate_all(state, raw_txs, max_count)?;
    state.nonces = plan.next_nonces;
    plan.apply_divergence_updates(&mut state.validators)?;
    Ok(plan.applied_count)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyapunov::{ConvergenceWindow, ValidatorMetrics};
    use crate::transition::{HaltReason, MAX_VALIDATORS};

    fn make_state(vc: u32) -> EpochState {
        let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
        for i in 0..vc as usize {
            validator_ids[i][0] = i as u8 + 1;
        }
        EpochState {
            epoch: 1,
            halt_reason: HaltReason::None,
            entropy_seed: [0u8; 32],
            validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
            validator_count: vc,
            convergence_window: ConvergenceWindow::new(),
            nonces: [0u64; MAX_VALIDATORS],
            validator_ids,
            cascade_health: 0,
            state_root: [0u8; 32],
            receipt_root: [0u8; 32],
            efb_root: [0u8; 32],
            causal_fingerprint: [0u8; 32],
        }
    }

    fn make_tx0_raw(author_id: [u8; 48], tx_sequence: u64) -> [u8; TX0_WIRE_BYTES] {
        let mut raw = [0u8; TX0_WIRE_BYTES];
        raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
        raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
        raw[4..12].copy_from_slice(&tx_sequence.to_le_bytes());
        raw[12..60].copy_from_slice(&author_id);
        raw[60..64].copy_from_slice(&0u32.to_le_bytes());
        raw
    }

    fn make_tx1_raw(
        author_id: [u8; 48],
        tx_sequence: u64,
        target_idx: u32,
        delta: u32,
    ) -> [u8; TX1_WIRE_BYTES] {
        let mut raw = [0u8; TX1_WIRE_BYTES];
        raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
        raw[2..4].copy_from_slice(&TX_TYPE_SCORE_DECREMENT.to_le_bytes());
        raw[4..12].copy_from_slice(&tx_sequence.to_le_bytes());
        raw[12..60].copy_from_slice(&author_id);
        raw[60..64].copy_from_slice(&(TX1_PAYLOAD_BYTES as u32).to_le_bytes());
        raw[64..68].copy_from_slice(&target_idx.to_le_bytes());
        raw[68..72].copy_from_slice(&delta.to_le_bytes());
        raw
    }

    fn author_id(slot: u8) -> [u8; 48] {
        let mut id = [0u8; 48];
        id[0] = slot + 1;
        id
    }

    fn sorted_before(a: &[u8; TX0_WIRE_BYTES], b: &[u8; TX0_WIRE_BYTES]) -> bool {
        sort_key(&[0u8; 32], &tx_id(a)) < sort_key(&[0u8; 32], &tx_id(b))
    }

    fn ordered_same_author_txs() -> ([u8; TX0_WIRE_BYTES], [u8; TX0_WIRE_BYTES]) {
        let tx0 = make_tx0_raw(author_id(0), 0);
        let mut tx1 = make_tx0_raw(author_id(0), 1);
        for b in 0u16..=u8::MAX as u16 {
            tx1[TX_HEADER_BYTES] = b as u8;
            if sorted_before(&tx0, &tx1) {
                return (tx0, tx1);
            }
        }
        panic!("could not construct ordered same-author transaction pair");
    }

    #[test]
    fn tx0_noop_advances_nonce() {
        let mut state = make_state(2);
        let raw = make_tx0_raw(author_id(0), 0);
        let (tx, _) = parse_tx0(&raw).unwrap();
        let idx = is_admissible(&state, &tx).unwrap();
        apply_tx_0(&mut state, idx).unwrap();
        assert_eq!(state.nonces[0], 1);
        assert_eq!(state.nonces[1], 0);
    }

    #[test]
    fn tx0_wrong_nonce_rejected() {
        let state = make_state(2);
        let raw = make_tx0_raw(author_id(0), 99);
        let (tx, _) = parse_tx0(&raw).unwrap();
        let err = is_admissible(&state, &tx).unwrap_err();
        assert_eq!(
            err,
            TxError::NonceMismatch {
                expected: 0,
                got: 99
            }
        );
    }

    #[test]
    fn tx0_unknown_author_rejected() {
        let state = make_state(2);
        let mut unknown_id = [0u8; 48];
        unknown_id[0] = 0xFF;
        let raw = make_tx0_raw(unknown_id, 0);
        let (tx, _) = parse_tx0(&raw).unwrap();
        let err = is_admissible(&state, &tx).unwrap_err();
        assert_eq!(err, TxError::AuthorNotFound);
    }

    #[test]
    fn parse_tx1_reads_target_and_delta() {
        let raw = make_tx1_raw(author_id(0), 7, 1, 25);
        let (tx, consumed) = parse_tx1(&raw).unwrap();
        assert_eq!(consumed, TX1_WIRE_BYTES);
        assert_eq!(tx.author_id, author_id(0));
        assert_eq!(tx.nonce, 7);
        assert_eq!(tx.target_idx, 1);
        assert_eq!(tx.delta, 25);
    }

    #[test]
    fn tx1_decrements_target_divergence_and_advances_author_nonce() {
        let mut state = make_state(2);
        state.validators[1].divergence = FixedPoint::from_raw(100);
        let raw = make_tx1_raw(author_id(0), 0, 1, 30);
        let (tx, _) = parse_tx1(&raw).unwrap();
        let author_idx = index_of_validator(&state, &tx.author_id).unwrap();

        apply_tx_1(&mut state, author_idx, &tx).unwrap();

        assert_eq!(state.nonces[0], 1);
        assert_eq!(state.nonces[1], 0);
        assert_eq!(state.validators[1].divergence.raw(), 70);
    }

    #[test]
    fn tx1_rejects_delta_above_current_divergence() {
        let mut state = make_state(2);
        state.validators[1].divergence = FixedPoint::from_raw(50);
        let raw = make_tx1_raw(author_id(0), 0, 1, 100);
        let (tx, _) = parse_tx1(&raw).unwrap();
        let author_idx = index_of_validator(&state, &tx.author_id).unwrap();

        assert_eq!(
            apply_tx_1(&mut state, author_idx, &tx).unwrap_err(),
            TxError::DeltaExceedsDivergence
        );
        assert_eq!(state.nonces[0], 0);
        assert_eq!(state.validators[1].divergence.raw(), 50);
    }

    #[test]
    fn apply_all_ordering_is_deterministic() {
        let mut s1 = make_state(4);
        let mut s2 = make_state(4);

        let tx_a = make_tx0_raw(author_id(0), 0);
        let tx_b = make_tx0_raw(author_id(1), 0);
        let tx_c = make_tx0_raw(author_id(2), 0);

        let txs_forward: &[&[u8]] = &[tx_a.as_slice(), tx_b.as_slice(), tx_c.as_slice()];
        let txs_reverse: &[&[u8]] = &[tx_c.as_slice(), tx_b.as_slice(), tx_a.as_slice()];

        let n1 = apply_all(&mut s1, txs_forward, 100).unwrap();
        let n2 = apply_all(&mut s2, txs_reverse, 100).unwrap();

        assert_eq!(n1, n2);
        assert_eq!(s1.nonces[0], s2.nonces[0]);
        assert_eq!(s1.nonces[1], s2.nonces[1]);
        assert_eq!(s1.nonces[2], s2.nonces[2]);
    }

    #[test]
    fn apply_all_accepts_tx1_and_commits_projected_validator_metrics() {
        let mut state = make_state(2);
        state.validators[1].divergence = FixedPoint::from_raw(100);
        let tx = make_tx1_raw(author_id(0), 0, 1, 100);

        let applied = apply_all(&mut state, &[tx.as_slice()], 100).unwrap();

        assert_eq!(applied, 1);
        assert_eq!(state.nonces[0], 1);
        assert_eq!(state.validators[1].divergence.raw(), 0);
    }

    #[test]
    fn apply_all_filters_inadmissible_tx1_without_mutation() {
        let mut state = make_state(2);
        state.validators[1].divergence = FixedPoint::from_raw(50);
        let tx = make_tx1_raw(author_id(0), 0, 1, 100);

        let applied = apply_all(&mut state, &[tx.as_slice()], 100).unwrap();

        assert_eq!(applied, 0);
        assert_eq!(state.nonces[0], 0);
        assert_eq!(state.validators[1].divergence.raw(), 50);
    }

    #[test]
    fn parse_tx0_invalid_version_rejected() {
        let mut raw = make_tx0_raw(author_id(0), 0);
        raw[0] = 0xFF;
        raw[1] = 0xFF;
        assert_eq!(parse_tx0(&raw).unwrap_err(), TxError::InvalidVersion);
    }

    #[test]
    fn parse_tx0_nonzero_payload_rejected() {
        let mut raw = make_tx0_raw(author_id(0), 0);
        raw[60..64].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(parse_tx0(&raw).unwrap_err(), TxError::MalformedEnvelope);
    }

    #[test]
    fn apply_all_idempotent_with_empty_txs() {
        let mut s1 = make_state(3);
        let mut s2 = make_state(3);
        apply_all(&mut s1, &[], 100).unwrap();
        apply_all(&mut s2, &[], 100).unwrap();
        assert_eq!(s1.nonces[0], s2.nonces[0]);
        assert_eq!(s1.nonces[1], s2.nonces[1]);
        assert_eq!(s1.nonces[2], s2.nonces[2]);
    }

    #[test]
    fn prevalidate_all_accepts_multiple_sorted_txs_from_same_validator() {
        let state = make_state(2);
        let (tx0, tx1) = ordered_same_author_txs();
        let plan = prevalidate_all(&state, &[tx0.as_slice(), tx1.as_slice()], 100).unwrap();

        assert_eq!(plan.applied_count, 2);
        assert_eq!(plan.next_nonces[0], 2);
        assert_eq!(state.nonces[0], 0, "prevalidation must not mutate state");
    }

    #[test]
    fn prevalidate_all_reports_nonce_overflow_without_mutating_state() {
        let mut state = make_state(1);
        state.nonces[0] = u64::MAX;
        let tx = make_tx0_raw(author_id(0), u64::MAX);

        assert_eq!(
            prevalidate_all(&state, &[tx.as_slice()], 100).unwrap_err(),
            TxError::MalformedEnvelope
        );
        assert_eq!(state.nonces[0], u64::MAX);
    }

    #[test]
    fn prevalidate_all_replayed_nonce_is_rejected_without_advancing_projection() {
        let mut state = make_state(1);
        state.nonces[0] = 1;
        let replay = make_tx0_raw(author_id(0), 0);

        let plan = prevalidate_all(&state, &[replay.as_slice()], 100).unwrap();
        assert_eq!(plan.applied_count, 0);
        assert_eq!(plan.next_nonces[0], 1);
        assert_eq!(state.nonces[0], 1, "prevalidation must not mutate state");
    }

    #[test]
    fn prevalidate_all_out_of_order_same_author_sequence_converges_deterministically() {
        let state = make_state(1);
        let (tx0, tx1) = ordered_same_author_txs();

        let forward = prevalidate_all(&state, &[tx0.as_slice(), tx1.as_slice()], 100).unwrap();
        let reverse = prevalidate_all(&state, &[tx1.as_slice(), tx0.as_slice()], 100).unwrap();

        assert_eq!(forward.applied_count, 2);
        assert_eq!(reverse.applied_count, 2);
        assert_eq!(forward.next_nonces, reverse.next_nonces);
        assert_eq!(forward.next_nonces[0], 2);
    }

    #[test]
    fn apply_all_conflicting_same_nonce_transactions_deduplicate_by_nonce_progression() {
        let mut s1 = make_state(1);
        let mut s2 = make_state(1);

        let tx_a = make_tx0_raw(author_id(0), 0);
        let mut tx_b = make_tx0_raw(author_id(0), 0);
        tx_b[TX_HEADER_BYTES] = 1; // signature differs -> distinct tx_id/sort key

        let n1 = apply_all(&mut s1, &[tx_a.as_slice(), tx_b.as_slice()], 100).unwrap();
        let n2 = apply_all(&mut s2, &[tx_b.as_slice(), tx_a.as_slice()], 100).unwrap();

        assert_eq!(n1, 1, "only one conflicting nonce-0 tx may apply");
        assert_eq!(n2, 1, "only one conflicting nonce-0 tx may apply");
        assert_eq!(s1.nonces[0], 1);
        assert_eq!(s2.nonces[0], 1);
    }

    #[test]
    fn candidate_total_order_breaks_equal_sort_key_by_tx_id() {
        let mut low_id = [0u8; 32];
        low_id[31] = 1;
        let mut high_id = [0u8; 32];
        high_id[31] = 2;

        let high_first = CandidateTx {
            key: [7u8; 32],
            id: high_id,
            raw_idx: 0,
            author_slot: 0,
        };
        let low_second = CandidateTx {
            key: [7u8; 32],
            id: low_id,
            raw_idx: 1,
            author_slot: 0,
        };

        assert!(candidate_after(&high_first, &low_second));
        assert!(!candidate_after(&low_second, &high_first));
    }

    #[test]
    fn apply_tx_0_rejects_out_of_range_slot_index() {
        let mut state = make_state(1);
        let err = apply_tx_0(&mut state, MAX_VALIDATORS).unwrap_err();
        assert_eq!(err, TxError::MalformedEnvelope);
        assert_eq!(state.nonces[0], 0);
    }
}
