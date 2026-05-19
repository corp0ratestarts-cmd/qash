// Fuzz target: individual encoding/decoding functions in encoding.rs.
//
// decode_fuzz.rs covers the top-level decode_full_state roundtrip. This target
// exercises the building-block functions directly so that invariant violations
// in the sub-parsers are reachable without needing a structurally valid full
// state buffer as input.
//
// Invariants verified:
//   1. decode_state_header never panics on any 52-byte input
//   2. encode_state_header → decode_state_header roundtrip is lossless for valid halt codes
//   3. decode_validator_dynamic never panics on any 48-byte input
//   4. encode_validator_dynamic → decode_validator_dynamic roundtrip is lossless for in-range values
//   5. decode_leaf_index is total (never fails) on any 48-byte input
//   6. compute_leaf_index → decode_leaf_index roundtrip recovers original fields
//
// Run: cargo hfuzz run encoding_fuzz  (from fuzz/)

use honggfuzz::fuzz;
use arbitrary::Arbitrary;
use qash_consensus::encoding::{
    decode_leaf_index, decode_state_header, decode_validator_dynamic,
    encode_state_header, encode_validator_dynamic, compute_leaf_index,
    STATE_HEADER_SIZE, VALIDATOR_DYNAMIC_SIZE,
};
use qash_consensus::fixed_point::FixedPoint;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    // Raw bytes for parser invariant checks.
    header_bytes: [u8; STATE_HEADER_SIZE as usize],
    validator_bytes: [u8; VALIDATOR_DYNAMIC_SIZE as usize],
    leaf_bytes: [u8; 48],

    // Structured fields for roundtrip checks.
    epoch: u64,
    validator_count: u32,
    halt_reason: u8,
    entropy_seed: [u8; 32],

    divergence_raw: i32,
    conflict_raw: i32,
    slash_raw: u32,

    validator_id: u64,
    leaf_epoch: u64,
    leaf_seed: [u8; 32],
}

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            let mut u = arbitrary::Unstructured::new(data);
            let fi = match FuzzInput::arbitrary(&mut u) {
                Ok(v) => v,
                Err(_) => return,
            };

            // ---- Invariant 1: decode_state_header never panics ----
            let _ = decode_state_header(&fi.header_bytes);

            // ---- Invariant 2: encode → decode roundtrip for valid halt codes ----
            // Valid halt codes are 0 (None) and 1–4 (protocol halt reasons).
            let halt = fi.halt_reason % 5;
            let mut hdr_buf = [0u8; STATE_HEADER_SIZE as usize];
            encode_state_header(
                fi.epoch,
                fi.validator_count,
                halt,
                &fi.entropy_seed,
                &mut hdr_buf,
            );
            let decoded = decode_state_header(&hdr_buf)
                .expect("encode_state_header output must round-trip through decode");
            assert_eq!(decoded.0, fi.epoch, "epoch mismatch after header roundtrip");
            assert_eq!(decoded.1, fi.validator_count, "validator_count mismatch after header roundtrip");
            assert_eq!(decoded.2, halt, "halt_reason mismatch after header roundtrip");
            assert_eq!(decoded.3, fi.entropy_seed, "entropy_seed mismatch after header roundtrip");

            // ---- Invariant 3: decode_validator_dynamic never panics ----
            let _ = decode_validator_dynamic(&fi.validator_bytes);

            // ---- Invariant 4: encode → decode roundtrip for in-range validator fields ----
            let scale = 1_000_000i128;
            let d_raw = (fi.divergence_raw as i128).abs() % (scale + 1);
            let c_raw = (fi.conflict_raw as i128).abs() % (scale + 1);
            // slash_accum must fit in i64 and be non-negative
            let s_raw = (fi.slash_raw as i128) % (i64::MAX as i128 + 1);

            let d = FixedPoint::from_raw(d_raw);
            let c = FixedPoint::from_raw(c_raw);
            let s = FixedPoint::from_raw(s_raw);

            let mut vd_buf = [0u8; VALIDATOR_DYNAMIC_SIZE as usize];
            encode_validator_dynamic(d, c, s, &mut vd_buf);
            let (d2, c2, s2) = decode_validator_dynamic(&vd_buf)
                .expect("encode_validator_dynamic output must round-trip through decode");
            assert_eq!(d2.raw(), d.raw(), "divergence mismatch after validator roundtrip");
            assert_eq!(c2.raw(), c.raw(), "conflict mismatch after validator roundtrip");
            assert_eq!(s2.raw(), s.raw(), "slash_accum mismatch after validator roundtrip");

            // ---- Invariant 5: decode_leaf_index is total ----
            let _ = decode_leaf_index(&fi.leaf_bytes);

            // ---- Invariant 6: compute_leaf_index → decode_leaf_index roundtrip ----
            let leaf = compute_leaf_index(fi.validator_id, fi.leaf_epoch, &fi.leaf_seed);
            let (vid2, ep2, seed2) = decode_leaf_index(&leaf);
            assert_eq!(vid2, fi.validator_id, "validator_id mismatch after leaf roundtrip");
            assert_eq!(ep2, fi.leaf_epoch, "epoch mismatch after leaf roundtrip");
            assert_eq!(seed2, fi.leaf_seed, "seed mismatch after leaf roundtrip");
        });
    }
}
