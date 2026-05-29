// Bloom-filter chunk deduplication for clone relay nodes.
//
// GENESIS_CONSTANTS.toml [clone_protocol]: bloom_filter_dedup = true
//
// Purpose: relay nodes use this filter to avoid re-forwarding chunks that have
// already passed through. False positives cause an occasional missed relay
// (acceptable); false negatives never occur (sound).
//
// Implementation: a bit-array bloom filter with k=7 SHA3-256-based hash
// functions, parameterised by a 1-byte domain tag per slot. Size is chosen
// to achieve <1% FPR at capacity of 65536 chunks (per epoch window).
//
// Domain B only — never influences Domain A state.

use sha3::{Digest, Sha3_256};

/// Number of hash functions (k). Optimal for the chosen bit-to-element ratio.
const K: usize = 7;

/// Bit-array size in bits: 2^20 = 1048576 bits (128 KiB).
/// At n=65536, k=7: FPR ≈ (1 − e^(−7·65536/1048576))^7 ≈ 0.13% < 1%.
const BITS: usize = 1 << 20;
const BYTES: usize = BITS / 8;

/// Domain separation prefix for bloom filter hash slots.
const BLOOM_DOMAIN: &[u8] = b"QASH/clone/bloom/v1\0";

/// Bloom filter for clone chunk deduplication.
///
/// `seen()` returns `true` (and inserts) iff the key was NOT previously seen.
/// After `clear()` the filter is empty and accepts all keys again.
pub struct ChunkRelayFilter {
    bits: Box<[u8; BYTES]>,
    insert_count: u64,
}

impl ChunkRelayFilter {
    pub fn new() -> Self {
        Self {
            bits: Box::new([0u8; BYTES]),
            insert_count: 0,
        }
    }

    /// Returns `true` if `key` is definitely new (not in filter) and inserts it.
    /// Returns `false` if `key` was already present (possible false positive).
    pub fn seen(&mut self, key: &[u8]) -> bool {
        let indices = Self::hash_indices(key);
        let already_present = indices.iter().all(|&i| self.get_bit(i));
        if !already_present {
            for i in indices {
                self.set_bit(i);
            }
            self.insert_count += 1;
        }
        !already_present
    }

    /// True if `key` may have been seen before (possible false positive).
    pub fn may_have_seen(&self, key: &[u8]) -> bool {
        Self::hash_indices(key).iter().all(|&i| self.get_bit(i))
    }

    /// Reset the filter to empty.
    pub fn clear(&mut self) {
        self.bits.fill(0);
        self.insert_count = 0;
    }

    /// Approximate number of distinct keys inserted since last clear.
    pub fn insert_count(&self) -> u64 {
        self.insert_count
    }

    fn get_bit(&self, idx: usize) -> bool {
        let byte = idx / 8;
        let bit = idx % 8;
        (self.bits[byte] >> bit) & 1 == 1
    }

    fn set_bit(&mut self, idx: usize) {
        let byte = idx / 8;
        let bit = idx % 8;
        self.bits[byte] |= 1 << bit;
    }

    fn hash_indices(key: &[u8]) -> [usize; K] {
        let mut out = [0usize; K];
        for (slot, entry) in out.iter_mut().enumerate() {
            let mut h = Sha3_256::new();
            h.update(BLOOM_DOMAIN);
            h.update([slot as u8]);
            h.update(key);
            let digest = h.finalize();
            // Use first 4 bytes to index into BITS.
            let raw = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) as usize;
            *entry = raw % BITS;
        }
        out
    }
}

impl Default for ChunkRelayFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_key_is_accepted() {
        let mut f = ChunkRelayFilter::new();
        assert!(f.seen(b"chunk-abc-1"));
    }

    #[test]
    fn repeated_key_is_rejected() {
        let mut f = ChunkRelayFilter::new();
        assert!(f.seen(b"chunk-dup"));
        assert!(!f.seen(b"chunk-dup"));
    }

    #[test]
    fn distinct_keys_all_accepted() {
        let mut f = ChunkRelayFilter::new();
        for i in 0u64..100 {
            let key = i.to_le_bytes();
            assert!(f.seen(&key), "key {i} should be new");
        }
        assert_eq!(f.insert_count(), 100);
    }

    #[test]
    fn clear_resets_filter() {
        let mut f = ChunkRelayFilter::new();
        f.seen(b"x");
        assert!(!f.seen(b"x"));
        f.clear();
        assert!(f.seen(b"x"), "key should be new after clear");
        assert_eq!(f.insert_count(), 1);
    }

    #[test]
    fn may_have_seen_consistent_with_seen() {
        let mut f = ChunkRelayFilter::new();
        assert!(!f.may_have_seen(b"never-inserted"));
        f.seen(b"inserted");
        assert!(f.may_have_seen(b"inserted"));
    }

    #[test]
    fn false_positive_rate_under_one_percent() {
        // Insert 65536 keys, check 10000 fresh keys for false positives.
        let mut f = ChunkRelayFilter::new();
        for i in 0u64..65536 {
            f.seen(&i.to_le_bytes());
        }
        let mut fp = 0u32;
        for i in 65536u64..75536 {
            if f.may_have_seen(&i.to_le_bytes()) {
                fp += 1;
            }
        }
        let fpr = fp as f64 / 10000.0;
        assert!(fpr < 0.01, "false positive rate {fpr:.4} >= 1%");
    }
}
