// Structural integrity checks for checked-in vector files.
//
// These are non-ignored CI tests. They do NOT rerun protocol logic — that is
// handled by vector_runner.rs and cascade_kat.rs. This file validates that the
// JSON files themselves have the required schema fields and minimum vector
// counts so that drift (accidental truncation, malformed edits) is caught
// before the functional tests even run.

const VECTORS_V1_JSON: &str = include_str!("../../../tests/vectors/vectors.v1.json");
const CASCADE_KAT_JSON: &str = include_str!("../../../tests/vectors/cascade_kat.json");

#[test]
fn vectors_v1_has_required_fields_and_minimum_count() {
    let root: serde_json::Value =
        serde_json::from_str(VECTORS_V1_JSON).expect("vectors.v1.json must be valid JSON");

    assert_eq!(
        root["version"].as_u64().unwrap_or(0),
        1,
        "vectors.v1.json: missing or wrong 'version' field"
    );
    assert!(
        root["status"].as_str().is_some(),
        "vectors.v1.json: missing 'status' field"
    );

    let vectors = root["vectors"]
        .as_array()
        .expect("vectors.v1.json: 'vectors' must be an array");
    assert!(
        vectors.len() >= 3,
        "vectors.v1.json: expected at least 3 vectors, found {}",
        vectors.len()
    );

    for (i, v) in vectors.iter().enumerate() {
        assert!(
            v["name"].as_str().is_some(),
            "vectors.v1.json: vector[{}] missing 'name'",
            i
        );
        assert!(
            v["pdf_section"].as_str().is_some(),
            "vectors.v1.json: vector[{}] missing 'pdf_section'",
            i
        );
    }
}

#[test]
fn cascade_kat_has_required_fields_and_minimum_count() {
    let vectors: serde_json::Value =
        serde_json::from_str(CASCADE_KAT_JSON).expect("cascade_kat.json must be valid JSON");

    let entries = vectors
        .as_array()
        .expect("cascade_kat.json root must be an array");
    assert!(
        entries.len() >= 2,
        "cascade_kat.json: expected at least 2 entries, found {}",
        entries.len()
    );

    for (i, entry) in entries.iter().enumerate() {
        assert!(
            entry["label"].as_str().is_some(),
            "cascade_kat.json: entry[{}] missing 'label'",
            i
        );
        assert!(
            entry["input_hex"].as_str().is_some(),
            "cascade_kat.json: entry[{}] missing 'input_hex'",
            i
        );
        assert!(
            entry["h_cascade_hex"].as_str().is_some(),
            "cascade_kat.json: entry[{}] missing 'h_cascade_hex'",
            i
        );
    }
}
