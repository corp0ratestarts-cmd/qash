// cascade_kat.rs — Known-Answer Tests for the 7-layer cascade (h_cascade, h_cascade_keyed).
//
// Vectors are loaded from tests/vectors/cascade_kat.json at compile time via include_str!.
// Cross-ISA CI runs this test on x86_64, aarch64, and riscv64gc; any bit-level divergence
// in the cascade implementation will fail here before it can pollute the state-root chain.
//
// DO NOT regenerate these vectors automatically. If the cascade spec changes, regenerate
// on ALL three authorized ISAs and verify the outputs are bitwise identical before committing.
//
// Spec: docs/spec/07_hash_cascade.md
// Coverage: proofs/COVERAGE.md — "H_cascade bitwise-identical across Tier A ISAs"

use qash_consensus::cascade::{h_cascade, h_cascade_keyed};

const CASCADE_KAT_JSON: &str = include_str!("../../../tests/vectors/cascade_kat.json");

// ---------------------------------------------------------------------------
// Minimal JSON extraction — avoids a proc-macro dependency in CI.
// We rely on serde_json for structured access.
// ---------------------------------------------------------------------------

fn hex_decode(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd-length hex string: {}", s);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("invalid hex digit"))
        .collect()
}

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

// ---------------------------------------------------------------------------
// KAT runner
// ---------------------------------------------------------------------------

#[test]
fn cascade_kat_all_vectors() {
    let root: serde_json::Value =
        serde_json::from_str(CASCADE_KAT_JSON).expect("cascade_kat.json must be valid JSON");

    let vectors = root
        .as_array()
        .expect("cascade_kat.json root must be an array");
    assert!(!vectors.is_empty(), "no KAT vectors found");

    let mut passed = 0usize;

    for entry in vectors {
        let label = entry["label"]
            .as_str()
            .expect("each entry must have a label");

        let input_hex = entry["input_hex"].as_str().expect("missing input_hex");
        let input = hex_decode(input_hex);

        // --- h_cascade (unkeyed) ---
        let expected_cascade_hex = entry["h_cascade_hex"]
            .as_str()
            .expect("missing h_cascade_hex");
        let expected_cascade = hex_decode(expected_cascade_hex);
        assert_eq!(
            expected_cascade.len(),
            64,
            "KAT[{}]: h_cascade_hex must be 64 bytes",
            label
        );

        let got_cascade = h_cascade(&input);
        assert_eq!(
            got_cascade.as_slice(),
            expected_cascade.as_slice(),
            "KAT[{}] h_cascade MISMATCH\n  expected: {}\n  got:      {}",
            label,
            expected_cascade_hex,
            hex_encode(&got_cascade)
        );

        // --- h_cascade_keyed ---
        let context_key_hex = entry["context_key_hex"]
            .as_str()
            .expect("missing context_key_hex");
        let context_key = hex_decode(context_key_hex);

        let expected_keyed_hex = entry["h_cascade_keyed_hex"]
            .as_str()
            .expect("missing h_cascade_keyed_hex");
        let expected_keyed = hex_decode(expected_keyed_hex);
        assert_eq!(
            expected_keyed.len(),
            64,
            "KAT[{}]: h_cascade_keyed_hex must be 64 bytes",
            label
        );

        let got_keyed = h_cascade_keyed(&context_key, &input);
        assert_eq!(
            got_keyed.as_slice(),
            expected_keyed.as_slice(),
            "KAT[{}] h_cascade_keyed MISMATCH\n  context_key: {}\n  expected:    {}\n  got:         {}",
            label,
            context_key_hex,
            expected_keyed_hex,
            hex_encode(&got_keyed)
        );

        passed += 1;
    }

    assert!(
        passed >= 3,
        "expected at least 3 KAT vectors, got {}",
        passed
    );
}

/// Regression guard: unkeyed empty-input vector matches stored value.
/// Duplicates the first entry in the JSON for fast single-test verification.
#[test]
fn cascade_kat_empty_input_fast() {
    let root: serde_json::Value = serde_json::from_str(CASCADE_KAT_JSON).unwrap();
    let empty = root
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["label"] == "empty")
        .expect("cascade_kat.json must contain an 'empty' entry");

    let got = h_cascade(b"");
    let expected = hex_decode(empty["h_cascade_hex"].as_str().unwrap());
    assert_eq!(
        got.as_slice(),
        expected.as_slice(),
        "h_cascade(\"\") regression"
    );
}
