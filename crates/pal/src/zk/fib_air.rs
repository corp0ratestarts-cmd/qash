/// Fibonacci AIR — test fixture for the QASH Plonky3 production backend.
///
/// This is NOT part of the QASH protocol. It is used exclusively in unit tests
/// to demonstrate that `Plonky3ProductionBackend` can generate and verify real
/// FRI-STARK proofs with the QASH FRI configuration.
///
/// The actual QASH shard transition AIR will replace this once the circuit is
/// developed. The fixture AIR provides a proof/verify roundtrip test today.
///
/// # Circuit description
///
/// Two columns (left, right). Public values: `[a, b, x]`.
///   - Row 0: left = a, right = b  (initial conditions)
///   - Row i → Row i+1: right' = left, right' = left + right  (Fibonacci step)
///   - Last row: right = x  (terminal condition: the n-th Fibonacci number)
use core::borrow::Borrow;

use p3_air::{Air, AirBuilder, BaseAir, WindowAccess};
use p3_field::PrimeCharacteristicRing;
use p3_matrix::dense::RowMajorMatrix;

use super::profile::QashVal;

const NUM_FIBONACCI_COLS: usize = 2;

// ── Row layout ────────────────────────────────────────────────────────────────

#[repr(C)]
pub struct FibRow<F> {
    pub left: F,
    pub right: F,
}

impl<F> Borrow<FibRow<F>> for [F] {
    fn borrow(&self) -> &FibRow<F> {
        debug_assert_eq!(self.len(), NUM_FIBONACCI_COLS);
        // SAFETY: FibRow<F> is #[repr(C)] with the same field type F; the slice
        // is exactly NUM_FIBONACCI_COLS elements so alignment and size match.
        let (prefix, shorts, suffix) = unsafe { self.align_to::<FibRow<F>>() };
        debug_assert!(prefix.is_empty());
        debug_assert!(suffix.is_empty());
        debug_assert_eq!(shorts.len(), 1);
        &shorts[0]
    }
}

// ── AIR definition ─────────────────────────────────────────────────────────────

/// Fibonacci AIR: two columns, three public values [a, b, x].
pub struct FibonacciAir;

impl<F> BaseAir<F> for FibonacciAir {
    fn width(&self) -> usize {
        NUM_FIBONACCI_COLS
    }

    fn num_public_values(&self) -> usize {
        3
    }
}

impl<AB: AirBuilder> Air<AB> for FibonacciAir {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let pv = builder.public_values();

        let a = pv[0].clone();
        let b = pv[1].clone();
        let x = pv[2].clone();

        let local: &FibRow<AB::Var> = main.current_slice().borrow();
        let next: &FibRow<AB::Var> = main.next_slice().borrow();

        // Row 0 constraints.
        builder.when_first_row().assert_eq(local.left.clone(), a);
        builder.when_first_row().assert_eq(local.right.clone(), b);

        // Transition: a' = b, b' = a + b.
        builder
            .when_transition()
            .assert_eq(local.right.clone(), next.left.clone());
        builder
            .when_transition()
            .assert_eq(local.left.clone() + local.right.clone(), next.right.clone());

        // Terminal: last row right = x.
        builder.when_last_row().assert_eq(local.right.clone(), x);
    }
}

// ── Trace generator ───────────────────────────────────────────────────────────

/// Generate a Fibonacci trace with `n` rows (must be power of two).
///
/// Returns a `RowMajorMatrix<QashVal>` suitable for proving with `p3_uni_stark::prove`.
pub fn generate_fib_trace(a: u64, b: u64, n: usize) -> RowMajorMatrix<QashVal> {
    assert!(n.is_power_of_two(), "trace height must be a power of two");
    let mut values = QashVal::zero_vec(n * NUM_FIBONACCI_COLS);
    // SAFETY: FibRow<QashVal> is #[repr(C)] with field QashVal; the allocation
    // is n*NUM_FIBONACCI_COLS elements, matching n FibRow entries exactly.
    let (prefix, rows, suffix) = unsafe { values.align_to_mut::<FibRow<QashVal>>() };
    assert!(prefix.is_empty());
    assert!(suffix.is_empty());

    rows[0] = FibRow {
        left: QashVal::from_u64(a),
        right: QashVal::from_u64(b),
    };
    for i in 1..n {
        rows[i].left = rows[i - 1].right;
        rows[i].right = rows[i - 1].left + rows[i - 1].right;
    }
    RowMajorMatrix::new(values, NUM_FIBONACCI_COLS)
}

/// Compute the n-th Fibonacci number starting from (a, b).
pub fn fib_nth(a: u64, b: u64, n: usize) -> u64 {
    if n == 0 {
        return a;
    }
    let (mut lo, mut hi) = (a, b);
    for _ in 1..n {
        let next = lo + hi;
        lo = hi;
        hi = next;
    }
    hi
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fib_trace_rows_satisfy_recurrence() {
        let n = 8;
        let trace = generate_fib_trace(0, 1, n);
        let rows = trace.values;
        // row 0: [0, 1]
        assert_eq!(rows[0], QashVal::from_u64(0));
        assert_eq!(rows[1], QashVal::from_u64(1));
        // row 1: [1, 1]
        assert_eq!(rows[2], QashVal::from_u64(1));
        assert_eq!(rows[3], QashVal::from_u64(1));
        // row 7: [13, 21]
        assert_eq!(rows[14], QashVal::from_u64(13));
        assert_eq!(rows[15], QashVal::from_u64(21));
    }

    #[test]
    fn fib_nth_standard_values() {
        assert_eq!(fib_nth(0, 1, 8), 21);
        assert_eq!(fib_nth(0, 1, 1), 1);
        assert_eq!(fib_nth(0, 1, 0), 0);
    }
}
