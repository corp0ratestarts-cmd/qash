#![no_main]

use libfuzzer_sys::fuzz_target;
use qash_consensus::transaction::{parse_tx0, tx_id, sort_key, TX0_WIRE_BYTES};

fuzz_target!(|data: &[u8]| {
    // Exercise the parser against arbitrary input.
    let Ok((tx, consumed)) = parse_tx0(data) else {
        return;
    };
    assert_eq!(consumed, TX0_WIRE_BYTES);

    // On a successful parse, also exercise tx_id and sort_key — these must
    // never panic regardless of the content of a validly-structured envelope.
    if data.len() >= TX0_WIRE_BYTES {
        let mut raw = [0u8; TX0_WIRE_BYTES];
        raw.copy_from_slice(&data[..TX0_WIRE_BYTES]);
        let id = tx_id(&raw);
        let seed = [0u8; 32];
        let _key = sort_key(&seed, &id);
    }

    // Confirm that re-parsing the same bytes yields the same result
    // (idempotent parse → no state mutation inside the parser).
    let Ok((tx2, _)) = parse_tx0(data) else {
        return;
    };
    assert_eq!(tx.nonce, tx2.nonce);
    assert_eq!(tx.author_id, tx2.author_id);
});
