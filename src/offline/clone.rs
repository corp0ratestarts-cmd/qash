// Clone Protocol — CascadeBoundCloneChunk
//
// Spec: docs/spec/05_clone_protocol.md (normative)
//       docs/spec/07_hash_cascade.md "Cascade Proof Format" (proof structure)
//       GENESIS_CONSTANTS.toml [clone_protocol] (chunk_verification_mode = "cascade_bound")
//
// Domain B — offline / clone operations.  The cascade proof fields are
// Domain A values (deterministic) but the transport/serialisation layer
// is Domain B.  No Domain B nondeterminism may influence proof verification.

// ---------------------------------------------------------------------------
// Cascade proof format — spec 07_hash_cascade.md §3 "Cascade Proof Format"
// ---------------------------------------------------------------------------

/// Sparse-Merkle inclusion proof for one H_cascade output.
///
/// Sizes are fixed at genesis:
///   leaf_index_bytes   = 48  (GENESIS_CONSTANTS.toml [obfuscation])
///   cascade output     = 64  (H_cascade → [u8; 64])
///   sparse_merkle_depth = 384 (GENESIS_CONSTANTS.toml [obfuscation])
///
/// Proof verification (spec §3):
///   1. Recompute Merkle root from leaf_index, l7_output, and merkle_path
///      using H_domain(LEAF_HASH, …) / H_domain(INTERNAL_HASH, …).
///   2. Compare against the epoch's cascade_root from the epoch state.
#[derive(Clone, Debug)]
pub struct CascadeProof {
    /// Epoch-relative leaf index (first 48 bytes of H_cascade_keyed(seed_t, …)).
    pub leaf_index: [u8; 48],
    /// H_cascade output committed into the sparse Merkle tree.
    pub l7_output: [u8; 64],
    /// Sibling hashes from leaf to root (depth = 384 nodes).
    /// Boxed to avoid 12 KiB on the stack.
    pub merkle_path: Box<[[u8; 32]; 384]>,
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

    /// Verify that proof.l7_output matches H_cascade_keyed(seed_t, cascade_input).
    ///
    /// `seed_t` is the entropy seed for `self.epoch` (01_consensus.md §1).
    /// Caller is responsible for supplying the correct seed.
    pub fn verify_l7(&self, seed_t: &[u8; 32]) -> bool {
        let input = self.cascade_input();
        let expected = crate::crypto::cascade::h_cascade_keyed(seed_t, &input);
        self.proof.l7_output == expected
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
}

/// Hop descriptor for offline multi-hop routing.
/// max_offline_hops = 7, max_offline_epochs = 12 (GENESIS_CONSTANTS.toml).
#[derive(Clone, Debug)]
pub struct CloneHop {
    pub channel: CloneChannel,
    pub hop_index: u8,    // 0-based, must be < 7
    pub epoch_offset: u8, // epochs since chunk was produced, must be < 12
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
                leaf_index: [0u8; 48],
                l7_output: [0u8; 64],
                merkle_path: Box::new([[0u8; 32]; 384]),
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
}
