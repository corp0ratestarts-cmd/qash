//! Transaction processing for Domain A consensus.
//!
//! Only the nonce is checked and incremented here.

use crate::hash::{h_domain, DomainTag};
use crate::transition::{EpochState, MAX_VALIDATORS};

// ---------------------------------------------------------------------------
// Wire constants
// ---------------------------------------------------------------------------

/// Version field value for all Tx-0 transactions.
pub const TX_VERSION: u16 = 0x0001;

/// Type field value for Tx-0 (noop).
pub const TX_TYPE_NOOP: u16 = 0x0000;

/// Fixed byte size of a Tx-0 envelope on the wire.
pub const TX0_WIRE_BYTES: usize = 248;

/// Fixed header size (version + type + nonce + author_id + payload_len).
pub const TX_HEADER_BYTES: usize = 64;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced during Tx-0 processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxError {
    MalformedEnvelope,
    AuthorNotFound,
    NonceMismatch { expected: u64, got: u64 },
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a single Tx-0 envelope from `raw`.
///
/// Returns the parsed `Tx0` and bytes consumed on success.
/// Does not validate admissibility — call `is_admissible` separately.
pub fn parse_tx0(raw: &[u8]) -> Result<(Tx0<'_>, usize), TxError> {
    if raw.len() < TX0_WIRE_BYTES {
        return Err(TxError::MalformedEnvelope);
    }

    let version = u16::from_le_bytes([raw[0], raw[1]]);
    let tx_type = u16::from_le_bytes([raw[2], raw[3]]);

    if version != TX_VERSION || tx_type != TX_TYPE_NOOP {
        return Err(TxError::MalformedEnvelope);
    }

    let nonce = u64::from_le_bytes([
        raw[4], raw[5], raw[6], raw[7], raw[8], raw[9], raw[10], raw[11],
    ]);

    let author_id: &[u8; 48] = match raw[12..60].try_into() {
        Ok(v) => v,
        Err(_) => return Err(TxError::MalformedEnvelope),
    };

    let payload_len = u32::from_le_bytes([raw[60], raw[61], raw[62], raw[63]]);
    if payload_len != 0 {
        return Err(TxError::MalformedEnvelope);
    }

    let sig_arr: &[u8; 184] = match raw[64..248].try_into() {
        Ok(v) => v,
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

// ---------------------------------------------------------------------------
// Tx-0 wire struct
// ---------------------------------------------------------------------------

/// A parsed Tx-0 (noop) transaction.
#[derive(Debug)]
pub struct Tx0<'a> {
    pub author_id: &'a [u8; 48],
    pub nonce: u64,
    pub signature: &'a [u8; 184],
}

// ---------------------------------------------------------------------------
// Admission check
// ---------------------------------------------------------------------------

/// Check that `tx` is admissible in `state`.
///
/// Returns the validator slot index on success, or an error.
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

// ---------------------------------------------------------------------------
// Apply a single Tx-0
// ---------------------------------------------------------------------------

/// Apply a single Tx-0, incrementing the nonce for the given validator slot.
pub fn apply_tx_0(state: &mut EpochState, idx: usize) -> Result<(), TxError> {
    state.nonces[idx] = state.nonces[idx]
        .checked_add(1)
        .ok_or(TxError::MalformedEnvelope)?;
    Ok(())
}

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
// prevalidate_all: decode → sort → validate against projected nonces
// ---------------------------------------------------------------------------

/// Per-entry for sorting: sort key + index into raw_txs.
#[derive(Clone, Copy)]
struct SortEntry {
    key: [u8; 32],
    raw_idx: u32,
}

impl SortEntry {
    const ZERO: SortEntry = SortEntry {
        key: [0u8; 32],
        raw_idx: 0,
    };
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
/// Malformed or inadmissible transactions are filtered out, matching the
/// transaction-set semantics. A nonce overflow is returned as an error because
/// accepting a matching `u64::MAX` nonce cannot be represented by the next state.
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

    for (raw_idx, raw) in raw_txs[..n].iter().enumerate() {
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

        let mut arr = [0u8; TX0_WIRE_BYTES];
        arr.copy_from_slice(&raw[..TX0_WIRE_BYTES]);
        let id = tx_id(&arr);
        let key = sort_key(&state.entropy_seed, &id);

        entries[valid] = SortEntry {
            key,
            raw_idx: raw_idx as u32,
        };
        valid += 1;
    }

    // Insertion sort (stable, deterministic, constant-size).
    let mut i = 1;
    while i < valid {
        let x = entries[i];
        let mut j = i;
        while j > 0 && entries[j - 1].key > x.key {
            entries[j] = entries[j - 1];
            j -= 1;
        }
        entries[j] = x;
        i += 1;
    }

    let limit = if (max_count as usize) < valid {
        max_count as usize
    } else {
        valid
    };
    let mut next_nonces = state.nonces;
    let mut applied: u32 = 0;

    for e in &entries[..valid] {
        if (applied as usize) >= limit {
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

    Ok(TxPrevalidation {
        next_nonces,
        applied_count: applied,
    })
}

/// Apply all transactions in `raw_txs` to `state`.
///
/// This compatibility helper delegates all fallible work to `prevalidate_all`
/// and then commits the already-computed nonce array with a single assignment.
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
// Internal helpers
// ---------------------------------------------------------------------------

/// Look up the slot index of a validator by ID.
pub(crate) fn index_of_validator(state: &EpochState, id: &[u8; 48]) -> Option<usize> {
    for i in 0..state.validator_count as usize {
        if &state.validator_ids[i] == id {
            return Some(i);
        }
    }
    None
}

/// Compute the canonical transaction ID (SHA3-256 over the fixed-size wire bytes).
pub(crate) fn tx_id(raw: &[u8; TX0_WIRE_BYTES]) -> [u8; 32] {
    h_domain(DomainTag::TxId, raw.as_slice())
}

/// Derive a sort key from the epoch entropy seed and the transaction ID.
pub(crate) fn sort_key(entropy_seed: &[u8; 32], tx_id: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(entropy_seed);
    buf[32..].copy_from_slice(tx_id);
    h_domain(DomainTag::TxSortKey, &buf)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transition::{EpochState, ValidatorMetrics};
    use crate::fixed_point::FixedPoint;

    fn make_state(validator_count: u32) -> EpochState {
        let mut s = EpochState::ZERO;
        s.validator_count = validator_count;
        for i in 0..validator_count as usize {
            s.validator_ids[i][0] = i as u8 + 1;
        }
        s
    }

    fn make_tx0_raw(author_id: [u8; 48], nonce: u64) -> [u8; TX0_WIRE_BYTES] {
        let mut raw = [0u8; TX0_WIRE_BYTES];
        raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
        raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
        raw[4..12].copy_from_slice(&nonce.to_le_bytes());
        raw[12..60].copy_from_slice(&author_id);
        raw[60..64].copy_from_slice(&0u32.to_le_bytes()); // payload_len = 0
                                                          // signature bytes remain zero (opaque in Domain A)
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
        apply_all(&mut state, &[raw.as_slice()], 100).unwrap();
        assert_eq!(state.nonces[0], 1);
        assert_eq!(state.nonces[1], 0);
    }

    #[test]
    fn nonce_mismatch_rejected() {
        let state = make_state(1);
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
    fn unknown_author_rejected() {
        let state = make_state(1);
        let raw = make_tx0_raw(author_id(5), 0);
        let (tx, _) = parse_tx0(&raw).unwrap();
        assert_eq!(is_admissible(&state, &tx), Err(TxError::AuthorNotFound));
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
    fn parse_tx0_invalid_version_rejected() {
        let mut raw = make_tx0_raw(author_id(0), 0);
        raw[0] = 0xFF;
        assert_eq!(parse_tx0(&raw).unwrap_err(), TxError::MalformedEnvelope);
    }

    #[test]
    fn parse_tx0_nonzero_payload_len_rejected() {
        let mut raw = make_tx0_raw(author_id(0), 0);
        raw[60] = 1;
        assert_eq!(parse_tx0(&raw).unwrap_err(), TxError::MalformedEnvelope);
    }
}
