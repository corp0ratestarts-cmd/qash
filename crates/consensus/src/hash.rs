//! Domain-separated hashing (consensus-critical).
//!
//! H_domain(tag, input) = SHA3-256( tag_u32_le || input )

use sha3::{Digest, Sha3_256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DomainTag {
    StateRoot      = 0x0000_0001,
    EntropyAdvance = 0x0000_0002,
    ValidatorId    = 0x0000_0003,
    LeafHash       = 0x0000_0004,
    InternalHash   = 0x0000_0005,
    TxId           = 0x0000_0010,
}

pub fn h_domain(tag: DomainTag, input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update((tag as u32).to_le_bytes());
    hasher.update(input);
    let out = hasher.finalize();
    let mut res = [0u8; 32];
    res.copy_from_slice(&out);
    res
}

/// Untagged SHA3-256 (only where the spec explicitly requires it).
pub fn sha3_256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(input);
    let out = hasher.finalize();
    let mut res = [0u8; 32];
    res.copy_from_slice(&out);
    res
}
