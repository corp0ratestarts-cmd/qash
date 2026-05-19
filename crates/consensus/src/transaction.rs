//! TX-0 (no-op) transaction pipeline for Domain A.
//!
//! Signature bytes are carried opaquely; verification is Domain B (PAL).
//! Only the nonce is checked and incremented here.

use crate::hash::{h_domain, DomainTag};
use crate::transition::{EpochState, MAX_VALIDATORS};

// ---------------------------------------------------------------------------
// Wire constants
// ---------------------------------------------------------------------------

/// Envelope version field value.
pub const TX_VERSION: u16 = 0x0001;
/// TX type for no-op.
pub const TX_TYPE_NOOP: u16 = 0x0000;
/// Dilithium5 signature size (opaque in Domain A).
pub const PQ_SIG_BYTES: usize = 2420;

/// Envelope header layout (64 bytes):
/// [version:2][tx_type:2][nonce:8][author_id:48][payload_len:4]
pub const TX_HEADER_BYTES: usize = 64;

/// Total wire size of a TX-0 envelope (no payload).
pub const TX0_WIRE_BYTES: usize = TX_HEADER_BYTES + PQ_SIG_BYTES;

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

    Ok((Tx0 { author_id, nonce, signature: sig_arr }, TX0_WIRE_BYTES))
}

// ---------------------------------------------------------------------------
// tx_id: canonical identifier (used for sort key computation)
// ---------------------------------------------------------------------------

/// tx_id = H_domain(TxId, raw_bytes[..TX0_WIRE_BYTES])
/// Commits to all envelope fields including the opaque signature.
pub fn tx_id(raw: &[u8; TX0_WIRE_BYTES]) -> [u8; 32] {
    h_domain(DomainTag::TxId, raw.as_slice())
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
        return Err(TxError::NonceMismatch { expected, got: tx.nonce });
    }
    Ok(idx)
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
    pub applied_count: u32,
}

// ---------------------------------------------------------------------------
// prevalidate_all: decode -> sort -> validate against projected nonces
// ---------------------------------------------------------------------------

/// Per-entry for sorting: sort key + index into raw_txs.
#[derive(Clone, Copy)]
struct SortEntry {
    key: [u8; 32],
    raw_idx: u32,
}

impl SortEntry {
    const ZERO: SortEntry = SortEntry { key: [0u8; 32], raw_idx: 0 };
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

    let mut entries = [SortEntry::ZERO; MAX_TX_PER_EPOCH];
    let mut valid: usize = 0;

    for (raw_idx, raw) in raw_txs.iter().enumerate().take(n) {
        let (tx, consumed) = match parse_tx0(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if consumed != TX0_WIRE_BYTES {
            continue;
        }
        if index_of_validator(state, &tx.author_id).is_none() {
            continue;
        }

        if raw.len() < TX0_WIRE_BYTES {
            continue;
        }
        let mut arr = [0u8; TX0_WIRE_BYTES];
        arr.copy_from_slice(&raw[..TX0_WIRE_BYTES]);
        let id = tx_id(&arr);
        let key = sort_key(&state.entropy_seed, &id);

        entries[valid] = SortEntry { key, raw_idx: raw_idx as u32 };
        valid += 1;
    }

    // Insertion sort (stable, deterministic, constant-size).
    let mut i: usize = 1;
    while i < valid {
        let mut j = i;
        while j > 0 && entries[j - 1].key > entries[j].key {
            entries.swap(j - 1, j);
            j -= 1;
        }
        i += 1;
    }

    let limit = if (max_count as usize) < valid { max_count as usize } else { valid };
    let mut next_nonces = state.nonces;
    let mut applied: u32 = 0;

    for e in &entries[..valid] {
        if applied as usize >= limit {
            break;
        }
        let raw = raw_txs[e.raw_idx as usize];
        let (tx, _) = match parse_tx0(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let idx = match index_of_validator(state, &tx.author_id) {
            Some(i) => i,
            None => continue,
        };
        let expected = next_nonces[idx];
        if tx.nonce != expected {
            continue;
        }
        next_nonces[idx] = next_nonces[idx]
            .checked_add(1)
            .ok_or(TxError::MalformedEnvelope)?;
        applied += 1;
    }

    Ok(TxPrevalidation { next_nonces, applied_count: applied })
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
            state_root: [0u8; 32],
        }
    }

    fn make_tx0_raw(author_id: [u8; 48], nonce: u64) -> [u8; TX0_WIRE_BYTES] {
        let mut raw = [0u8; TX0_WIRE_BYTES];
        raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
        raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
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
        assert_eq!(err, TxError::NonceMismatch { expected: 0, got: 99 });
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
    fn apply_tx_0_rejects_out_of_range_slot_index() {
        let mut state = make_state(1);
        let err = apply_tx_0(&mut state, MAX_VALIDATORS).unwrap_err();
        assert_eq!(err, TxError::MalformedEnvelope);
        assert_eq!(state.nonces[0], 0);
    }
}
