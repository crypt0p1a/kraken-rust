use crate::{KrakenError, lenient};
use kraken_network::PrivateClient;
use serde::Deserialize;

/// The token authenticating the private feeds of the WebSocket API
///
/// <https://docs.kraken.com/api/docs/rest-api/get-websockets-token>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WebSocketsToken {
    pub token: String,
    /// Seconds before it expires: to be used within 15 minutes, it then lasts as long as the
    /// connection
    #[serde(deserialize_with = "lenient::number")]
    pub expires: i64,
}

/// Requires the "WebSocket interface - On" permission
pub(crate) async fn websockets_token(
    client: &PrivateClient,
) -> Result<WebSocketsToken, KrakenError> {
    client.call("GetWebSocketsToken").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_token() {
        let token: WebSocketsToken = serde_json::from_str(
            r#"{"token":"1Dwc4lzSwNWOAwkMdqhssNNFhs1ed606d1WcF3XfEMw","expires":900}"#,
        )
        .unwrap();
        assert_eq!(token.expires, 900);
        assert!(serde_json::from_str::<WebSocketsToken>(r#"{"expires":900}"#).is_err());
    }
}
