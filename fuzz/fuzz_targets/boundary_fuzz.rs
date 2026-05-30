// Fuzz target: Domain A/B boundary — CommitmentFrame encode/decode roundtrip.
//
// Invariants verified on every input:
//   1. decode(arbitrary) never panics — only returns Ok or Err
//   2. encode-then-decode roundtrip is identity for valid frames
//   3. decode(encode(frame)) == frame for all constructible frames
//   4. No panic on any byte sequence (any length, any content)
//
// Run: cargo hfuzz run boundary_fuzz  (from fuzz/)
// Corpus: fuzz/hfuzz_workspace/boundary_fuzz/input/

use honggfuzz::fuzz;
use qash_pal::commitment_transport::{
    CommitmentFrame, CommitmentFrameError, COMMITMENT_FRAME_BYTES, COMMITMENT_FRAME_MAGIC,
};
// Note: qash-pal root is crates/pal/src/root.rs; commitment_transport is a pub mod there.

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            // Invariant 1+4: decode must never panic regardless of input.
            let result = CommitmentFrame::decode(data);

            // Invariant 2: if decode succeeds, re-encoding must reproduce the same bytes.
            if let Ok(frame) = result {
                let encoded = frame.encode();
                let redecoded = CommitmentFrame::decode(&encoded);
                match redecoded {
                    Ok(f2) => assert_eq!(frame, f2, "encode->decode roundtrip not identity"),
                    Err(_) => panic!("encode produced bytes that decode rejected"),
                }
            }

            // Invariant 3: construct a valid frame from fuzzed bytes and verify roundtrip.
            // Use the first COMMITMENT_FRAME_BYTES bytes of data padded with zeros.
            if data.len() >= 8 {
                let mut padded = [0u8; COMMITMENT_FRAME_BYTES];
                // Force a valid magic header.
                padded[..8].copy_from_slice(&COMMITMENT_FRAME_MAGIC);
                let payload_len = (data.len() - 8).min(COMMITMENT_FRAME_BYTES - 8);
                padded[8..8 + payload_len].copy_from_slice(&data[8..8 + payload_len]);

                // This must always succeed since we injected a valid magic.
                let frame = CommitmentFrame::decode(&padded)
                    .expect("decode with valid magic must not fail");
                let reenc = frame.encode();
                let frame2 = CommitmentFrame::decode(&reenc)
                    .expect("decode of encode must not fail");
                assert_eq!(frame, frame2);
            }

            // Wrong-length inputs must return InvalidLength (not panic).
            if data.len() != COMMITMENT_FRAME_BYTES {
                match CommitmentFrame::decode(data) {
                    Err(CommitmentFrameError::InvalidLength) => {}
                    Err(CommitmentFrameError::InvalidMagic) => {}
                    Ok(_) => {
                        // Should only reach here if length happens to match.
                        panic!("wrong-length decode returned Ok");
                    }
                }
            }
        });
    }
}
