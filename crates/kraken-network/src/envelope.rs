//! The answers of Kraken: `{"error": ["EAPI:Invalid key", ...], "result": ...}`

use crate::KrakenError;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(Deserialize)]
struct Envelope {
    #[serde(default)]
    error: Vec<String>,
    #[serde(default)]
    result: Option<Value>,
}

/// Maps an answer of Kraken to its `result`, or to the error it holds
pub(crate) fn decode<O: DeserializeOwned>(status: u16, text: &str) -> Result<O, KrakenError> {
    if status != 200 {
        return Err(KrakenError::Network {
            status,
            body: text.to_string(),
        });
    }

    let envelope: Envelope = serde_json::from_str(text).map_err(KrakenError::decode)?;

    if !envelope.error.is_empty() {
        return Err(KrakenError::Api {
            status,
            body: envelope.error.join("; "),
        });
    }

    match envelope.result {
        None | Some(Value::Null) => Err(KrakenError::Api {
            status,
            body: text.to_string(),
        }),
        Some(result) => serde_json::from_value(result).map_err(KrakenError::decode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn unwraps_the_result() {
        let value: HashMap<String, i32> = decode(200, r#"{"error":[],"result":{"a":1}}"#).unwrap();
        assert_eq!(value["a"], 1);
    }

    #[test]
    fn maps_the_errors() {
        assert_eq!(
            decode::<Value>(
                200,
                r#"{"error":["EAPI:Invalid key","EGeneral:Too many requests"]}"#
            ),
            Err(KrakenError::Api {
                status: 200,
                body: "EAPI:Invalid key; EGeneral:Too many requests".into()
            })
        );
        assert_eq!(
            decode::<Value>(200, r#"{"error":[]}"#),
            Err(KrakenError::Api {
                status: 200,
                body: r#"{"error":[]}"#.into()
            })
        );
        assert_eq!(
            decode::<Value>(520, "unavailable"),
            Err(KrakenError::Network {
                status: 520,
                body: "unavailable".into()
            })
        );
        assert!(matches!(
            decode::<Value>(200, "not json"),
            Err(KrakenError::Decode { .. })
        ));
    }
}
