// Fuzz target: h_cascade and h_cascade_keyed — Domain A pure functions.
//
// Invariants verified on every input:
//   1. Output length is always exactly 64 bytes (no panic, no truncation)
//   2. Determinism: identical inputs produce identical outputs
//   3. No panic on any byte sequence (length 0 to honggfuzz max)
//
// Run: cargo hfuzz run cascade_fuzz  (from fuzz/)
// Corpus: fuzz/hfuzz_workspace/cascade_fuzz/input/

use honggfuzz::fuzz;
use qash_consensus::cascade::{h_cascade, h_cascade_keyed};

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            // Invariant 1+3: h_cascade must never panic and must return 64 bytes.
            let out = h_cascade(data);
            assert_eq!(out.len(), 64);

            // Invariant 2: determinism.
            assert_eq!(out, h_cascade(data));

            // Split data: first 4 bytes as context key, rest as payload.
            if let Some((key, rest)) = data.split_first_chunk::<4>() {
                let keyed = h_cascade_keyed(key, rest);
                assert_eq!(keyed.len(), 64);
                // Determinism for keyed path.
                assert_eq!(keyed, h_cascade_keyed(key, rest));
            }

            // Empty key must equal unkeyed (spec §4).
            assert_eq!(h_cascade(data), h_cascade_keyed(&[], data));
        });
    }
}
