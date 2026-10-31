use kraken_crypto::SecretError;

/// Errors of the calls to Kraken
///
/// Same variants as the `CexError` of `cex-spot-trading`, plus [`KrakenError::InvalidArgument`],
/// so that both providers can share a single error type once merged.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum KrakenError {
    /// The server answered with an HTTP status other than 200
    #[error("HTTP {status}: {body}")]
    Network { status: u16, body: String },

    /// The server answered with a non-empty `error` list, or without the expected `result`
    #[error("Kraken error (HTTP {status}): {body}")]
    Api { status: u16, body: String },

    /// The request couldn't be sent or its answer couldn't be read
    #[error("Transport error: {reason}")]
    Transport { reason: String },

    /// The answer doesn't match the expected model
    #[error("Unexpected answer: {reason}")]
    Decode { reason: String },

    /// The request can't be built from the given parameters (e.g. API secret not in base64)
    #[error("Invalid argument: {reason}")]
    InvalidArgument { reason: String },
}

impl KrakenError {
    /// [`KrakenError::Decode`] from any decoding error
    pub fn decode(error: impl std::fmt::Display) -> Self {
        KrakenError::Decode {
            reason: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for KrakenError {
    fn from(error: reqwest::Error) -> Self {
        KrakenError::Transport {
            reason: error.to_string(),
        }
    }
}

impl From<SecretError> for KrakenError {
    fn from(error: SecretError) -> Self {
        KrakenError::InvalidArgument {
            reason: error.to_string(),
        }
    }
}
