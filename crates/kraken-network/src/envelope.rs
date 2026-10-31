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

    let value: Value = serde_json::from_str(text).map_err(KrakenError::decode)?;
    if !value.is_object() {
        return Err(KrakenError::Decode {
            reason: format!("expected an object, got {text}"),
        });
    }
    let envelope: Envelope = serde_json::from_value(value).map_err(KrakenError::decode)?;

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

    #[test]
    fn the_errors_win_over_the_result() {
        assert_eq!(
            decode::<Value>(
                200,
                r#"{"error":["EOrder:Insufficient funds"],"result":{"a":1}}"#
            ),
            Err(KrakenError::Api {
                status: 200,
                body: "EOrder:Insufficient funds".into()
            })
        );
    }

    #[test]
    fn a_null_result_is_an_error() {
        assert!(matches!(
            decode::<Value>(200, r#"{"error":[],"result":null}"#),
            Err(KrakenError::Api { .. })
        ));
    }

    #[test]
    fn rejects_the_malformed_envelopes() {
        for text in [
            r#"{"error":"EAPI:Invalid key"}"#,
            r#"{"error":[1]}"#,
            "[]",
            "",
        ] {
            assert!(
                matches!(decode::<Value>(200, text), Err(KrakenError::Decode { .. })),
                "{text}"
            );
        }
    }

    #[test]
    fn keeps_the_body_of_the_http_errors() {
        for status in [400, 403, 404, 429, 500, 502] {
            assert_eq!(
                decode::<Value>(status, r#"{"error":["EService:Unavailable"]}"#),
                Err(KrakenError::Network {
                    status,
                    body: r#"{"error":["EService:Unavailable"]}"#.into()
                })
            );
        }
    }

    #[test]
    fn decodes_the_result_with_the_model() {
        assert!(matches!(
            decode::<HashMap<String, i32>>(200, r#"{"error":[],"result":{"a":"x"}}"#),
            Err(KrakenError::Decode { .. })
        ));
        assert_eq!(decode::<i64>(200, r#"{"error":[],"result":42}"#), Ok(42));
    }
}
