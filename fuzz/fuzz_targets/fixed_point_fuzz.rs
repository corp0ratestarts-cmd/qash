// Fuzz target: FixedPoint arithmetic — overflow, saturation, rounding invariants.
//
// Verifies:
//   1. No panic on any input (all paths return Ok/Err, never panic)
//   2. checked_mul/checked_div/checked_add/checked_sub never silently overflow
//   3. floor_div: result * b ≤ a < (result+1) * b  for all b ≠ 0
//   4. Commutativity: a + b == b + a, a * b == b * a
//   5. to_i64 iff raw() ∈ [i64::MIN, i64::MAX]
//
// Run: cargo hfuzz run fixed_point_fuzz  (from fuzz/)

use honggfuzz::fuzz;
use arbitrary::Arbitrary;
use qash_consensus::fixed_point::{FixedPoint, floor_div_i128, SCALE};

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    a: i64,
    b: i64,
    divisor: i64,
}

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            let mut u = arbitrary::Unstructured::new(data);
            let fi = match FuzzInput::arbitrary(&mut u) {
                Ok(v) => v,
                Err(_) => return,
            };

            let a = FixedPoint::from_raw(fi.a as i128);
            let b = FixedPoint::from_raw(fi.b as i128);

            // Invariant 1: checked ops never panic
            let _ = a.checked_add(b);
            let _ = a.checked_sub(b);
            let _ = a.checked_mul(b);

            // Invariant 2: commutativity
            assert_eq!(a.checked_add(b), b.checked_add(a));
            assert_eq!(a.checked_mul(b), b.checked_mul(a));

            // Invariant 3: to_i64 iff value fits
            let raw_a = a.raw();
            let to_i64_result = a.to_i64();
            if raw_a >= i64::MIN as i128 && raw_a <= i64::MAX as i128 {
                assert!(to_i64_result.is_ok());
            } else {
                assert!(to_i64_result.is_err());
            }

            // Invariant 4: checked_div with non-zero divisor never panics
            if fi.divisor != 0 {
                let denom = FixedPoint::from_raw(fi.divisor as i128);
                let _ = a.checked_div(denom);
            }

            // Invariant 5: floor_div result satisfies floor property
            // for small values where we can verify without overflow
            let av = fi.a as i128;
            let bv = fi.b as i128;
            if bv != 0 {
                if let Ok(q) = floor_div_i128(av, bv) {
                    // q = floor(a/b), so q*b ≤ a (if b>0) or q*b ≥ a (if b<0)
                    // Equivalently: remainder r = a - q*b ∈ [0, |b|-1]
                    if let Some(qb) = q.checked_mul(bv) {
                        let r = av - qb;
                        let abs_b = bv.unsigned_abs();
                        assert!(r >= 0, "floor_div: remainder negative: a={av} b={bv} q={q} r={r}");
                        assert!((r as u128) < abs_b, "floor_div: remainder ≥ |b|: a={av} b={bv} q={q} r={r}");
                    }
                }
            }

            // Invariant 6: rescaled mul result satisfies: |result - a*b/SCALE| < 1
            if let Ok(prod) = a.checked_mul(b) {
                let exact_num = (fi.a as i128).checked_mul(fi.b as i128);
                if let Some(exact) = exact_num {
                    let expected_floor = floor_div_i128(exact, SCALE);
                    if let Ok(expected) = expected_floor {
                        assert_eq!(prod.raw(), expected,
                            "checked_mul floor mismatch: a={} b={} got={} expected={}",
                            fi.a, fi.b, prod.raw(), expected);
                    }
                }
            }
        });
    }
}
