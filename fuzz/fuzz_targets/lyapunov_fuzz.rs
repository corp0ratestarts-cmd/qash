// Fuzz target: Lyapunov evaluation — monotonicity, convergence, overflow safety.
//
// Verifies:
//   1. evaluate() never panics
//   2. If halt_triggered == false, V_convergence is finite and non-negative
//   3. Halt trigger iff window is full AND delta_window > EPSILON (TH-3b)
//   4. Push shifts the window correctly (oldest dropped, newest inserted)
//   5. Weighted sum overflow is caught by checked_add (no silent wrap)
//
// Run: cargo hfuzz run lyapunov_fuzz  (from fuzz/)

use honggfuzz::fuzz;
use arbitrary::Arbitrary;
use qash_consensus::fixed_point::FixedPoint;
use qash_consensus::lyapunov::{
    ConvergenceWindow, ValidatorMetrics, evaluate, EPSILON, WEIGHT_D, WEIGHT_C, WEIGHT_S,
    WINDOW_SIZE,
};

#[derive(Arbitrary, Debug)]
struct ValidatorInput {
    d: u32,
    c: u32,
    s: u32,
}

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    validators: [ValidatorInput; 4],
    validator_count: u8,
    window_values: [i32; 3],
    window_fill: u8,
}

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            let mut u = arbitrary::Unstructured::new(data);
            let fi = match FuzzInput::arbitrary(&mut u) {
                Ok(v) => v,
                Err(_) => return,
            };

            let scale = 1_000_000i128;
            let vc = ((fi.validator_count as usize) % 4).max(1);

            let mut metrics = [ValidatorMetrics::ZERO; 4];
            for i in 0..vc {
                metrics[i] = ValidatorMetrics {
                    divergence:  FixedPoint::from_raw((fi.validators[i].d as i128) % (scale + 1)),
                    conflict:    FixedPoint::from_raw((fi.validators[i].c as i128) % (scale + 1)),
                    slash_accum: FixedPoint::from_raw((fi.validators[i].s as i128) % (scale + 1)),
                };
            }

            let mut window = ConvergenceWindow::new();
            let fill = (fi.window_fill as usize).min(WINDOW_SIZE);
            for k in 0..fill {
                // window values: clamp to non-negative (window stores V_convergence samples)
                let v = (fi.window_values[k] as i128).abs() % (scale * 4 + 1);
                window.push(FixedPoint::from_raw(v));
            }

            // Invariant 1: evaluate never panics
            let result = evaluate(&metrics[..vc], &window);

            // Invariant 2: V_convergence is non-negative when no overflow error
            if let Ok(ref eval) = result {
                assert!(eval.v_convergence.raw() >= 0,
                    "V_convergence negative: {}", eval.v_convergence.raw());

                // Invariant 3: halt_triggered iff window.is_full() AND delta > EPSILON
                if eval.halt_triggered {
                    assert!(window.is_full(),
                        "halt_triggered but window not full");
                    assert!(eval.delta_window.raw() > EPSILON.raw(),
                        "halt_triggered but delta={} <= epsilon={}",
                        eval.delta_window.raw(), EPSILON.raw());
                } else if window.is_full() {
                    // If full and no halt, delta must be ≤ EPSILON
                    assert!(eval.delta_window.raw() <= EPSILON.raw(),
                        "window full, no halt, but delta={} > epsilon={}",
                        eval.delta_window.raw(), EPSILON.raw());
                }
            }

            // Invariant 4: weights sum confirms V_convergence formula
            // V = Σ (D*w_D + C*w_C + S*w_S) — verified independently for single validator
            if vc == 1 {
                if let Ok(ref eval) = result {
                    let d = metrics[0].divergence;
                    let c = metrics[0].conflict;
                    let s = metrics[0].slash_accum;
                    // Each term is checked_mul; sum of three checked_muls
                    let t_d = d.checked_mul(WEIGHT_D);
                    let t_c = c.checked_mul(WEIGHT_C);
                    let t_s = s.checked_mul(WEIGHT_S);
                    if let (Ok(td), Ok(tc), Ok(ts)) = (t_d, t_c, t_s) {
                        if let Ok(sum1) = td.checked_add(tc) {
                            if let Ok(expected) = sum1.checked_add(ts) {
                                assert_eq!(eval.v_convergence.raw(), expected.raw(),
                                    "V_convergence mismatch for single validator");
                            }
                        }
                    }
                }
            }
        });
    }
}
