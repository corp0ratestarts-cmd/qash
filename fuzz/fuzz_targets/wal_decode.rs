// Fuzz target: recovery_wal decode path — Domain B.
//
// Invariants verified on every input:
//   1. decode_record never panics on arbitrary bytes (any length)
//   2. Parsing a byte buffer as a WAL stream never panics
//
// Run: cargo hfuzz run wal_decode  (from fuzz/)

use honggfuzz::fuzz;
use qash_pal::recovery_wal::{
    decode_record, RECOVERY_RECORD_BYTES, RECOVERY_WAL_MAGIC,
};

fn try_replay_stream(data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    let magic = &data[..8];
    if magic != RECOVERY_WAL_MAGIC {
        return; // bad magic is a known-handled case; no panic allowed
    }
    let mut pos = 8;
    loop {
        let end = pos + RECOVERY_RECORD_BYTES;
        if end > data.len() {
            break;
        }
        let _ = decode_record(&data[pos..end]);
        pos = end;
    }
}

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            // Invariant 1: decode_record never panics on any input (any length).
            let _ = decode_record(data);

            // Invariant 2: stream parsing never panics.
            try_replay_stream(data);
        });
    }
}
