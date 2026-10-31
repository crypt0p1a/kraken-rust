use crate::KrakenError;
use kraken_network::PublicClient;
use serde::Deserialize;

/// <https://docs.kraken.com/api/docs/rest-api/get-server-time>
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ServerTime {
    /// UTC timestamp, in seconds
    #[serde(rename = "unixtime")]
    pub unix_time: i64,
    /// RFC 1123 date, e.g. "Sun, 21 Mar 21 14:23:14 +0000"
    pub rfc1123: String,
}

pub(crate) async fn server_time(client: &PublicClient) -> Result<ServerTime, KrakenError> {
    client.call("Time").await
}
