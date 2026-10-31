//! Strict decoding: the models must map every field of the answers of Kraken

#![allow(dead_code)]

use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeSet;

/// Decodes `value` with the model `T`, failing on the fields ignored by `T`, on the required
/// fields missing or null, and on the values `T` can't read
pub fn decode_strict<T: DeserializeOwned>(context: &str, value: &Value) -> T {
    let mut ignored = BTreeSet::new();
    let decoded = serde_ignored::deserialize(value.clone(), |path| {
        ignored.insert(path.to_string());
    });

    match decoded {
        Err(error) => panic!("{context}: the model can't read {value}: {error}"),
        Ok(_) if !ignored.is_empty() => {
            panic!(
                "{context}: fields sent by Kraken but ignored by the model: {ignored:?} in {value}"
            )
        }
        Ok(decoded) => decoded,
    }
}

/// The `result` of a fixture of `tests/fixtures`
pub fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    let mut answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["error"], Value::Array(vec![]), "{name}");
    answer["result"].take()
}

/// The whole fixture, envelope included, to be served by a mocked server
pub fn fixture_answer(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
