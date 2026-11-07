//! Lenient deserializers: like CEX.IO, Kraken sends its decimals as strings ("1234.5") and some
//! identifiers as numbers. Shared with `cex-spot-trading`.
//!
//! The amounts are read with [`decimal`] / [`opt_decimal`]: the string of Kraken is parsed into
//! an exact [`Decimal`], never through a float, so that "123456789.12345678" stays exact.

// kept identical to the CEX.IO module, to merge both: some deserializers are unused by Kraken
#![allow(dead_code)]

use rust_decimal::Decimal;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::fmt::{self, Display};
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

/// Parses a decimal written by Kraken, e.g. "0.00058823", "-12", "1.5e-7"
pub(crate) fn parse_decimal(text: &str) -> Result<Decimal, String> {
    let text = text.trim();
    Decimal::from_str_exact(text)
        .or_else(|_| Decimal::from_scientific(text))
        .map_err(|error| format!("invalid decimal {text:?}: {error}"))
}

/// A decimal, or None for null and for an empty string
struct DecimalVisitor;

impl<'de> Visitor<'de> for DecimalVisitor {
    type Value = Option<Decimal>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a decimal, as a string or a number")
    }

    fn visit_str<E: Error>(self, text: &str) -> Result<Self::Value, E> {
        if text.trim().is_empty() {
            return Ok(None);
        }
        parse_decimal(text).map(Some).map_err(E::custom)
    }

    fn visit_i64<E: Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Some(Decimal::from(value)))
    }

    fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Some(Decimal::from(value)))
    }

    /// Kraken sends its amounts as strings: only small numbers like the fee tiers ([0, 0.26])
    /// are JSON numbers, whose shortest representation is the text sent by Kraken
    fn visit_f64<E: Error>(self, value: f64) -> Result<Self::Value, E> {
        if !value.is_finite() {
            return Err(E::custom(format!("invalid decimal {value}")));
        }
        parse_decimal(&value.to_string())
            .map(Some)
            .map_err(E::custom)
    }

    fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

/// An exact decimal, sent as a string or as a number
pub(crate) fn decimal<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
    deserializer
        .deserialize_any(DecimalVisitor)?
        .ok_or_else(|| D::Error::custom("expected a decimal, got nothing"))
}

/// An exact decimal: None when missing, null or empty
pub(crate) fn opt_decimal<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error> {
    deserializer.deserialize_option(DecimalVisitor)
}

/// [`decimal`], for the values of the arrays: `<(Lenient, Lenient)>::deserialize`
#[derive(Deserialize)]
pub(crate) struct ExactDecimal(#[serde(deserialize_with = "decimal")] pub Decimal);

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

    #[derive(Deserialize, Debug, PartialEq)]
    struct Amounts {
        #[serde(deserialize_with = "super::decimal")]
        amount: rust_decimal::Decimal,
        #[serde(default, deserialize_with = "super::opt_decimal")]
        optional: Option<rust_decimal::Decimal>,
    }

    fn amounts(json: &str) -> Amounts {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn keeps_every_digit_of_the_large_volumes() {
        // beyond the ~16 significant digits of a f64
        for text in [
            "123456789.12345678",
            "98765432109876.54321",
            "0.000000000123456789",
            "1000000000000.00000001",
        ] {
            let decoded = amounts(&format!(r#"{{"amount":"{text}"}}"#));
            assert_eq!(decoded.amount.to_string(), text, "{text}");
        }
        let float: f64 = "98765432109876.54321".parse().unwrap();
        assert_ne!(float.to_string(), "98765432109876.54321");
    }

    #[test]
    fn keeps_the_trailing_zeros_of_kraken() {
        assert_eq!(
            amounts(r#"{"amount":"1.25000000"}"#).amount.to_string(),
            "1.25000000"
        );
        assert_eq!(amounts(r#"{"amount":" -0.5 "}"#).amount.to_string(), "-0.5");
    }

    #[test]
    fn reads_the_decimals_sent_as_numbers() {
        assert_eq!(amounts(r#"{"amount":42}"#).amount.to_string(), "42");
        assert_eq!(amounts(r#"{"amount":-42}"#).amount.to_string(), "-42");
        assert_eq!(amounts(r#"{"amount":0.26}"#).amount.to_string(), "0.26");
        assert_eq!(
            amounts(r#"{"amount":"1.5e-7"}"#).amount.to_string(),
            "0.00000015"
        );
    }

    #[test]
    fn reads_the_missing_decimals_as_none() {
        for json in [
            r#"{"amount":"1"}"#,
            r#"{"amount":"1","optional":null}"#,
            r#"{"amount":"1","optional":""}"#,
            r#"{"amount":"1","optional":"  "}"#,
        ] {
            assert_eq!(amounts(json).optional, None, "{json}");
        }
        assert_eq!(
            amounts(r#"{"amount":"1","optional":"0.1"}"#)
                .optional
                .map(|it| it.to_string()),
            Some("0.1".into())
        );
    }

    #[test]
    fn rejects_what_is_not_a_decimal() {
        for json in [
            r#"{}"#,
            r#"{"amount":null}"#,
            r#"{"amount":""}"#,
            r#"{"amount":"abc"}"#,
            r#"{"amount":true}"#,
            r#"{"amount":[1]}"#,
            r#"{"amount":"1","optional":"x"}"#,
            // beyond the 28 digits of a Decimal: refused rather than rounded
            r#"{"amount":"123456789012345678901234567890"}"#,
        ] {
            assert!(serde_json::from_str::<Amounts>(json).is_err(), "{json}");
        }
    }
}
