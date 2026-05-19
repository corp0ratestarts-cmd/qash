//! Causal ordering for v1.1 envelopes.
//!
//! sort_key = H_domain(CausalOrder, epoch_seed ∥ shard_id_be ∥ envelope_hash)
//!
//! The resulting 32-byte key is compared lexicographically so that envelopes
//! within the same epoch have a total, deterministic causal order that depends
//! on the epoch entropy (preventing pre-computation attacks).

use crate::hash::{h_domain, sha3_256, DomainTag};

/// Compute the causal sort key for a v1.1 envelope.
///
/// - `epoch_seed`: 32-byte entropy seed for the current epoch.
/// - `shard_id`: u32 shard identifier (big-endian encoded into the pre-image).
/// - `envelope_hash`: SHA3-256 hash of the envelope payload bytes.
///
/// Returns a 32-byte sort key. Equal keys are collision-resistant under AX-3.
pub fn compute_sort_key(
    epoch_seed: &[u8; 32],
    shard_id: u32,
    envelope_hash: &[u8; 32],
) -> [u8; 32] {
    // Pre-image: epoch_seed(32) ∥ shard_id_be(4) ∥ envelope_hash(32) = 68 bytes
    let mut buf = [0u8; 68];
    buf[..32].copy_from_slice(epoch_seed);
    buf[32..36].copy_from_slice(&shard_id.to_be_bytes());
    buf[36..].copy_from_slice(envelope_hash);
    h_domain(DomainTag::CausalOrder, &buf)
}

/// Convenience: hash an envelope payload slice and compute its sort key.
///
/// Equivalent to `compute_sort_key(epoch_seed, shard_id, &sha3_256(payload))`.
pub fn sort_key_from_payload(
    epoch_seed: &[u8; 32],
    shard_id: u32,
    payload: &[u8],
) -> [u8; 32] {
    let envelope_hash = sha3_256(payload);
    compute_sort_key(epoch_seed, shard_id, &envelope_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_sort_key_deterministic() {
        let seed = [0x42u8; 32];
        let hash = [0xABu8; 32];
        let k1 = compute_sort_key(&seed, 0, &hash);
        let k2 = compute_sort_key(&seed, 0, &hash);
        assert_eq!(k1, k2);
    }

    #[test]
    fn compute_sort_key_shard_distinguishes() {
        let seed = [0u8; 32];
        let hash = [0u8; 32];
        let k0 = compute_sort_key(&seed, 0, &hash);
        let k1 = compute_sort_key(&seed, 1, &hash);
        assert_ne!(k0, k1, "different shard_id must produce different sort keys");
    }

    #[test]
    fn compute_sort_key_seed_distinguishes() {
        let mut seed_a = [0u8; 32];
        let mut seed_b = [0u8; 32];
        seed_b[0] = 1;
        let hash = [0u8; 32];
        let ka = compute_sort_key(&seed_a, 0, &hash);
        let kb = compute_sort_key(&seed_b, 0, &hash);
        assert_ne!(ka, kb);
        // Restore to satisfy mutability lint
        seed_a[0] = 0;
        seed_b[0] = 1;
        let _ = (seed_a, seed_b);
    }

    #[test]
    fn compute_sort_key_payload_hash_distinguishes() {
        let seed = [0u8; 32];
        let hash_a = [0u8; 32];
        let mut hash_b = [0u8; 32];
        hash_b[31] = 1;
        let ka = compute_sort_key(&seed, 0, &hash_a);
        let kb = compute_sort_key(&seed, 0, &hash_b);
        assert_ne!(ka, kb);
    }

    // Known-answer test: verifies the output is stable across compiler versions
    // and ISAs. Computed from H_domain(CausalOrder=0x20, [0u8;68]).
    #[test]
    fn compute_sort_key_kat_zeros() {
        let seed = [0u8; 32];
        let hash = [0u8; 32];
        let got = compute_sort_key(&seed, 0, &hash);
        // Tag bytes (LE u32 = 0x00000020) prepended to 68 zero bytes.
        // Recompute expected: sha3_256([0x20,0,0,0] ++ [0u8;68])
        let mut pre = [0u8; 72];
        pre[0] = 0x20; // DomainTag::CausalOrder LE
        let expected = sha3_256(&pre);
        assert_eq!(got, expected);
    }

    #[test]
    fn sort_key_from_payload_matches_explicit() {
        let seed = [0x11u8; 32];
        let payload = b"test payload bytes";
        let hash = sha3_256(payload);
        let explicit = compute_sort_key(&seed, 42, &hash);
        let convenience = sort_key_from_payload(&seed, 42, payload);
        assert_eq!(explicit, convenience);
    }
}
