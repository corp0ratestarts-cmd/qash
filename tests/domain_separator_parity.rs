// Domain-separator parity gate.
//
// The 7 domain separators exist in two canonical locations:
//   1. src/crypto/cascade.rs  (DOM_SEP_L1 … DOM_SEP_L7 — used by the hash function)
//   2. GENESIS_CONSTANTS.toml [crypto.cascade.domain_separators] (normative spec record)
//
// This test asserts byte-for-byte equality between both sources.
// Renaming a separator in one place without updating the other → immediate CI failure.

use qash::crypto::cascade::{
    DOM_SEP_L1, DOM_SEP_L2, DOM_SEP_L3, DOM_SEP_L4,
    DOM_SEP_L5, DOM_SEP_L6, DOM_SEP_L7,
};

fn extract_domain_separators(toml: &str) -> Vec<String> {
    let mut in_array = false;
    let mut results = Vec::new();
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with("domain_separators") {
            in_array = true;
            continue;
        }
        if in_array {
            if t.starts_with(']') {
                break;
            }
            // Lines like:  "QASH:CASCADE:L1:PARALLEL",
            if t.starts_with('"') {
                let inner = t.trim_matches(|c| c == '"' || c == ',' || c == ' ');
                results.push(inner.to_string());
            }
        }
    }
    results
}

#[test]
fn domain_separators_match_genesis_toml() {
    let toml = std::fs::read_to_string("GENESIS_CONSTANTS.toml")
        .expect("GENESIS_CONSTANTS.toml not found — run from workspace root");

    let toml_seps = extract_domain_separators(&toml);
    assert_eq!(toml_seps.len(), 7, "expected 7 domain separators in GENESIS_CONSTANTS.toml");

    let code_seps: &[&[u8]] = &[
        DOM_SEP_L1, DOM_SEP_L2, DOM_SEP_L3, DOM_SEP_L4,
        DOM_SEP_L5, DOM_SEP_L6, DOM_SEP_L7,
    ];

    for (i, (code, toml_str)) in code_seps.iter().zip(toml_seps.iter()).enumerate() {
        let code_str = std::str::from_utf8(code).expect("dom sep is valid UTF-8");
        assert_eq!(
            code_str, toml_str.as_str(),
            "domain separator L{} mismatch: cascade.rs has {:?}, GENESIS_CONSTANTS.toml has {:?}",
            i + 1, code_str, toml_str
        );
    }
}
