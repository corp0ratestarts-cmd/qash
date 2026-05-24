use serde_json::Value;

pub fn parse_vectors_file(file_name: &str, contents: &str) -> Vec<Value> {
    let root: Value = serde_json::from_str(contents)
        .unwrap_or_else(|error| panic!("{file_name}: invalid JSON: {error}"));
    let vectors = root["vectors"]
        .as_array()
        .unwrap_or_else(|| panic!("{file_name}: missing or non-array top-level 'vectors' field"));
    vectors.clone()
}

pub fn vector_by_id<'a>(vectors: &'a [Value], id: &str, file_name: &str) -> &'a Value {
    vectors
        .iter()
        .find(|vector| vector.get("id").and_then(Value::as_str) == Some(id))
        .unwrap_or_else(|| panic!("{file_name}: missing vector id '{id}'"))
}

pub fn required_str<'a>(node: &'a Value, key: &str, file_name: &str, vector_id: &str) -> &'a str {
    node.get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{file_name}:{vector_id}: missing or non-string '{key}'"))
}

pub fn optional_str<'a>(
    node: &'a Value,
    key: &str,
    file_name: &str,
    vector_id: &str,
) -> Option<&'a str> {
    match node.get(key) {
        Some(value) => Some(value.as_str().unwrap_or_else(|| {
            panic!("{file_name}:{vector_id}: non-string optional field '{key}'")
        })),
        None => None,
    }
}

pub fn required_u64(node: &Value, key: &str, file_name: &str, vector_id: &str) -> u64 {
    node.get(key)
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("{file_name}:{vector_id}: missing or non-u64 '{key}'"))
}

pub fn required_i64(node: &Value, key: &str, file_name: &str, vector_id: &str) -> i64 {
    node.get(key)
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("{file_name}:{vector_id}: missing or non-i64 '{key}'"))
}

pub fn required_bool(node: &Value, key: &str, file_name: &str, vector_id: &str) -> bool {
    node.get(key)
        .and_then(Value::as_bool)
        .unwrap_or_else(|| panic!("{file_name}:{vector_id}: missing or non-bool '{key}'"))
}

pub fn optional_bool(node: &Value, key: &str, file_name: &str, vector_id: &str) -> Option<bool> {
    match node.get(key) {
        Some(value) => {
            Some(value.as_bool().unwrap_or_else(|| {
                panic!("{file_name}:{vector_id}: non-bool optional field '{key}'")
            }))
        }
        None => None,
    }
}

pub fn required_object<'a>(
    node: &'a Value,
    key: &str,
    file_name: &str,
    vector_id: &str,
) -> &'a Value {
    node.get(key)
        .filter(|value| value.is_object())
        .unwrap_or_else(|| panic!("{file_name}:{vector_id}: missing or non-object '{key}'"))
}
