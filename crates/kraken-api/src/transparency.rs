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
}
