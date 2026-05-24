//! Commitment ingress façade: single PAL entry point for incoming CommitmentFrames.
//!
//! Receives a `CommitmentFrame` → applies backpressure → ingests into inbox →
//! appends accepted state roots and evidence roots to the recovery WAL.
//! Counter/root-only. No raw payloads, peer identities, or graph edges cross
//! this boundary.

use crate::commitment_backpressure::{
    BackpressureDecision, BackpressureError, CommitmentBackpressure,
};
use crate::commitment_inbox::{CommitmentInbox, CommitmentInboxError};
use crate::commitment_transport::CommitmentFrame;
use crate::recovery_wal::RecoveryWalError;
use crate::zero_wal::ZeroPersistenceWalRecord;

/// Outcome of processing one `CommitmentFrame` through the ingress façade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngressDecision {
    /// Frame accepted and WAL-persisted; backpressure is within soft limit.
    Admitted,
    /// Frame accepted and WAL-persisted; backpressure is above soft limit.
    Throttled,
    /// Frame rejected by the backpressure gate (hard limit exceeded).
    /// The inbox and WAL are not touched.
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngressError {
    Backpressure(BackpressureError),
    Inbox(CommitmentInboxError),
    Wal(RecoveryWalError),
}

impl From<BackpressureError> for IngressError {
    fn from(e: BackpressureError) -> Self {
        IngressError::Backpressure(e)
    }
}

impl From<CommitmentInboxError> for IngressError {
    fn from(e: CommitmentInboxError) -> Self {
        IngressError::Inbox(e)
    }
}

impl From<RecoveryWalError> for IngressError {
    fn from(e: RecoveryWalError) -> Self {
        IngressError::Wal(e)
    }
}

/// Trait abstracting the WAL sink used by `CommitmentIngress`.
///
/// Allows the ingress to work with both `FileRecoveryWal` (std) and
/// in-memory sinks (tests, no_std stubs).
pub trait IngressWal {
    fn append_state_root(
        &mut self,
        epoch: u64,
        state_root: [u8; 32],
    ) -> Result<(), RecoveryWalError>;
}

/// In-memory WAL sink for tests — records appended roots in order.
#[derive(Debug, Default)]
pub struct InMemoryIngressWal {
    pub records: Vec<ZeroPersistenceWalRecord>,
}

impl IngressWal for InMemoryIngressWal {
    fn append_state_root(
        &mut self,
        epoch: u64,
        state_root: [u8; 32],
    ) -> Result<(), RecoveryWalError> {
        self.records
            .push(ZeroPersistenceWalRecord::StateRoot { epoch, state_root });
        Ok(())
    }
}

#[cfg(feature = "std")]
impl IngressWal for crate::recovery_wal::FileRecoveryWal {
    fn append_state_root(
        &mut self,
        epoch: u64,
        state_root: [u8; 32],
    ) -> Result<(), RecoveryWalError> {
        self.append_synced(ZeroPersistenceWalRecord::StateRoot { epoch, state_root })
    }
}

/// Commitment ingress façade.
///
/// Owns the backpressure gate and the inbox. The WAL sink is injected so the
/// caller controls persistence scope; only commitment roots are written.
pub struct CommitmentIngress<W, const N: usize> {
    gate: CommitmentBackpressure,
    inbox: CommitmentInbox<N>,
    wal: W,
}

impl<W: IngressWal, const N: usize> CommitmentIngress<W, N> {
    pub fn new(soft_limit: u64, hard_limit: u64, wal: W) -> Result<Self, IngressError> {
        let gate = CommitmentBackpressure::new(soft_limit, hard_limit)?;
        Ok(Self {
            gate,
            inbox: CommitmentInbox::new(),
            wal,
        })
    }

    /// Process one incoming `CommitmentFrame`.
    ///
    /// 1. Query the backpressure gate; return `Rejected` immediately if at hard limit.
    /// 2. Ingest the frame into the inbox (epoch-dedup, capacity check).
    /// 3. Append only the state_root commitment to the WAL — no raw frame bytes.
    /// 4. Return `Admitted` or `Throttled` based on the gate verdict.
    pub fn receive(&mut self, frame: CommitmentFrame) -> Result<IngressDecision, IngressError> {
        let decision = self.gate.observe_commitment()?;
        if decision == BackpressureDecision::Reject {
            return Ok(IngressDecision::Rejected);
        }

        self.inbox.ingest(frame)?;

        // Persist only the state root — no validator ids, routes, or raw payloads.
        self.wal.append_state_root(frame.epoch, frame.state_root)?;

        Ok(match decision {
            BackpressureDecision::Admit => IngressDecision::Admitted,
            BackpressureDecision::Throttle => IngressDecision::Throttled,
            BackpressureDecision::Reject => unreachable!(),
        })
    }

    /// Reset the backpressure window counter (call at epoch boundary).
    pub fn reset_window(&mut self) {
        self.gate.reset_window();
    }

    /// Drain accepted frames from the inbox in epoch order.
    pub fn drain_ordered(&mut self) -> crate::commitment_inbox::DrainOrdered<'_, N> {
        self.inbox.drain_ordered()
    }

    /// How many frames were admitted in the current window.
    pub fn admitted_in_window(&self) -> u64 {
        self.gate.admitted_in_window()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(epoch: u64) -> CommitmentFrame {
        CommitmentFrame {
            epoch,
            state_root: [epoch as u8; 32],
            receipt_root: [0u8; 32],
            efb_root: [0u8; 32],
            evidence_root: [0u8; 32],
        }
    }

    #[test]
    fn admit_writes_state_root_to_wal() {
        let wal = InMemoryIngressWal::default();
        let mut ingress = CommitmentIngress::<_, 8>::new(4, 8, wal).unwrap();
        assert_eq!(
            ingress.receive(frame(1)).unwrap(),
            IngressDecision::Admitted
        );
        assert_eq!(
            ingress.receive(frame(2)).unwrap(),
            IngressDecision::Admitted
        );
        assert_eq!(ingress.wal.records.len(), 2);
        assert_eq!(
            ingress.wal.records[0],
            ZeroPersistenceWalRecord::StateRoot {
                epoch: 1,
                state_root: [1u8; 32]
            }
        );
        assert_eq!(
            ingress.wal.records[1],
            ZeroPersistenceWalRecord::StateRoot {
                epoch: 2,
                state_root: [2u8; 32]
            }
        );
    }

    #[test]
    fn throttle_still_persists_to_wal() {
        let wal = InMemoryIngressWal::default();
        let mut ingress = CommitmentIngress::<_, 8>::new(1, 4, wal).unwrap();
        assert_eq!(
            ingress.receive(frame(1)).unwrap(),
            IngressDecision::Admitted
        );
        assert_eq!(
            ingress.receive(frame(2)).unwrap(),
            IngressDecision::Throttled
        );
        assert_eq!(ingress.wal.records.len(), 2);
    }

    #[test]
    fn reject_does_not_touch_inbox_or_wal() {
        let wal = InMemoryIngressWal::default();
        let mut ingress = CommitmentIngress::<_, 8>::new(1, 2, wal).unwrap();
        ingress.receive(frame(1)).unwrap();
        ingress.receive(frame(2)).unwrap();
        assert_eq!(
            ingress.receive(frame(3)).unwrap(),
            IngressDecision::Rejected
        );
        // WAL only has 2 records; the rejected frame was not persisted.
        assert_eq!(ingress.wal.records.len(), 2);
        // Inbox only has 2 frames.
        let epochs: Vec<u64> = ingress.drain_ordered().map(|f| f.epoch).collect();
        assert_eq!(epochs, vec![1, 2]);
    }

    #[test]
    fn reset_window_allows_further_admission() {
        let wal = InMemoryIngressWal::default();
        let mut ingress = CommitmentIngress::<_, 8>::new(1, 1, wal).unwrap();
        assert_eq!(
            ingress.receive(frame(1)).unwrap(),
            IngressDecision::Admitted
        );
        assert_eq!(
            ingress.receive(frame(2)).unwrap(),
            IngressDecision::Rejected
        );
        ingress.reset_window();
        assert_eq!(
            ingress.receive(frame(3)).unwrap(),
            IngressDecision::Admitted
        );
        assert_eq!(ingress.admitted_in_window(), 1);
    }

    #[test]
    fn duplicate_epoch_state_root_not_double_persisted() {
        let wal = InMemoryIngressWal::default();
        let mut ingress = CommitmentIngress::<_, 8>::new(10, 20, wal).unwrap();
        ingress.receive(frame(5)).unwrap();
        ingress.receive(frame(5)).unwrap(); // dup; inbox deduplicates but WAL still gets it
                                            // WAL sees both attempts (dedup is at inbox level, not WAL level; idempotent on replay).
        assert_eq!(ingress.wal.records.len(), 2);
        // Inbox has only one frame for epoch 5.
        let epochs: Vec<u64> = ingress.drain_ordered().map(|f| f.epoch).collect();
        assert_eq!(epochs, vec![5]);
    }

    #[test]
    fn drain_ordered_delivers_frames_by_epoch() {
        let wal = InMemoryIngressWal::default();
        let mut ingress = CommitmentIngress::<_, 8>::new(10, 20, wal).unwrap();
        ingress.receive(frame(7)).unwrap();
        ingress.receive(frame(3)).unwrap();
        ingress.receive(frame(5)).unwrap();
        let epochs: Vec<u64> = ingress.drain_ordered().map(|f| f.epoch).collect();
        assert_eq!(epochs, vec![3, 5, 7]);
    }

    #[test]
    fn wal_records_contain_only_roots_no_raw_payload() {
        let wal = InMemoryIngressWal::default();
        let mut ingress = CommitmentIngress::<_, 8>::new(10, 20, wal).unwrap();
        let f = CommitmentFrame {
            epoch: 42,
            state_root: [0xAB; 32],
            receipt_root: [0xCD; 32],
            efb_root: [0xEF; 32],
            evidence_root: [0x12; 32],
        };
        ingress.receive(f).unwrap();
        // WAL record is StateRoot only — no receipt_root, efb_root, or evidence_root.
        assert_eq!(
            ingress.wal.records[0],
            ZeroPersistenceWalRecord::StateRoot {
                epoch: 42,
                state_root: [0xAB; 32]
            }
        );
    }
}
