#![no_main]
// Fuzz target: decode_full_state — Domain A pure function.
//
// Invariants verified on every input:
//   1. decode_full_state never panics on arbitrary bytes
//   2. If decode succeeds, encode(decode(x)) must round-trip cleanly
//   3. Re-decoded state must produce identical fields
//
// Run: cargo fuzz run decode_fuzz -- -max_total_time=60

use libfuzzer_sys::fuzz_target;
use qash_consensus::transition::{
    decode_full_state, encode_full_state_into, FULL_STATE_MAX_BYTES,
};

fuzz_target!(|data: &[u8]| {
    match decode_full_state(data) {
        Ok(state) => {
            // Invariant 2: round-trip via encode must succeed.
            let mut buf = [0u8; FULL_STATE_MAX_BYTES];
            let len = encode_full_state_into(&state, &mut buf);

            // Invariant 3: re-decode must succeed and produce the same state.
            let state2 = decode_full_state(&buf[..len])
                .expect("encode→decode roundtrip must succeed on a valid state");

            assert_eq!(state.epoch, state2.epoch);
            assert_eq!(state.halt_reason, state2.halt_reason);
            assert_eq!(state.validator_count, state2.validator_count);
            assert_eq!(state.entropy_seed, state2.entropy_seed);
            assert_eq!(state.state_root, state2.state_root);

            for i in 0..state.validator_count as usize {
                assert_eq!(
                    state.validators[i].divergence.raw(),
                    state2.validators[i].divergence.raw()
                );
                assert_eq!(
                    state.validators[i].slash_accum.raw(),
                    state2.validators[i].slash_accum.raw()
                );
                assert_eq!(state.nonces[i], state2.nonces[i]);
            }
        }
        Err(_) => {
            // Rejection of invalid bytes is the expected common case.
        }
    }
});
