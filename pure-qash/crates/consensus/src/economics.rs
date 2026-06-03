//! Domain A deterministic economics module — Constitutional Scarcity Axiom.
//!
//! All values are integers. No floating-point. No wall clock. No oracle. No governance.
//! All arithmetic is checked; overflow triggers absorbing halt.
//!
//! See `docs/spec/08_tokenomics.md` for the formal spec.

use crate::transition::{EpochState, HaltReason};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Non-negative supply amount in atomic units.
///
/// Using u128 (not i128) because amounts cannot be negative.
/// Any conversion to i128 for FixedPoint operations must use checked casts.
pub type Amount = u128;

/// Economics state included in canonical EpochState encoding.
/// Conservation invariant: total_supply = issued_total - burned_fees_total - burned_slashes_total
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EconomicsState {
    pub total_supply:         Amount,
    pub issued_total:         Amount,
    pub burned_fees_total:    Amount,
    pub burned_slashes_total: Amount,
}

impl EconomicsState {
    pub const fn zero() -> Self {
        Self {
            total_supply:         0,
            issued_total:         0,
            burned_fees_total:    0,
            burned_slashes_total: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Genesis constants (from GENESIS_CONSTANTS.toml [economics] and [fee])
// ---------------------------------------------------------------------------

const INITIAL_REWARD:    Amount = 1_000_000_000;
const DECAY_INTERVAL:    u64    = 10_512_000;
const TAIL_REWARD:       Amount = 10_000;

const FEE_BASE_PTX0:     Amount = 1_000;
const FEE_PER_BYTE:      Amount = 10;

// ---------------------------------------------------------------------------
// Issuance
// ---------------------------------------------------------------------------

/// Compute the per-epoch validator reward pool.
/// Deterministic, monotone-decreasing, bounded below by TAIL_REWARD.
/// No floats. No wall clock. No oracle.
pub fn epoch_reward(epoch: u64) -> Result<Amount, HaltReason> {
    let shift = (epoch / DECAY_INTERVAL) as u32;
    let decayed = if shift >= 128 {
        0u128
    } else {
        INITIAL_REWARD >> shift
    };
    Ok(decayed.max(TAIL_REWARD))
}

// ---------------------------------------------------------------------------
// Fee computation
// ---------------------------------------------------------------------------

/// PTX-0 transaction type identifier in the Pure QASH registry.
pub const PTX0_TYPE: u16 = 0x0000;

/// Compute the required fee for a transaction. Fee is a pure resource cost; no bidding.
pub fn required_fee(tx_type: u16, payload_len: u32) -> Result<Amount, HaltReason> {
    let base = match tx_type {
        PTX0_TYPE => FEE_BASE_PTX0,
        _ => return Err(HaltReason::DecodeInvalid),
    };
    let payload_cost = (payload_len as Amount)
        .checked_mul(FEE_PER_BYTE)
        .ok_or(HaltReason::ArithOverflow)?;
    base.checked_add(payload_cost).ok_or(HaltReason::ArithOverflow)
}

/// Validate that the attached fee exactly equals the required fee.
/// Overpayment and underpayment both reject — no bidding, no change.
pub fn validate_exact_fee(attached: Amount, required: Amount) -> Result<(), HaltReason> {
    if attached == required {
        Ok(())
    } else {
        Err(HaltReason::DecodeInvalid)
    }
}

// ---------------------------------------------------------------------------
// State mutation (all checked arithmetic; overflow → absorbing halt)
// ---------------------------------------------------------------------------

/// Burn a fee amount: subtract from total_supply, add to burned_fees_total.
pub fn apply_fee_burn(
    economics: &mut EconomicsState,
    amount: Amount,
) -> Result<(), HaltReason> {
    economics.total_supply = economics
        .total_supply
        .checked_sub(amount)
        .ok_or(HaltReason::ArithOverflow)?;
    economics.burned_fees_total = economics
        .burned_fees_total
        .checked_add(amount)
        .ok_or(HaltReason::ArithOverflow)?;
    Ok(())
}

/// Burn a slash amount: subtract from total_supply, add to burned_slashes_total.
pub fn apply_slash_burn(
    economics: &mut EconomicsState,
    amount: Amount,
) -> Result<(), HaltReason> {
    economics.total_supply = economics
        .total_supply
        .checked_sub(amount)
        .ok_or(HaltReason::ArithOverflow)?;
    economics.burned_slashes_total = economics
        .burned_slashes_total
        .checked_add(amount)
        .ok_or(HaltReason::ArithOverflow)?;
    Ok(())
}

/// Issue the epoch reward: add to total_supply and issued_total.
/// Validators receive the reward pool; no fee revenue for validators.
pub fn apply_epoch_reward(
    economics: &mut EconomicsState,
    epoch: u64,
) -> Result<Amount, HaltReason> {
    let reward = epoch_reward(epoch)?;
    economics.total_supply = economics
        .total_supply
        .checked_add(reward)
        .ok_or(HaltReason::ArithOverflow)?;
    economics.issued_total = economics
        .issued_total
        .checked_add(reward)
        .ok_or(HaltReason::ArithOverflow)?;
    Ok(reward)
}

/// Verify the conservation invariant: total_supply = issued_total - burned_fees - burned_slashes.
pub fn verify_conservation(e: &EconomicsState) -> Result<(), HaltReason> {
    let burned = e
        .burned_fees_total
        .checked_add(e.burned_slashes_total)
        .ok_or(HaltReason::ArithOverflow)?;
    let expected = e
        .issued_total
        .checked_sub(burned)
        .ok_or(HaltReason::ArithOverflow)?;
    if e.total_supply == expected {
        Ok(())
    } else {
        Err(HaltReason::ArithOverflow)
    }
}

// ---------------------------------------------------------------------------
// EpochState integration (pure-qash adds economics field)
// ---------------------------------------------------------------------------

/// Apply epoch economics to the full epoch state:
/// 1. Issue reward (increases supply)
/// 2. Burn accumulated fees for this epoch
/// 3. Burn accumulated slashes for this epoch
/// 4. Verify conservation invariant
pub fn apply_epoch_economics(
    state: &mut EpochState,
    fee_total: Amount,
    slash_total: Amount,
) -> Result<Amount, HaltReason> {
    let reward = apply_epoch_reward(&mut state.economics, state.epoch)?;
    apply_fee_burn(&mut state.economics, fee_total)?;
    apply_slash_burn(&mut state.economics, slash_total)?;
    verify_conservation(&state.economics)?;
    Ok(reward)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_reward_initial() {
        assert_eq!(epoch_reward(0).unwrap(), INITIAL_REWARD);
    }

    #[test]
    fn epoch_reward_tail_floor() {
        // After many decay intervals, reward must not fall below TAIL_REWARD
        assert_eq!(epoch_reward(u64::MAX).unwrap(), TAIL_REWARD);
        assert_eq!(epoch_reward(DECAY_INTERVAL * 200).unwrap(), TAIL_REWARD);
    }

    #[test]
    fn epoch_reward_monotone_non_increasing() {
        let r0 = epoch_reward(0).unwrap();
        let r1 = epoch_reward(DECAY_INTERVAL).unwrap();
        let r2 = epoch_reward(DECAY_INTERVAL * 2).unwrap();
        assert!(r0 >= r1);
        assert!(r1 >= r2);
        assert!(r2 >= TAIL_REWARD);
    }

    #[test]
    fn required_fee_ptx0_base() {
        let fee = required_fee(PTX0_TYPE, 0).unwrap();
        assert_eq!(fee, FEE_BASE_PTX0);
    }

    #[test]
    fn required_fee_ptx0_with_payload() {
        let fee = required_fee(PTX0_TYPE, 10).unwrap();
        assert_eq!(fee, FEE_BASE_PTX0 + 10 * FEE_PER_BYTE);
    }

    #[test]
    fn required_fee_unknown_type() {
        assert_eq!(required_fee(0xFFFF, 0), Err(HaltReason::DecodeInvalid));
    }

    #[test]
    fn validate_exact_fee_exact() {
        assert!(validate_exact_fee(1_000, 1_000).is_ok());
    }

    #[test]
    fn validate_exact_fee_overpayment_rejects() {
        assert_eq!(validate_exact_fee(1_001, 1_000), Err(HaltReason::DecodeInvalid));
    }

    #[test]
    fn validate_exact_fee_underpayment_rejects() {
        assert_eq!(validate_exact_fee(999, 1_000), Err(HaltReason::DecodeInvalid));
    }

    #[test]
    fn fee_burn_reduces_supply() {
        let mut e = EconomicsState { total_supply: 1_000, issued_total: 1_000,
            burned_fees_total: 0, burned_slashes_total: 0 };
        apply_fee_burn(&mut e, 100).unwrap();
        assert_eq!(e.total_supply, 900);
        assert_eq!(e.burned_fees_total, 100);
    }

    #[test]
    fn slash_burn_reduces_supply() {
        let mut e = EconomicsState { total_supply: 1_000, issued_total: 1_000,
            burned_fees_total: 0, burned_slashes_total: 0 };
        apply_slash_burn(&mut e, 50).unwrap();
        assert_eq!(e.total_supply, 950);
        assert_eq!(e.burned_slashes_total, 50);
    }

    #[test]
    fn conservation_invariant_holds() {
        let e = EconomicsState {
            total_supply: 900,
            issued_total: 1_000,
            burned_fees_total: 60,
            burned_slashes_total: 40,
        };
        assert!(verify_conservation(&e).is_ok());
    }

    #[test]
    fn conservation_invariant_violation_detected() {
        let e = EconomicsState {
            total_supply: 901, // wrong by 1
            issued_total: 1_000,
            burned_fees_total: 60,
            burned_slashes_total: 40,
        };
        assert_eq!(verify_conservation(&e), Err(HaltReason::ArithOverflow));
    }

    #[test]
    fn no_validator_fee_revenue() {
        // Reward comes only from apply_epoch_reward, not from fee burns.
        // Fee burns reduce supply; they do not flow to any validator.
        let mut e = EconomicsState::zero();
        let reward = apply_epoch_reward(&mut e, 0).unwrap();
        assert_eq!(reward, INITIAL_REWARD);
        assert_eq!(e.total_supply, INITIAL_REWARD);
        // Now burn a fee — supply drops, no validator receives it
        apply_fee_burn(&mut e, 500).unwrap();
        assert_eq!(e.total_supply, INITIAL_REWARD - 500);
        assert_eq!(e.burned_fees_total, 500);
    }
}
