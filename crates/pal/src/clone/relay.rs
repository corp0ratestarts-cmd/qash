// Store-and-forward relay buffer for clone protocol.
//
// GENESIS_CONSTANTS.toml [clone_protocol]: store_and_forward = true
//
// Purpose: relay nodes buffer chunks for peers that are temporarily absent.
// Chunks are held until the peer reconnects or until the buffer expires after
// MAX_OFFLINE_EPOCHS epochs (matching the cascade proof staleness limit).
//
// Domain B only.  The relay buffer does not influence Domain A state; it is
// a transport-layer concern.
//
// Eviction policy:
//   - Per-destination FIFO queue capped at MAX_BUFFERED_CHUNKS.
//   - Chunks whose epoch_age (caller-supplied) exceeds MAX_EPOCH_AGE are
//     evicted on any access (lazy expiry).
//   - When the queue is full, the oldest chunk is evicted to make room
//     (tail-drop oldest).

/// Maximum consecutive offline epochs before a chunk proof is considered stale.
/// Mirrors MAX_OFFLINE_EPOCHS from src/offline/clone.rs (genesis constant).
pub const MAX_EPOCH_AGE: u8 = 12;

/// Maximum chunks held per destination before oldest are evicted.
pub const MAX_BUFFERED_CHUNKS: usize = 64;

/// A buffered chunk entry held by the relay.
#[derive(Clone, Debug)]
pub struct BufferedChunk {
    /// Epoch in which the chunk was produced.
    pub chunk_epoch: u64,
    /// Serialised chunk bytes (caller-owned; relay treats as opaque).
    pub bytes: Vec<u8>,
}

/// Error type for relay buffer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayError {
    /// Chunk's epoch_age exceeds MAX_EPOCH_AGE and was not buffered.
    ChunkTooOld,
}

/// Per-destination store-and-forward buffer.
///
/// Keyed by destination identity (caller uses any byte key, e.g. validator ID).
/// Not thread-safe — wrap in a mutex if shared across threads.
pub struct StoreForwardBuffer {
    queues: std::collections::BTreeMap<Vec<u8>, std::collections::VecDeque<BufferedChunk>>,
}

impl StoreForwardBuffer {
    pub fn new() -> Self {
        Self { queues: std::collections::BTreeMap::new() }
    }

    /// Buffer `chunk` for `destination`.
    ///
    /// `current_epoch` is the epoch at the relay node now. Rejects chunks
    /// whose age (`current_epoch - chunk.chunk_epoch`) exceeds MAX_EPOCH_AGE.
    /// If the queue is full, the oldest chunk is evicted to make room.
    pub fn enqueue(
        &mut self,
        destination: &[u8],
        chunk: BufferedChunk,
        current_epoch: u64,
    ) -> Result<(), RelayError> {
        let age = current_epoch.saturating_sub(chunk.chunk_epoch);
        if age > u64::from(MAX_EPOCH_AGE) {
            return Err(RelayError::ChunkTooOld);
        }
        let queue = self.queues.entry(destination.to_vec()).or_default();
        if queue.len() >= MAX_BUFFERED_CHUNKS {
            queue.pop_front(); // evict oldest
        }
        queue.push_back(chunk);
        Ok(())
    }

    /// Drain all buffered chunks for `destination` that are still within
    /// `MAX_EPOCH_AGE` of `current_epoch`.  Expired chunks are silently dropped.
    pub fn drain(&mut self, destination: &[u8], current_epoch: u64) -> Vec<BufferedChunk> {
        let Some(queue) = self.queues.get_mut(destination) else {
            return vec![];
        };
        let mut out = Vec::new();
        while let Some(front) = queue.front() {
            let age = current_epoch.saturating_sub(front.chunk_epoch);
            if age > u64::from(MAX_EPOCH_AGE) {
                queue.pop_front(); // expired
            } else {
                break;
            }
        }
        // Move all remaining (non-expired) chunks out.
        out.extend(queue.drain(..));
        // Queue is now empty — remove the entry to reclaim memory.
        self.queues.remove(destination);
        out
    }

    /// Number of destinations with buffered chunks.
    pub fn destination_count(&self) -> usize {
        self.queues.len()
    }

    /// Total chunks buffered across all destinations.
    pub fn total_buffered(&self) -> usize {
        self.queues.values().map(|q| q.len()).sum()
    }

    /// Evict all chunks older than `MAX_EPOCH_AGE` epochs.  Call periodically
    /// (e.g. once per epoch) to reclaim memory.
    pub fn evict_expired(&mut self, current_epoch: u64) {
        self.queues.retain(|_, queue| {
            queue.retain(|c| {
                current_epoch.saturating_sub(c.chunk_epoch) <= u64::from(MAX_EPOCH_AGE)
            });
            !queue.is_empty()
        });
    }
}

impl Default for StoreForwardBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(epoch: u64) -> BufferedChunk {
        BufferedChunk { chunk_epoch: epoch, bytes: vec![epoch as u8; 8] }
    }

    #[test]
    fn enqueue_and_drain_roundtrip() {
        let mut buf = StoreForwardBuffer::new();
        buf.enqueue(b"peer-a", chunk(10), 10).unwrap();
        buf.enqueue(b"peer-a", chunk(10), 10).unwrap();
        let drained = buf.drain(b"peer-a", 10);
        assert_eq!(drained.len(), 2);
        assert_eq!(buf.total_buffered(), 0);
    }

    #[test]
    fn rejects_too_old_chunk() {
        let mut buf = StoreForwardBuffer::new();
        let err = buf.enqueue(b"peer-a", chunk(0), u64::from(MAX_EPOCH_AGE) + 1);
        assert_eq!(err, Err(RelayError::ChunkTooOld));
    }

    #[test]
    fn drain_drops_expired_chunks() {
        let mut buf = StoreForwardBuffer::new();
        buf.enqueue(b"peer-a", chunk(0), 5).unwrap();
        // By epoch 20, chunk(0) is age 20 > MAX_EPOCH_AGE.
        let drained = buf.drain(b"peer-a", 20);
        assert!(drained.is_empty());
    }

    #[test]
    fn evicts_oldest_when_full() {
        let mut buf = StoreForwardBuffer::new();
        for i in 0..MAX_BUFFERED_CHUNKS {
            buf.enqueue(b"peer-x", chunk(i as u64), i as u64).unwrap();
        }
        assert_eq!(buf.total_buffered(), MAX_BUFFERED_CHUNKS);
        // One more should evict the oldest.
        buf.enqueue(b"peer-x", chunk(MAX_BUFFERED_CHUNKS as u64), MAX_BUFFERED_CHUNKS as u64).unwrap();
        assert_eq!(buf.total_buffered(), MAX_BUFFERED_CHUNKS);
    }

    #[test]
    fn drain_empty_destination_returns_empty() {
        let mut buf = StoreForwardBuffer::new();
        assert!(buf.drain(b"unknown", 0).is_empty());
    }

    #[test]
    fn evict_expired_reclaims_memory() {
        let mut buf = StoreForwardBuffer::new();
        buf.enqueue(b"peer-a", chunk(0), 0).unwrap();
        buf.enqueue(b"peer-b", chunk(1), 1).unwrap();
        buf.evict_expired(u64::from(MAX_EPOCH_AGE) + 2);
        assert_eq!(buf.total_buffered(), 0);
        assert_eq!(buf.destination_count(), 0);
    }

    #[test]
    fn destination_count_tracks_active_queues() {
        let mut buf = StoreForwardBuffer::new();
        buf.enqueue(b"a", chunk(0), 0).unwrap();
        buf.enqueue(b"b", chunk(0), 0).unwrap();
        assert_eq!(buf.destination_count(), 2);
        buf.drain(b"a", 0);
        assert_eq!(buf.destination_count(), 1);
    }
}
