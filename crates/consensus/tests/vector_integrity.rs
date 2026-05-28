// Structural integrity checks for checked-in vector files.
//
// These are non-ignored CI tests. They do NOT rerun protocol logic — that is
// handled by vector_runner.rs and cascade_kat.rs. This file validates that the
// JSON files themselves have the required schema fields and minimum vector
// counts so that drift (accidental truncation, malformed edits) is caught
// before the functional tests even run.

const VECTORS_V1_JSON: &str = include_str!("../../../tests/vectors/vectors.v1.json");
const VECTORS_V1_2_JSON: &str = include_str!("../../../tests/vectors/vectors.v1.2.json");
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
fn vectors_v1_2_has_required_sharded_roots() {
    let root: serde_json::Value =
        serde_json::from_str(VECTORS_V1_2_JSON).expect("vectors.v1.2.json must be valid JSON");

    assert_eq!(
        root["version"].as_str().unwrap_or(""),
        "1.2",
        "vectors.v1.2.json: missing or wrong 'version' field"
    );
    assert_eq!(
        root["shard_count"].as_u64().unwrap_or(0),
        2,
        "vectors.v1.2.json: expected shard_count=2"
    );

    let epochs = root["epochs"]
        .as_array()
        .expect("vectors.v1.2.json: 'epochs' must be an array");
    assert!(
        epochs.len() >= 12,
        "vectors.v1.2.json: expected at least 12 epochs, found {}",
        epochs.len()
    );

    for (i, epoch) in epochs.iter().enumerate() {
        for field in ["state_root", "receipt_root", "efb_root"] {
            let value = epoch[field]
                .as_str()
                .unwrap_or_else(|| panic!("vectors.v1.2.json: epoch[{}] missing '{}'", i, field));
            assert_eq!(
                value.len(),
                64,
                "vectors.v1.2.json: epoch[{}].{} must be 32-byte hex",
                i,
                field
            );
        }
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
        assert!(
            entry["context_key_hex"].as_str().is_some(),
            "cascade_kat.json: entry[{}] missing 'context_key_hex'",
            i
        );
        assert!(
            entry["h_cascade_keyed_hex"].as_str().is_some(),
            "cascade_kat.json: entry[{}] missing 'h_cascade_keyed_hex'",
            i
        );

        for field in ["h_cascade_hex", "h_cascade_keyed_hex"] {
            let value = entry[field]
                .as_str()
                .unwrap_or_else(|| panic!("cascade_kat.json: entry[{}] missing '{}'", i, field));
            assert_eq!(
                value.len(),
                128,
                "cascade_kat.json: entry[{}].{} must be 64-byte hex",
                i,
                field
            );
        }
    }
}
