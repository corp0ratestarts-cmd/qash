// KAT (known-answer test) for the 7-level astronomical cascade.
//
// Golden vectors are stored in tests/vectors/cascade_kat.json.
// This test loads each case, recomputes h_cascade / h_cascade_keyed,
// and asserts bit-for-bit equality.  Any change to cascade.rs that
// alters the output — even a "refactor" — will break this test.

use qash::crypto::cascade::{h_cascade, h_cascade_keyed};

fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("invalid hex"))
        .collect()
}

fn extract_field<'a>(block: &'a str, field: &str) -> &'a str {
    for line in block.lines() {
        let line = line.trim();
        if line.starts_with(&format!("\"{}\"", field)) {
            // e.g. "h_cascade_hex": "abc..."
            let after_colon = line.splitn(2, ':').nth(1).unwrap().trim();
            // strip surrounding quotes and trailing comma
            let inner = after_colon.trim_matches(|c| c == '"' || c == ',');
            return inner;
        }
    }
    panic!("field '{}' not found in block:\n{}", field, block);
}

#[test]
fn cascade_kat_vectors() {
    let json = std::fs::read_to_string("tests/vectors/cascade_kat.json")
        .expect("tests/vectors/cascade_kat.json not found — run from workspace root");

    // Split into per-case blocks by "}," / "}" boundaries
    let cases: Vec<&str> = json
        .split('{')
        .skip(1) // leading empty or array open
        .collect();

    assert!(cases.len() >= 3, "expected at least 3 KAT cases");

    for (i, block) in cases.iter().enumerate() {
        let input_hex    = extract_field(block, "input_hex");
        let expected_hex = extract_field(block, "h_cascade_hex");
        let ctx_key_hex  = extract_field(block, "context_key_hex");
        let expected_keyed_hex = extract_field(block, "h_cascade_keyed_hex");

        let input      = hex_decode(input_hex);
        let ctx_key    = hex_decode(ctx_key_hex);
        let expected   = hex_decode(expected_hex);
        let exp_keyed  = hex_decode(expected_keyed_hex);

        let got        = h_cascade(&input);
        let got_keyed  = h_cascade_keyed(&ctx_key, &input);

        assert_eq!(
            got.as_ref(), expected.as_slice(),
            "h_cascade KAT #{} failed (label in JSON)", i
        );
        assert_eq!(
            got_keyed.as_ref(), exp_keyed.as_slice(),
            "h_cascade_keyed KAT #{} failed (label in JSON)", i
        );
    }
}
