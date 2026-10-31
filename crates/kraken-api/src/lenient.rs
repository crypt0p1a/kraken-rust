//! Lenient deserializers: like CEX.IO, Kraken sends its decimals as strings ("1234.5") and some
//! identifiers as numbers. Shared with `cex-spot-trading`.

// kept identical to the CEX.IO module, to merge both: some deserializers are unused by Kraken
#![allow(dead_code)]

use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Deserialize)]
#[serde(untagged)]
enum Lenient<T> {
    Value(T),
    Text(String),
}

pub(crate) fn number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + FromStr,
    T::Err: Display,
{
    match Lenient::<T>::deserialize(deserializer)? {
        Lenient::Value(value) => Ok(value),
        Lenient::Text(text) => text.trim().parse().map_err(D::Error::custom),
    }
}

pub(crate) fn opt_number<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + FromStr,
    T::Err: Display,
{
    match Option::<Lenient<T>>::deserialize(deserializer)? {
        None => Ok(None),
        Some(Lenient::Value(value)) => Ok(Some(value)),
        Some(Lenient::Text(text)) if text.trim().is_empty() => Ok(None),
        Some(Lenient::Text(text)) => text.trim().parse().map(Some).map_err(D::Error::custom),
    }
}

/// A percentage: 0 when the value isn't a number
pub(crate) fn percentage<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    Ok(match Lenient::<f32>::deserialize(deserializer)? {
        Lenient::Value(value) => value,
        Lenient::Text(text) => text.trim().parse().unwrap_or(0.0),
    })
}

pub(crate) fn opt_percentage<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<f32>, D::Error> {
    Ok(match Option::<Lenient<f32>>::deserialize(deserializer)? {
        None => None,
        Some(Lenient::Value(value)) => Some(value),
        Some(Lenient::Text(text)) => Some(text.trim().parse().unwrap_or(0.0)),
    })
}

fn primitive_to_string<E: Error>(value: Value) -> Result<Option<String>, E> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text)),
        Value::Number(number) => Ok(Some(number.to_string())),
        Value::Bool(boolean) => Ok(Some(boolean.to_string())),
        other => Err(E::custom(format!("expected a primitive, got {other}"))),
    }
}

pub(crate) fn string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    primitive_to_string(Value::deserialize(deserializer)?)?
        .ok_or_else(|| D::Error::custom("expected a primitive, got null"))
}

pub(crate) fn opt_string<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    primitive_to_string(Value::deserialize(deserializer)?)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Sample {
        #[serde(deserialize_with = "super::number")]
        number: f64,
        #[serde(default, deserialize_with = "super::opt_number")]
        optional: Option<i64>,
        #[serde(deserialize_with = "super::percentage")]
        percentage: f32,
        #[serde(deserialize_with = "super::string")]
        text: String,
    }

    #[test]
    fn accepts_numbers_as_strings() {
        let sample: Sample = serde_json::from_str(
            r#"{"number":"12.5","optional":"42","percentage":"-1.5","text":1234}"#,
        )
        .unwrap();

        assert_eq!(sample.number, 12.5);
        assert_eq!(sample.optional, Some(42));
        assert_eq!(sample.percentage, -1.5);
        assert_eq!(sample.text, "1234");
    }

    #[test]
    fn accepts_plain_values_and_missing_optionals() {
        let sample: Sample =
            serde_json::from_str(r#"{"number":12.5,"percentage":"not a number","text":"abc"}"#)
                .unwrap();

        assert_eq!(sample.number, 12.5);
        assert_eq!(sample.optional, None);
        assert_eq!(sample.percentage, 0.0);
        assert_eq!(sample.text, "abc");
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Optionals {
        #[serde(default, deserialize_with = "super::opt_number")]
        number: Option<f64>,
        #[serde(default, deserialize_with = "super::opt_string")]
        text: Option<String>,
    }

    #[test]
    fn reads_null_and_empty_strings_as_none() {
        for json in [
            r#"{}"#,
            r#"{"number":null,"text":null}"#,
            r#"{"number":"","text":null}"#,
            r#"{"number":"   "}"#,
        ] {
            let decoded: Optionals = serde_json::from_str(json).unwrap();
            assert_eq!(decoded.number, None, "{json}");
            assert_eq!(decoded.text, None, "{json}");
        }
    }

    #[test]
    fn trims_the_numbers_and_converts_the_primitives_to_strings() {
        let decoded: Optionals =
            serde_json::from_str(r#"{"number":" 12.5 ","text":true}"#).unwrap();
        assert_eq!(decoded.number, Some(12.5));
        assert_eq!(decoded.text.as_deref(), Some("true"));

        let decoded: Optionals = serde_json::from_str(r#"{"number":-3,"text":1.5}"#).unwrap();
        assert_eq!(decoded.number, Some(-3.0));
        assert_eq!(decoded.text.as_deref(), Some("1.5"));
    }

    #[test]
    fn rejects_what_is_not_a_number_or_a_primitive() {
        assert!(serde_json::from_str::<Optionals>(r#"{"number":"abc"}"#).is_err());
        assert!(serde_json::from_str::<Optionals>(r#"{"number":[1]}"#).is_err());
        assert!(serde_json::from_str::<Optionals>(r#"{"text":{"a":1}}"#).is_err());
        assert!(serde_json::from_str::<Optionals>(r#"{"text":[1]}"#).is_err());
        assert!(
            serde_json::from_str::<Sample>(r#"{"number":"1","percentage":1,"text":null}"#).is_err()
        );
    }

    #[test]
    fn reads_the_integers() {
        #[derive(Deserialize)]
        struct Integer(#[serde(deserialize_with = "super::number")] i64);

        assert_eq!(serde_json::from_str::<Integer>(r#""42""#).unwrap().0, 42);
        assert_eq!(serde_json::from_str::<Integer>("42").unwrap().0, 42);
        assert!(serde_json::from_str::<Integer>(r#""4.2""#).is_err());
    }
}
