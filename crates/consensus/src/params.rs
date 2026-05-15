//! Consensus parameter fingerprinting (used by tests/main only).
//! Nothing in the consensus path should depend on this module.

use crate::{
    encoding,
    fixed_point,
    hash::{h_domain, DomainTag},
    lyapunov,
    transition,
};

const HASH_ALGORITHM_ID: u32 = 0x0000_0001; // SHA3-256

pub fn consensus_params_hash() -> [u8; 32] {
    // Layout (all LE):
    //   encoding_version: u32       =   4
    //   hash_algorithm_id: u32      =   4
    //   weight_d: i128              =  16
    //   weight_c: i128              =  16
    //   weight_s: i128              =  16
    //   epsilon: i128               =  16
    //   scale: i128                 =  16
    //   window_size: u32            =   4
    //   max_validators: u32         =   4
    //   encoding_header_size: u32   =   4
    //   fixed_point_byte_width: u32 =   4
    //   domain tags ×5: u32 each    =  20
    //                         TOTAL = 124
    const BUF_SIZE: usize = 124;
    let mut buf = [0u8; BUF_SIZE];
    let mut o: usize = 0;

    buf[o..o + 4].copy_from_slice(&encoding::ENCODING_VERSION.to_le_bytes()); o += 4;
    buf[o..o + 4].copy_from_slice(&HASH_ALGORITHM_ID.to_le_bytes()); o += 4;

    buf[o..o + 16].copy_from_slice(&lyapunov::WEIGHT_D.raw().to_le_bytes()); o += 16;
    buf[o..o + 16].copy_from_slice(&lyapunov::WEIGHT_C.raw().to_le_bytes()); o += 16;
    buf[o..o + 16].copy_from_slice(&lyapunov::WEIGHT_S.raw().to_le_bytes()); o += 16;

    buf[o..o + 16].copy_from_slice(&lyapunov::EPSILON.raw().to_le_bytes()); o += 16;
    buf[o..o + 16].copy_from_slice(&fixed_point::SCALE.to_le_bytes()); o += 16;

    buf[o..o + 4].copy_from_slice(&(lyapunov::WINDOW_SIZE as u32).to_le_bytes()); o += 4;
    buf[o..o + 4].copy_from_slice(&(transition::MAX_VALIDATORS as u32).to_le_bytes()); o += 4;

    buf[o..o + 4].copy_from_slice(&encoding::STATE_HEADER_SIZE.to_le_bytes()); o += 4;
    buf[o..o + 4].copy_from_slice(&fixed_point::FIXED_POINT_WIRE_BYTES.to_le_bytes()); o += 4;

    buf[o..o + 4].copy_from_slice(&(DomainTag::StateRoot as u32).to_le_bytes()); o += 4;
    buf[o..o + 4].copy_from_slice(&(DomainTag::EntropyAdvance as u32).to_le_bytes()); o += 4;
    buf[o..o + 4].copy_from_slice(&(DomainTag::ValidatorId as u32).to_le_bytes()); o += 4;
    buf[o..o + 4].copy_from_slice(&(DomainTag::LeafHash as u32).to_le_bytes()); o += 4;
    buf[o..o + 4].copy_from_slice(&(DomainTag::InternalHash as u32).to_le_bytes()); o += 4;

    debug_assert_eq!(o, BUF_SIZE);

    h_domain(DomainTag::InternalHash, &buf)
}
