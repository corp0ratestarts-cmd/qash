//! Consensus parameter fingerprinting (used by tests/main only).
//! Nothing in the consensus path should depend on this module.

// Compile-time guards: every numeric genesis constant in Rust must match
// GENESIS_CONSTANTS.toml exactly. Drift → build break.

const _GENESIS_WEIGHT_D:   i128 = 350_000;
const _GENESIS_WEIGHT_C:   i128 = 300_000;
const _GENESIS_WEIGHT_S:   i128 = 200_000;
const _GENESIS_WEIGHT_CH:  i128 = 150_000;
const _GENESIS_EPSILON:    i128 = 20_000;
const _GENESIS_PHI_MAX_SAFE: i128 = 944_473_296_573_929_042_432;

const _: () = {
    if crate::lyapunov::WEIGHT_D.raw()  != _GENESIS_WEIGHT_D  { panic!("WEIGHT_D mismatch vs GENESIS_CONSTANTS.toml"); }
    if crate::lyapunov::WEIGHT_C.raw()  != _GENESIS_WEIGHT_C  { panic!("WEIGHT_C mismatch vs GENESIS_CONSTANTS.toml"); }
    if crate::lyapunov::WEIGHT_S.raw()  != _GENESIS_WEIGHT_S  { panic!("WEIGHT_S mismatch vs GENESIS_CONSTANTS.toml"); }
    if crate::lyapunov::WEIGHT_CH.raw() != _GENESIS_WEIGHT_CH { panic!("WEIGHT_CH mismatch vs GENESIS_CONSTANTS.toml"); }
    if crate::lyapunov::EPSILON.raw()   != _GENESIS_EPSILON   { panic!("EPSILON mismatch vs GENESIS_CONSTANTS.toml"); }
    if crate::lyapunov::PHI_MAX_SAFE    != _GENESIS_PHI_MAX_SAFE { panic!("PHI_MAX_SAFE in lyapunov.rs does not match phi_max_safe pinned in GENESIS_CONSTANTS.toml"); }
    // TH-GC ordering invariant: honest budget must be strictly below halt threshold.
    if crate::lyapunov::EPSILON_HONEST.raw() >= crate::lyapunov::EPSILON.raw() {
        panic!("EPSILON_HONEST must be strictly less than EPSILON (ε_halt) for TH-GC to hold");
    }
    // Weight sum invariant: all active + pending weights ≤ 1_000_000 (= SCALE).
    // WEIGHT_SH and WEIGHT_BH are currently 0 (v1.1.1 rollout not yet active).
    // When v1.1.1 activates, D/C/S/CH will be reduced to maintain sum = 1_000_000.
    if _GENESIS_WEIGHT_D + _GENESIS_WEIGHT_C + _GENESIS_WEIGHT_S + _GENESIS_WEIGHT_CH
        + crate::lyapunov::WEIGHT_SH.raw() + crate::lyapunov::WEIGHT_BH.raw() > 1_000_000
    {
        panic!("Lyapunov weights (including v1.1.1 zero-weights) exceed SCALE — invariant violated");
    }
};

use crate::{
    encoding, fixed_point,
    hash::{h_domain, DomainTag, HashPrimitive, CONSENSUS_HASH_PRIMITIVE_COUNT},
    lyapunov, transition,
};

const HASH_ALGORITHM_ID: u32 = 0x0000_0002; // multi-primitive consensus root v1

pub fn consensus_params_hash() -> [u8; 32] {
    // Layout (all LE):
    //   encoding_version: u32       =   4
    //   hash_algorithm_id: u32      =   4
    //   weight_d: i128              =  16
    //   weight_c: i128              =  16
    //   weight_s: i128              =  16
    //   epsilon: i128               =  16
    //   phi_max_safe: i128          =  16
    //   scale: i128                 =  16
    //   window_size: u32            =   4
    //   max_validators: u32         =   4
    //   encoding_header_size: u32   =   4
    //   fixed_point_byte_width: u32 =   4
    //   domain tags ×6: u32 each    =  24
    //   active hash primitive count  =   4
    //   active hash primitive ids ×2 =   8
    //                         TOTAL = 140
    const BUF_SIZE: usize = 140;
    let mut buf = [0u8; BUF_SIZE];
    let mut o: usize = 0;

    buf[o..o + 4].copy_from_slice(&encoding::ENCODING_VERSION.to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&HASH_ALGORITHM_ID.to_le_bytes());
    o += 4;

    buf[o..o + 16].copy_from_slice(&lyapunov::WEIGHT_D.raw().to_le_bytes());
    o += 16;
    buf[o..o + 16].copy_from_slice(&lyapunov::WEIGHT_C.raw().to_le_bytes());
    o += 16;
    buf[o..o + 16].copy_from_slice(&lyapunov::WEIGHT_S.raw().to_le_bytes());
    o += 16;

    buf[o..o + 16].copy_from_slice(&lyapunov::EPSILON.raw().to_le_bytes());
    o += 16;
    buf[o..o + 16].copy_from_slice(&fixed_point::SCALE.to_le_bytes());
    o += 16;

    buf[o..o + 4].copy_from_slice(&(lyapunov::WINDOW_SIZE as u32).to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&(transition::MAX_VALIDATORS as u32).to_le_bytes());
    o += 4;

    buf[o..o + 4].copy_from_slice(&encoding::STATE_HEADER_SIZE.to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&fixed_point::FIXED_POINT_WIRE_BYTES.to_le_bytes());
    o += 4;

    buf[o..o + 4].copy_from_slice(&(DomainTag::StateRoot as u32).to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&(DomainTag::EntropyAdvance as u32).to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&(DomainTag::ValidatorId as u32).to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&(DomainTag::LeafHash as u32).to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&(DomainTag::InternalHash as u32).to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&(DomainTag::ConsensusRoot as u32).to_le_bytes());
    o += 4;

    buf[o..o + 4].copy_from_slice(&(CONSENSUS_HASH_PRIMITIVE_COUNT as u32).to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&(HashPrimitive::Sha3_256 as u32).to_le_bytes());
    o += 4;
    buf[o..o + 4].copy_from_slice(&(HashPrimitive::Sm3 as u32).to_le_bytes());
    o += 4;

    debug_assert_eq!(o, BUF_SIZE);

    h_domain(DomainTag::InternalHash, &buf)
}
