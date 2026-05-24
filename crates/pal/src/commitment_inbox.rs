//! Commitment-only inbox for delayed or reordered transport frames.
//!
//! The inbox stores only fixed-size commitment frames. It does not record sender
//! identity, routes, peer metadata, or transaction topology.

use crate::commitment_transport::CommitmentFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitmentInboxError {
    CapacityExceeded,
}

#[derive(Debug)]
pub struct CommitmentInbox<const N: usize> {
    len: usize,
    frames: [Option<CommitmentFrame>; N],
}

impl<const N: usize> CommitmentInbox<N> {
    pub fn new() -> Self {
        Self {
            len: 0,
            frames: [None; N],
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn ingest(&mut self, frame: CommitmentFrame) -> Result<(), CommitmentInboxError> {
        for idx in 0..self.len {
            if let Some(existing) = self.frames[idx] {
                if existing.epoch == frame.epoch && existing.state_root == frame.state_root {
                    return Ok(());
                }
            }
        }
        if self.len == N {
            return Err(CommitmentInboxError::CapacityExceeded);
        }
        self.frames[self.len] = Some(frame);
        self.len = self
            .len
            .checked_add(1)
            .ok_or(CommitmentInboxError::CapacityExceeded)?;
        Ok(())
    }

    pub fn drain_ordered(&mut self) -> DrainOrdered<'_, N> {
        self.sort_by_epoch();
        DrainOrdered { inbox: self, cursor: 0 }
    }

    fn sort_by_epoch(&mut self) {
        let mut i = 1usize;
        while i < self.len {
            let mut j = i;
            while j > 0 && epoch_of(self.frames[j - 1]) > epoch_of(self.frames[j]) {
                self.frames.swap(j - 1, j);
                match j.checked_sub(1) {
                    Some(next) => j = next,
                    None => break,
                }
            }
            match i.checked_add(1) {
                Some(next) => i = next,
                None => break,
            }
        }
    }
}

impl<const N: usize> Default for CommitmentInbox<N> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DrainOrdered<'a, const N: usize> {
    inbox: &'a mut CommitmentInbox<N>,
    cursor: usize,
}

impl<'a, const N: usize> Iterator for DrainOrdered<'a, N> {
    type Item = CommitmentFrame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.inbox.len {
            self.inbox.len = 0;
            return None;
        }
        let item = self.inbox.frames[self.cursor].take();
        self.cursor = match self.cursor.checked_add(1) {
            Some(next) => next,
            None => {
                self.inbox.len = 0;
                return None;
            }
        };
        item
    }
}

fn epoch_of(frame: Option<CommitmentFrame>) -> u64 {
    frame.map(|f| f.epoch).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(epoch: u64) -> CommitmentFrame {
        CommitmentFrame {
            epoch,
            state_root: [epoch as u8; 32],
            receipt_root: [1u8; 32],
            efb_root: [2u8; 32],
            evidence_root: [3u8; 32],
        }
    }

    #[test]
    fn drains_reordered_frames_by_epoch() {
        let mut inbox = CommitmentInbox::<4>::new();
        inbox.ingest(frame(3)).unwrap();
        inbox.ingest(frame(1)).unwrap();
        inbox.ingest(frame(2)).unwrap();
        let epochs: Vec<u64> = inbox.drain_ordered().map(|f| f.epoch).collect();
        assert_eq!(epochs, vec![1, 2, 3]);
    }

    #[test]
    fn drops_duplicate_epoch_state_root() {
        let mut inbox = CommitmentInbox::<4>::new();
        inbox.ingest(frame(1)).unwrap();
        inbox.ingest(frame(1)).unwrap();
        assert_eq!(inbox.len(), 1);
    }
}
