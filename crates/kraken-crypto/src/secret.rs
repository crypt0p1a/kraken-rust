use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::fmt;

/// Why an API secret can't be used
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretError {
    #[error("the API secret is empty")]
    Empty,
    #[error("the API secret isn't base64: {0}")]
    NotBase64(String),
}

/// Decoded API secret, the key of the signatures. Never printed
#[derive(Clone)]
pub struct ApiSecret {
    pub(crate) bytes: Vec<u8>,
}

impl ApiSecret {
    /// The secret as given by Kraken, in base64
    pub fn from_base64(secret: &str) -> Result<Self, SecretError> {
        let secret = secret.trim();

        if secret.is_empty() {
            return Err(SecretError::Empty);
        }

        BASE64
            .decode(secret)
            .map(|bytes| Self { bytes })
            .map_err(|error| SecretError::NotBase64(error.to_string()))
    }
}

impl fmt::Debug for ApiSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ApiSecret(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_the_invalid_secrets() {
        assert_eq!(
            ApiSecret::from_base64("  ").unwrap_err(),
            SecretError::Empty
        );
        assert!(matches!(
            ApiSecret::from_base64("not base64!"),
            Err(SecretError::NotBase64(_))
        ));
    }

    #[test]
    fn never_prints_the_secret() {
        let secret = ApiSecret::from_base64("c2VjcmV0").unwrap();
        assert_eq!(format!("{secret:?}"), "ApiSecret(***)");
    }

    #[test]
    fn ignores_the_surrounding_whitespace() {
        let padded = ApiSecret::from_base64("  c2VjcmV0\n").unwrap();
        assert_eq!(padded.bytes, b"secret");
    }

    #[test]
    fn decodes_binary_secrets() {
        let secret = ApiSecret::from_base64("AP8QIA==").unwrap();
        assert_eq!(secret.bytes, [0x00, 0xff, 0x10, 0x20]);
    }

    #[test]
    fn requires_the_padding() {
        assert!(ApiSecret::from_base64("c2VjcmV0").is_ok());
        assert!(matches!(
            ApiSecret::from_base64("c2VjcmV0LQ"),
            Err(SecretError::NotBase64(_))
        ));
    }
}
