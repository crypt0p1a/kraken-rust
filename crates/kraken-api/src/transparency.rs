//! Public transparency data (MiCA): the order book and the trades of the exchange, described
//! with their regulatory identifiers (DTI, MIC)
//!
//! The pairs are given by their symbol, e.g. "BTC/USD", and the times are ISO 8601 strings.

use crate::KrakenError;
use crate::lenient;
use kraken_network::PublicClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// An aggregated price level
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PreTradeLevel {
    /// "BUY" or "SELL"
    pub side: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub price: Decimal,
    #[serde(rename = "qty", deserialize_with = "lenient::decimal")]
    pub quantity: Decimal,
    /// Orders at this level
    #[serde(deserialize_with = "lenient::number")]
    pub count: i64,
    /// ISO 8601
    pub submission_ts: String,
    /// ISO 8601
    pub publication_ts: String,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-pre-trade-data>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PreTradeData {
    /// e.g. "BTC/USD"
    pub symbol: String,
    /// e.g. "Bitcoin / US Dollar"
    pub description: String,
    pub base_asset: String,
    /// "UNIT" or "NOML"
    pub base_notation: String,
    /// Digital Token Identifier, empty when the asset has none
    #[serde(default)]
    pub base_dti_code: Option<String>,
    #[serde(default)]
    pub base_dti_short_name: Option<String>,
    pub quote_asset: String,
    /// "MONE"
    pub quote_notation: String,
    #[serde(default)]
    pub quote_dti_code: Option<String>,
    #[serde(default)]
    pub quote_dti_short_name: Option<String>,
    /// Market Identifier Code, e.g. "PGSL"
    pub venue: String,
    /// "CLOB"
    pub system: String,
    /// The top 10 levels, highest first
    pub bids: Vec<PreTradeLevel>,
    /// The top 10 levels, lowest first
    pub asks: Vec<PreTradeLevel>,
}

#[derive(Serialize)]
struct Symbol<'a> {
    symbol: &'a str,
}

/// The top 10 levels of each side of the order book of `symbol`, e.g. "BTC/USD"
pub(crate) async fn pre_trade(
    client: &PublicClient,
    symbol: &str,
) -> Result<PreTradeData, KrakenError> {
    client.call_with("PreTrade", Some(&Symbol { symbol })).await
}

/// A trade of the exchange
///
/// Its pair is described by the same fields as [`PreTradeData`], repeated rather than shared
/// through `#[serde(flatten)]`: a flattened struct would hide the undocumented fields from the
/// strict decoding of the tests
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PostTrade {
    pub trade_id: String,
    /// Without the fees
    #[serde(deserialize_with = "lenient::decimal")]
    pub price: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub quantity: Decimal,
    pub symbol: String,
    pub description: String,
    pub base_asset: String,
    pub base_notation: String,
    #[serde(default)]
    pub base_dti_code: Option<String>,
    #[serde(default)]
    pub base_dti_short_name: Option<String>,
    pub quote_asset: String,
    pub quote_notation: String,
    #[serde(default)]
    pub quote_dti_code: Option<String>,
    #[serde(default)]
    pub quote_dti_short_name: Option<String>,
    /// Market Identifier Code where the trade was executed
    pub trade_venue: String,
    /// ISO 8601, to the microsecond
    pub trade_ts: String,
    pub publication_venue: String,
    /// ISO 8601
    pub publication_ts: String,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-post-trade-data>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PostTradeData {
    /// Time of the most recent trade: to give as `from_ts` to get the next ones. Missing without
    /// trades
    #[serde(default)]
    pub last_ts: Option<String>,
    #[serde(deserialize_with = "lenient::number")]
    pub count: i64,
    /// Most recent first, whatever the documentation of Kraken says
    #[serde(default)]
    pub trades: Vec<PostTrade>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-post-trade-data>: the last 1000 trades of
/// every pair when empty
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PostTradeRequest {
    /// e.g. "BTC/USD"
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub symbol: Option<String>,
    /// ISO 8601
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub from_ts: Option<String>,
    /// ISO 8601
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub to_ts: Option<String>,
    /// 1 to 1000: 1000 when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub count: Option<u32>,
}

pub(crate) async fn post_trade(
    client: &PublicClient,
    request: &PostTradeRequest,
) -> Result<PostTradeData, KrakenError> {
    client.call_with("PostTrade", Some(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_pre_trade_request() {
        assert_eq!(
            encode(&Symbol { symbol: "BTC/USD" }).unwrap(),
            "symbol=BTC%2FUSD"
        );
    }

    #[test]
    fn encodes_the_post_trade_request() {
        let request = PostTradeRequest {
            symbol: Some("BTC/USD".into()),
            from_ts: Some("2026-11-02T09:37:16Z".into()),
            to_ts: None,
            count: Some(2),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "symbol=BTC%2FUSD&from_ts=2026-11-02T09%3A37%3A16Z&count=2"
        );
        assert_eq!(encode(&PostTradeRequest::default()).unwrap(), "");
    }

    #[test]
    fn decodes_the_pre_trade_data() {
        let data: PreTradeData = serde_json::from_str(
            r#"{"symbol":"BTC/USD","description":"Bitcoin / US Dollar","base_asset":"XBT",
            "base_notation":"UNIT","base_dti_code":"V15WLZJMF","base_dti_short_name":"BTC,XBT",
            "quote_asset":"USD","quote_notation":"MONE","quote_dti_code":"",
            "quote_dti_short_name":"","venue":"PGSL","system":"CLOB",
            "bids":[{"side":"BUY","price":"77931.00000","qty":"0.02565977","count":2,
                "publication_ts":"2026-11-02T09:37:16.830882918Z",
                "submission_ts":"2026-11-02T09:37:16.830882918Z"}],"asks":[]}"#,
        )
        .unwrap();
        assert_eq!(data.venue, "PGSL");
        assert_eq!(data.base_dti_code.as_deref(), Some("V15WLZJMF"));
        assert_eq!(data.bids[0].quantity, dec!(0.02565977));
        assert_eq!(data.bids[0].count, 2);
    }

    #[test]
    fn decodes_the_post_trade_data() {
        let data: PostTradeData = serde_json::from_str(
            r#"{"last_ts":"2026-11-02T09:37:16.828994156Z","count":1,"trades":[
            {"trade_id":"OOM6MK-R3F4D-FTCPL7","price":"77931.00000","quantity":"0.02565931",
            "symbol":"BTC/USD","description":"Bitcoin / US Dollar","base_asset":"XBT",
            "base_notation":"UNIT","base_dti_code":"V15WLZJMF","base_dti_short_name":"BTC,XBT",
            "quote_asset":"USD","quote_notation":"MONE","quote_dti_code":"",
            "quote_dti_short_name":"","trade_venue":"PGSL",
            "trade_ts":"2026-11-02T09:37:16.828994156Z","publication_venue":"PGSL",
            "publication_ts":"2026-11-02T09:37:16.828994156Z"}]}"#,
        )
        .unwrap();
        assert_eq!(data.count, 1);
        assert_eq!(data.trades[0].price, dec!(77931));

        let empty: PostTradeData = serde_json::from_str(r#"{"count":0}"#).unwrap();
        assert!(empty.trades.is_empty() && empty.last_ts.is_none());
    }
}
