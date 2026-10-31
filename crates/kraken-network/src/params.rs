//! Encoding of the parameters, for the models of the requests: Kraken expects form / query
//! strings, its lists being comma separated values (`pair=XBTUSD,ETHUSD`).

use crate::KrakenError;
use serde::{Serialize, Serializer};

/// Form-urlencodes the parameters, `None` fields being skipped by the models
pub fn encode<P: Serialize + ?Sized>(params: &P) -> Result<String, KrakenError> {
    serde_urlencoded::to_string(params).map_err(|error| KrakenError::InvalidArgument {
        reason: error.to_string(),
    })
}

/// Serializes a list as comma separated values, for `serialize_with`
pub fn comma_separated<S, T>(values: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<str>,
{
    let joined = values
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join(",");
    serializer.serialize_str(&joined)
}

/// [`comma_separated`] for optional lists, to use with `skip_serializing_if = "Option::is_none"`
pub fn opt_comma_separated<S, T>(values: &Option<Vec<T>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<str>,
{
    match values {
        Some(values) => comma_separated(values, serializer),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Sample {
        #[serde(serialize_with = "comma_separated")]
        list: Vec<String>,
        #[serde(
            skip_serializing_if = "Option::is_none",
            serialize_with = "opt_comma_separated"
        )]
        optional: Option<Vec<String>>,
    }

    #[test]
    fn joins_the_lists() {
        let sample = Sample {
            list: vec!["XBTUSD".into(), "ETHUSD".into()],
            optional: None,
        };
        assert_eq!(encode(&sample).unwrap(), "list=XBTUSD%2CETHUSD");

        let sample = Sample {
            list: vec![],
            optional: Some(vec!["a".into()]),
        };
        assert_eq!(encode(&sample).unwrap(), "list=&optional=a");
    }

    #[test]
    fn escapes_the_special_characters() {
        let sample = Sample {
            list: vec!["a b".into(), "c&d=e".into(), "é/+".into()],
            optional: None,
        };
        assert_eq!(
            encode(&sample).unwrap(),
            "list=a+b%2Cc%26d%3De%2C%C3%A9%2F%2B"
        );
    }

    #[test]
    fn refuses_the_nested_values() {
        #[derive(Serialize)]
        struct Nested {
            inner: Vec<u8>,
        }
        assert!(matches!(
            encode(&Nested { inner: vec![1] }),
            Err(KrakenError::InvalidArgument { .. })
        ));
    }
}
