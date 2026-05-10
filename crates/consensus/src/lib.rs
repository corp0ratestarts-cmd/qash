#![no_std]
#![forbid(unsafe_code)]

use blake3::Hasher;

/// Deterministic, no-alloc consensus hash function used by the core.
pub fn consensus_hash(input: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(input);
    let out = hasher.finalize();
    let bytes = out.as_bytes();
    let mut res = [0u8; 32];
    res.copy_from_slice(&bytes[..32]);
    res
}
