// Clone Protocol — CascadeBoundCloneChunk
//
// Spec: docs/spec/05_clone_protocol.md (normative)
//       docs/spec/07_hash_cascade.md "Cascade Proof Format" (proof structure)
//       GENESIS_CONSTANTS.toml [clone_protocol] (chunk_verification_mode = "cascade_bound")
//
// Domain B — offline / clone operations.  The cascade proof fields are
// Domain A values (deterministic) but the transport/serialisation layer
// is Domain B.  No Domain B nondeterminism may influence proof verification.

use sha3::{Digest, Sha3_256};

// ---------------------------------------------------------------------------
// Genesis-fixed constants (GENESIS_CONSTANTS.toml [clone_protocol])
// ---------------------------------------------------------------------------

/// Maximum consecutive offline hops before a relay must re-verify online.
pub const MAX_OFFLINE_HOPS: u8 = 7;

/// Maximum epochs a chunk may be in-flight before its proof is considered stale.
pub const MAX_OFFLINE_EPOCHS: u8 = 12;

/// Sparse Merkle tree depth (GENESIS_CONSTANTS.toml [obfuscation]).
pub const SPARSE_MERKLE_DEPTH: usize = 384;

/// Leaf index byte width (GENESIS_CONSTANTS.toml [obfuscation]).
pub const LEAF_INDEX_BYTES: usize = 48;

/// H_cascade output width in bytes.
pub const CASCADE_OUTPUT_BYTES: usize = 64;

// Domain separation tags for Merkle hashing (spec §3 / 07_hash_cascade.md).
const MERKLE_LEAF_DOMAIN: &[u8] = b"QASH/merkle/leaf/v1\0";
const MERKLE_INTERNAL_DOMAIN: &[u8] = b"QASH/merkle/internal/v1\0";

// ---------------------------------------------------------------------------
// Cascade proof format — spec 07_hash_cascade.md §3 "Cascade Proof Format"
// ---------------------------------------------------------------------------

/// Sparse-Merkle inclusion proof for one H_cascade output.
///
/// Sizes are fixed at genesis:
///   leaf_index_bytes    = 48  (GENESIS_CONSTANTS.toml [obfuscation])
///   cascade output      = 64  (H_cascade → [u8; 64])
///   sparse_merkle_depth = 384 (GENESIS_CONSTANTS.toml [obfuscation])
///
/// Proof verification (spec §3):
///   1. Recompute Merkle root from leaf_index, l7_output, and merkle_path
///      using H_domain(LEAF_HASH, …) / H_domain(INTERNAL_HASH, …).
///   2. Compare against the epoch's cascade_root from the epoch state.
#[derive(Clone, Debug)]
pub struct CascadeProof {
    /// Epoch-relative leaf index (first 48 bytes of H_cascade_keyed(seed_t, …)).
    pub leaf_index: [u8; LEAF_INDEX_BYTES],
    /// H_cascade output committed into the sparse Merkle tree.
    pub l7_output: [u8; CASCADE_OUTPUT_BYTES],
    /// Sibling hashes from leaf to root (depth = 384 nodes).
    /// Boxed to avoid 12 KiB on the stack.
    pub merkle_path: Box<[[u8; 32]; SPARSE_MERKLE_DEPTH]>,
}

impl CascadeProof {
    /// Tier 1 (online) verification: recompute the Merkle root from this proof
    /// and compare against `cascade_root` from live epoch state.
    ///
    /// Protocol (spec §3 / 07_hash_cascade.md):
    ///   leaf_hash      = SHA3-256("QASH/merkle/leaf/v1" || leaf_index || l7_output)
    ///   internal_hash  = SHA3-256("QASH/merkle/internal/v1" || left || right)
    ///   bit i of leaf_index selects left/right at depth i (LSB first)
    pub fn verify_merkle_root(&self, cascade_root: &[u8; 32]) -> bool {
        // Compute leaf hash: domain || leaf_index || l7_output
        let leaf_hash: [u8; 32] = {
            let mut h = Sha3_256::new();
            h.update(MERKLE_LEAF_DOMAIN);
            h.update(self.leaf_index);
            h.update(self.l7_output);
            h.finalize().into()
        };

        // Walk up the tree, using the leaf_index bits to determine sibling position.
        let mut current = leaf_hash;
        for (depth, sibling) in self.merkle_path.iter().enumerate() {
            // Byte and bit index within leaf_index for this depth level.
            let byte_idx = depth / 8;
            let bit_idx = depth % 8;
            let bit = if byte_idx < LEAF_INDEX_BYTES {
                (self.leaf_index[byte_idx] >> bit_idx) & 1
            } else {
                0
            };

            // bit == 0 → current is left child; bit == 1 → current is right child.
            let (left, right) = if bit == 0 {
                (&current, sibling)
            } else {
                (sibling, &current)
            };

            let mut h = Sha3_256::new();
            h.update(MERKLE_INTERNAL_DOMAIN);
            h.update(left);
            h.update(right);
            current = h.finalize().into();
        }

        &current == cascade_root
    }
}

// ---------------------------------------------------------------------------
// CascadeBoundCloneChunk — spec 05_clone_protocol.md §3
// ---------------------------------------------------------------------------

/// A self-authenticated clone chunk carrying a cascade inclusion proof.
///
/// Used when `chunk_verification_mode = "cascade_bound"` (GENESIS_CONSTANTS.toml
/// [clone_protocol]).  Each chunk is independently verifiable against the
/// epoch's cascade_root without trusting the transport channel.
///
/// Tiered verification (spec Appendix I interpretation):
///   - Tier 1 (online):  full Merkle path verification against live epoch state
///   - Tier 2 (offline): verify l7_output only (cached cascade root)
///   - Tier 3 (air-gap): verify chunk digest against QR/NFC payload checksum
#[derive(Clone, Debug)]
pub struct CascadeBoundCloneChunk {
    /// Sequential chunk index within the clone session.
    pub chunk_index: u32,
    /// Epoch in which this chunk was produced.
    pub epoch: u64,
    /// Raw chunk payload (serialised state slice, max size TBD in spec §3).
    pub payload: Vec<u8>,
    /// Cascade inclusion proof for this chunk's payload hash.
    pub proof: CascadeProof,
}

impl CascadeBoundCloneChunk {
    /// Compute the cascade input for a chunk (spec §3):
    ///   input = chunk_index_le4 ∥ epoch_le8 ∥ payload
    pub fn cascade_input(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + 8 + self.payload.len());
        buf.extend_from_slice(&self.chunk_index.to_le_bytes());
        buf.extend_from_slice(&self.epoch.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Tier 2 (offline) verification: check that proof.l7_output matches
    /// H_cascade_keyed(seed_t, cascade_input).
    ///
    /// `seed_t` is the entropy seed for `self.epoch` (01_consensus.md §1).
    /// Caller is responsible for supplying the correct seed.
    pub fn verify_l7(&self, seed_t: &[u8; 32]) -> bool {
        let input = self.cascade_input();
        let expected = crate::crypto::cascade::h_cascade_keyed(seed_t, &input);
        self.proof.l7_output == expected
    }

    /// Tier 1 (online) verification: verify l7_output AND the full Merkle
    /// inclusion proof against `cascade_root` from live epoch state.
    pub fn verify_tier1(&self, seed_t: &[u8; 32], cascade_root: &[u8; 32]) -> bool {
        self.verify_l7(seed_t) && self.proof.verify_merkle_root(cascade_root)
    }

    /// Tier 3 (air-gap) verification: SHA3-256 chunk digest for out-of-band
    /// comparison against a QR/NFC payload checksum.
    ///
    /// digest = SHA3-256("QASH/clone/chunk/v1" || chunk_index_le4 || epoch_le8 || payload)
    pub fn chunk_digest(&self) -> [u8; 32] {
        let mut h = Sha3_256::new();
        h.update(b"QASH/clone/chunk/v1\0");
        h.update(self.chunk_index.to_le_bytes());
        h.update(self.epoch.to_le_bytes());
        h.update(&self.payload);
        h.finalize().into()
    }
}

// ---------------------------------------------------------------------------
// Clone channel types — spec 05_clone_protocol.md §2
// ---------------------------------------------------------------------------

/// Transport channels supported by clone protocol v1.2.
/// Maps to GENESIS_CONSTANTS.toml [clone_protocol].channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloneChannel {
    QrCode,
    Nfc,
    Ble,
    WifiDirect,
    LoRa,
    Ultrasonic,
}

impl CloneChannel {
    /// Maximum payload bytes per chunk for this channel.
    ///
    /// QR: limited by QR capacity (~2KB), NFC/BLE/LoRa/Ultrasonic are lower.
    pub fn max_chunk_bytes(&self) -> usize {
        match self {
            CloneChannel::QrCode => 2048,
            CloneChannel::Nfc => 512,
            CloneChannel::Ble => 512,
            CloneChannel::WifiDirect => 65536,
            CloneChannel::LoRa => 255,
            CloneChannel::Ultrasonic => 128,
        }
    }

    /// True if this channel supports the BLE dual-role mode
    /// (GENESIS_CONSTANTS.toml: ble_mode = "dual").
    pub fn supports_dual_role(&self) -> bool {
        *self == CloneChannel::Ble
    }
}

// ---------------------------------------------------------------------------
// CloneHop — spec 05_clone_protocol.md §4 (multi-hop routing)
// ---------------------------------------------------------------------------

/// Error type for hop validation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneHopError {
    HopIndexExceedsMax,
    EpochOffsetExceedsMax,
}

/// Hop descriptor for offline multi-hop routing.
/// max_offline_hops = 7, max_offline_epochs = 12 (GENESIS_CONSTANTS.toml).
#[derive(Clone, Debug)]
pub struct CloneHop {
    pub channel: CloneChannel,
    /// 0-based hop index; must be < MAX_OFFLINE_HOPS (7).
    pub hop_index: u8,
    /// Epochs since chunk was produced; must be < MAX_OFFLINE_EPOCHS (12).
    pub epoch_offset: u8,
}

impl CloneHop {
    /// Create and validate a hop descriptor.
    pub fn new(
        channel: CloneChannel,
        hop_index: u8,
        epoch_offset: u8,
    ) -> Result<Self, CloneHopError> {
        if hop_index >= MAX_OFFLINE_HOPS {
            return Err(CloneHopError::HopIndexExceedsMax);
        }
        if epoch_offset >= MAX_OFFLINE_EPOCHS {
            return Err(CloneHopError::EpochOffsetExceedsMax);
        }
        Ok(Self { channel, hop_index, epoch_offset })
    }

    /// True if this is the final hop (hop_index == MAX_OFFLINE_HOPS - 1).
    pub fn is_terminal(&self) -> bool {
        self.hop_index == MAX_OFFLINE_HOPS - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(chunk_index: u32, epoch: u64, payload: &[u8]) -> CascadeBoundCloneChunk {
        CascadeBoundCloneChunk {
            chunk_index,
            epoch,
            payload: payload.to_vec(),
            proof: CascadeProof {
                leaf_index: [0u8; LEAF_INDEX_BYTES],
                l7_output: [0u8; CASCADE_OUTPUT_BYTES],
                merkle_path: Box::new([[0u8; 32]; SPARSE_MERKLE_DEPTH]),
            },
        }
    }

    #[test]
    fn cascade_input_encodes_index_epoch_payload() {
        let chunk = make_chunk(1, 42, b"hello");
        let input = chunk.cascade_input();
        assert_eq!(&input[0..4], &1u32.to_le_bytes());
        assert_eq!(&input[4..12], &42u64.to_le_bytes());
        assert_eq!(&input[12..], b"hello");
    }

    #[test]
    fn verify_l7_rejects_wrong_seed() {
        let mut chunk = make_chunk(0, 0, b"data");
        let seed = [0x01u8; 32];
        let expected = crate::crypto::cascade::h_cascade_keyed(&seed, &chunk.cascade_input());
        chunk.proof.l7_output = expected;

        assert!(chunk.verify_l7(&seed));
        let wrong_seed = [0x02u8; 32];
        assert!(!chunk.verify_l7(&wrong_seed));
    }

    #[test]
    fn chunk_digest_is_deterministic() {
        let chunk = make_chunk(7, 99, b"payload");
        assert_eq!(chunk.chunk_digest(), chunk.chunk_digest());
        assert_ne!(chunk.chunk_digest(), [0u8; 32]);
    }

    #[test]
    fn chunk_digest_binds_all_fields() {
        let a = make_chunk(1, 10, b"x");
        let b = make_chunk(2, 10, b"x");
        let c = make_chunk(1, 11, b"x");
        let d = make_chunk(1, 10, b"y");
        assert_ne!(a.chunk_digest(), b.chunk_digest());
        assert_ne!(a.chunk_digest(), c.chunk_digest());
        assert_ne!(a.chunk_digest(), d.chunk_digest());
    }

    #[test]
    fn merkle_root_verification_all_zero_path_is_consistent() {
        let proof = CascadeProof {
            leaf_index: [0u8; LEAF_INDEX_BYTES],
            l7_output: [0u8; CASCADE_OUTPUT_BYTES],
            merkle_path: Box::new([[0u8; 32]; SPARSE_MERKLE_DEPTH]),
        };
        // Compute what root the all-zero proof produces, then verify it matches.
        let leaf_hash: [u8; 32] = {
            let mut h = Sha3_256::new();
            h.update(MERKLE_LEAF_DOMAIN);
            h.update([0u8; LEAF_INDEX_BYTES]);
            h.update([0u8; CASCADE_OUTPUT_BYTES]);
            h.finalize().into()
        };
        let mut current = leaf_hash;
        for sibling in proof.merkle_path.iter() {
            let mut h = Sha3_256::new();
            h.update(MERKLE_INTERNAL_DOMAIN);
            h.update(current); // leaf_index bit 0 → always left child
            h.update(sibling);
            current = h.finalize().into();
        }
        assert!(proof.verify_merkle_root(&current));
    }

    #[test]
    fn merkle_root_verification_rejects_wrong_root() {
        let proof = CascadeProof {
            leaf_index: [0u8; LEAF_INDEX_BYTES],
            l7_output: [0u8; CASCADE_OUTPUT_BYTES],
            merkle_path: Box::new([[0u8; 32]; SPARSE_MERKLE_DEPTH]),
        };
        assert!(!proof.verify_merkle_root(&[0xFFu8; 32]));
    }

    #[test]
    fn all_channels_have_nonzero_max_chunk_bytes() {
        let channels = [
            CloneChannel::QrCode,
            CloneChannel::Nfc,
            CloneChannel::Ble,
            CloneChannel::WifiDirect,
            CloneChannel::LoRa,
            CloneChannel::Ultrasonic,
        ];
        for ch in &channels {
            assert!(ch.max_chunk_bytes() > 0, "{ch:?} has zero chunk size");
        }
    }

    #[test]
    fn wifi_direct_has_largest_capacity() {
        assert!(CloneChannel::WifiDirect.max_chunk_bytes() > CloneChannel::QrCode.max_chunk_bytes());
        assert!(CloneChannel::WifiDirect.max_chunk_bytes() > CloneChannel::LoRa.max_chunk_bytes());
    }

    #[test]
    fn clone_hop_validates_bounds() {
        assert!(CloneHop::new(CloneChannel::Ble, 0, 0).is_ok());
        assert!(CloneHop::new(CloneChannel::Ble, MAX_OFFLINE_HOPS - 1, MAX_OFFLINE_EPOCHS - 1).is_ok());
        assert_eq!(
            CloneHop::new(CloneChannel::Ble, MAX_OFFLINE_HOPS, 0).unwrap_err(),
            CloneHopError::HopIndexExceedsMax
        );
        assert_eq!(
            CloneHop::new(CloneChannel::Ble, 0, MAX_OFFLINE_EPOCHS).unwrap_err(),
            CloneHopError::EpochOffsetExceedsMax
        );
    }

    #[test]
    fn terminal_hop_detection() {
        let hop = CloneHop::new(CloneChannel::LoRa, MAX_OFFLINE_HOPS - 1, 0).unwrap();
        assert!(hop.is_terminal());
        let non_terminal = CloneHop::new(CloneChannel::LoRa, 0, 0).unwrap();
        assert!(!non_terminal.is_terminal());
    }

    #[test]
    fn only_ble_supports_dual_role() {
        assert!(CloneChannel::Ble.supports_dual_role());
        assert!(!CloneChannel::QrCode.supports_dual_role());
        assert!(!CloneChannel::WifiDirect.supports_dual_role());
        assert!(!CloneChannel::LoRa.supports_dual_role());
    }
}
