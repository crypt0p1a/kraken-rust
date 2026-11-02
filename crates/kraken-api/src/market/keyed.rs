//! The answers keyed by the Kraken name of the requested pair: `{"XXBTZUSD": ..., "last": ...}`

use crate::KrakenError;
use serde_json::{Map, Value};

/// The single pair of the answer, its value, and the `last` beside it when there is one
pub(crate) fn single_pair(
    mut result: Map<String, Value>,
) -> Result<(String, Value, Option<Value>), KrakenError> {
    let last = result.remove("last");
    let mut entries = result.into_iter();
    let (pair, value) = entries.next().ok_or_else(|| KrakenError::Decode {
        reason: "no pair in the answer".into(),
    })?;
    if let Some((other, _)) = entries.next() {
        return Err(KrakenError::Decode {
            reason: format!("expected a single pair, got {pair} and {other}"),
        });
    }
    Ok((pair, value, last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn splits_the_pair_and_last() {
        let (pair, value, last) = single_pair(map(json!({"XXBTZUSD": [1], "last": "12"}))).unwrap();
        assert_eq!(pair, "XXBTZUSD");
        assert_eq!(value, json!([1]));
        assert_eq!(last, Some(json!("12")));

        let (_, _, last) = single_pair(map(json!({"XXBTZUSD": {}}))).unwrap();
        assert_eq!(last, None);
    }

    #[test]
    fn rejects_no_pair_or_several() {
        assert!(matches!(
            single_pair(map(json!({"last": 1}))),
            Err(KrakenError::Decode { .. })
        ));
        assert!(matches!(
            single_pair(map(json!({"A": 1, "B": 2}))),
            Err(KrakenError::Decode { .. })
        ));
    }
}
