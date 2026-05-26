//! Faulty transport wrapper for integration testing.
//!
//! Wraps an inner `CommitmentTransport` with configurable fault injection:
//! - `drop_rate`: probability (0–255) that a sent frame is silently dropped
//! - `reorder`: buffer N frames before flushing in LIFO order (simulates reorder)
//!
//! Used exclusively in tests. Not for production use.

use std::collections::VecDeque;

use crate::commitment_transport::{CommitmentFrame, CommitmentFrameError, CommitmentTransport};

pub struct FaultyTransport<T: CommitmentTransport> {
    inner: T,
    drop_rate: u8,
    reorder_buf: VecDeque<CommitmentFrame>,
    reorder_window: usize,
    rng_state: u64,
}

impl<T: CommitmentTransport> FaultyTransport<T> {
    /// Create a faulty transport.
    ///
    /// - `drop_rate`: 0 = never drop; 255 = always drop
    /// - `reorder_window`: 0 = no reordering; N = buffer up to N frames before delivery
    pub fn new(inner: T, drop_rate: u8, reorder_window: usize) -> Self {
        Self {
            inner,
            drop_rate,
            reorder_buf: VecDeque::new(),
            reorder_window,
            // Deterministic seed — faulty transport must be reproducible in tests.
            rng_state: 0x1234_5678_9abc_def0,
        }
    }

    fn next_rand(&mut self) -> u8 {
        // xorshift64 — same as interpreter_conformance.rs
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        (x & 0xFF) as u8
    }
}

impl<T: CommitmentTransport<Error = CommitmentFrameError>> CommitmentTransport
    for FaultyTransport<T>
{
    type Error = CommitmentFrameError;

    fn send_commitment(&mut self, frame: CommitmentFrame) -> Result<(), CommitmentFrameError> {
        // Possibly drop.
        if self.next_rand() < self.drop_rate {
            return Ok(());
        }

        // Possibly buffer for reordering.
        if self.reorder_window > 0 {
            self.reorder_buf.push_back(frame);
            if self.reorder_buf.len() >= self.reorder_window {
                // Flush in reverse order (LIFO = maximum reorder stress).
                while let Some(f) = self.reorder_buf.pop_back() {
                    self.inner.send_commitment(f)?;
                }
            }
            return Ok(());
        }

        self.inner.send_commitment(frame)
    }

    fn recv_commitment(&mut self) -> Result<Option<CommitmentFrame>, CommitmentFrameError> {
        self.inner.recv_commitment()
    }
}

impl<T: CommitmentTransport<Error = CommitmentFrameError>> FaultyTransport<T> {
    /// Flush any buffered frames (draining the reorder buffer in LIFO order).
    pub fn flush_reorder_buf(&mut self) -> Result<(), CommitmentFrameError> {
        while let Some(f) = self.reorder_buf.pop_back() {
            self.inner.send_commitment(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commitment_transport::InMemoryCommitmentTransport;

    fn make_frame(epoch: u64) -> CommitmentFrame {
        CommitmentFrame {
            epoch,
            state_root: [epoch as u8; 32],
            receipt_root: [0u8; 32],
            efb_root: [0u8; 32],
            evidence_root: [0u8; 32],
        }
    }

    #[test]
    fn no_faults_delivers_all_frames() {
        let inner = InMemoryCommitmentTransport::new();
        let mut transport = FaultyTransport::new(inner, 0, 0);
        for i in 0..10 {
            transport.send_commitment(make_frame(i)).unwrap();
        }
        let mut received = Vec::new();
        while let Some(f) = transport.recv_commitment().unwrap() {
            received.push(f.epoch);
        }
        assert_eq!(received, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn always_drop_delivers_nothing() {
        let inner = InMemoryCommitmentTransport::new();
        let mut transport = FaultyTransport::new(inner, 255, 0);
        for i in 0..10 {
            transport.send_commitment(make_frame(i)).unwrap();
        }
        assert_eq!(transport.recv_commitment().unwrap(), None);
    }

    #[test]
    fn reorder_window_delivers_in_lifo_order() {
        let inner = InMemoryCommitmentTransport::new();
        let mut transport = FaultyTransport::new(inner, 0, 3);
        // Send 3 frames — triggers a flush after the 3rd.
        transport.send_commitment(make_frame(10)).unwrap();
        transport.send_commitment(make_frame(11)).unwrap();
        transport.send_commitment(make_frame(12)).unwrap();
        // LIFO: delivered as 12, 11, 10.
        let f1 = transport.recv_commitment().unwrap().unwrap();
        let f2 = transport.recv_commitment().unwrap().unwrap();
        let f3 = transport.recv_commitment().unwrap().unwrap();
        assert_eq!([f1.epoch, f2.epoch, f3.epoch], [12, 11, 10]);
    }

    #[test]
    fn flush_reorder_buf_drains_partial_window() {
        let inner = InMemoryCommitmentTransport::new();
        let mut transport = FaultyTransport::new(inner, 0, 5);
        // Send only 2 frames (below window threshold — nothing flushed yet).
        transport.send_commitment(make_frame(20)).unwrap();
        transport.send_commitment(make_frame(21)).unwrap();
        assert_eq!(transport.recv_commitment().unwrap(), None);
        // Explicit flush delivers them.
        transport.flush_reorder_buf().unwrap();
        let f1 = transport.recv_commitment().unwrap().unwrap();
        let f2 = transport.recv_commitment().unwrap().unwrap();
        assert_eq!([f1.epoch, f2.epoch], [21, 20]);
    }
}
