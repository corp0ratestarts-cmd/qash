#![cfg(kani)]

use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::transaction::{tx1_project_divergence, TxError};

#[kani::proof]
fn tx1_project_divergence_never_increases() {
    let current: u32 = kani::any();
    let delta: u32 = kani::any();
    kani::assume(current <= 1_000_000);
    kani::assume(delta <= current);

    let next = tx1_project_divergence(FixedPoint::from_raw(i128::from(current)), delta).unwrap();

    assert!(next.raw() <= i128::from(current));
    assert_eq!(next.raw(), i128::from(current - delta));
}

#[kani::proof]
fn tx1_project_divergence_rejects_excess_delta() {
    let current: u32 = kani::any();
    let delta: u32 = kani::any();
    kani::assume(current <= 1_000_000);
    kani::assume(delta > current);

    assert_eq!(
        tx1_project_divergence(FixedPoint::from_raw(i128::from(current)), delta),
        Err(TxError::DeltaExceedsDivergence)
    );
}
