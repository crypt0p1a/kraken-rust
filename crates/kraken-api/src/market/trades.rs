use super::AssetClass;
use super::keyed::single_pair;
use crate::KrakenError;
use crate::lenient::{self, ExactDecimal};
use crate::trading::{OrderSide, OrderType};
use kraken_network::PublicClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A public trade of the market
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PublicTrade {
    pub price: Decimal,
    pub volume: Decimal,
    /// UTC timestamp in seconds, with decimals
    pub time: f64,
    /// Side of the taker
    pub side: OrderSide,
    /// [`OrderType::Market`] or [`OrderType::Limit`]
    pub order_type: OrderType,
    pub miscellaneous: String,
    pub trade_id: i64,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-recent-trades>
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct RecentTrades {
    /// Kraken name of the pair, e.g. "XXBTZUSD"
    pub pair: String,
    /// Oldest first
    pub trades: Vec<PublicTrade>,
    /// To give as `since` to get the next trades: inclusive, the last trade of this answer is
    /// sent again
    pub last: String,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-recent-trades>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct RecentTradesRequest {
    pub pair: String,
    /// The `last` of a previous answer, or a UTC timestamp in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub since: Option<String>,
    /// 1 to 1000: 1000 when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
}

/// [price, volume, time, "b"/"s", "m"/"l", miscellaneous, trade id]
#[derive(Deserialize)]
struct RawTrade(
    ExactDecimal,
    ExactDecimal,
    #[serde(deserialize_with = "lenient::number")] f64,
    String,
    String,
    String,
    #[serde(deserialize_with = "lenient::number")] i64,
);

impl TryFrom<RawTrade> for PublicTrade {
    type Error = KrakenError;

    fn try_from(raw: RawTrade) -> Result<Self, Self::Error> {
        let RawTrade(price, volume, time, side, order_type, miscellaneous, trade_id) = raw;
        let side = match side.as_str() {
            "b" => OrderSide::Buy,
            "s" => OrderSide::Sell,
            other => {
                return Err(KrakenError::Decode {
                    reason: format!("unknown side of a trade: {other:?}"),
                });
            }
        };
        let order_type = match order_type.as_str() {
            "m" => OrderType::Market,
            "l" => OrderType::Limit,
            _ => OrderType::Unknown,
        };
        Ok(PublicTrade {
            price: price.0,
            volume: volume.0,
            time,
            side,
            order_type,
            miscellaneous,
            trade_id,
        })
    }
}

fn decode_trades(result: Map<String, Value>) -> Result<RecentTrades, KrakenError> {
    let (pair, trades, last) = single_pair(result)?;
    let trades: Vec<RawTrade> = serde_json::from_value(trades).map_err(KrakenError::decode)?;
    let last = match last {
        Some(Value::String(last)) => last,
        Some(Value::Number(last)) => last.to_string(),
        _ => String::new(),
    };
    Ok(RecentTrades {
        pair,
        trades: trades
            .into_iter()
            .map(PublicTrade::try_from)
            .collect::<Result<_, _>>()?,
        last,
    })
}

pub(crate) async fn recent_trades(
    client: &PublicClient,
    request: &RecentTradesRequest,
) -> Result<RecentTrades, KrakenError> {
    decode_trades(client.call_with("Trades", Some(request)).await?)
}

/// Best bid and ask at a time
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Spread {
    /// UTC timestamp, in seconds
    pub time: i64,
    pub bid: Decimal,
    pub ask: Decimal,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-recent-spreads>
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct RecentSpreads {
    pub pair: String,
    /// Oldest first
    pub spreads: Vec<Spread>,
    /// To give as `since` to get the next spreads
    pub last: i64,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-recent-spreads>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct RecentSpreadsRequest {
    pub pair: String,
    /// The `last` of a previous answer
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub since: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
}

/// [time, bid, ask]
#[derive(Deserialize)]
struct RawSpread(
    #[serde(deserialize_with = "lenient::number")] i64,
    ExactDecimal,
    ExactDecimal,
);

fn decode_spreads(result: Map<String, Value>) -> Result<RecentSpreads, KrakenError> {
    let (pair, spreads, last) = single_pair(result)?;
    let spreads: Vec<RawSpread> = serde_json::from_value(spreads).map_err(KrakenError::decode)?;
    let last = last
        .map(|last| lenient::number::<_, i64>(last).map_err(KrakenError::decode))
        .transpose()?
        .unwrap_or_default();
    Ok(RecentSpreads {
        pair,
        spreads: spreads
            .into_iter()
            .map(|RawSpread(time, bid, ask)| Spread {
                time,
                bid: bid.0,
                ask: ask.0,
            })
            .collect(),
        last,
    })
}

pub(crate) async fn recent_spreads(
    client: &PublicClient,
    request: &RecentSpreadsRequest,
) -> Result<RecentSpreads, KrakenError> {
    decode_spreads(client.call_with("Spread", Some(request)).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_trades_request() {
        let request = RecentTradesRequest {
            pair: "XBTUSD".into(),
            since: Some("1793612207465553566".into()),
            count: Some(3),
            asset_class: None,
        };
        assert_eq!(
            encode(&request).unwrap(),
            "pair=XBTUSD&since=1793612207465553566&count=3"
        );
    }

    #[test]
    fn encodes_the_spreads_request() {
        let request = RecentSpreadsRequest {
            pair: "XBTUSD".into(),
            since: Some(1793611608),
            asset_class: None,
        };
        assert_eq!(encode(&request).unwrap(), "pair=XBTUSD&since=1793611608");
    }

    #[test]
    fn decodes_the_trades() {
        let trades = decode_trades(
            serde_json::from_str(
                r#"{"XXBTZUSD":[["77934.10000","0.00049086",1793612206.5751364,"b","l","",113876654],
                ["77930.0","1.5",1793612207.0233848,"s","m","",113876655]],
                "last":"1793612207465553566"}"#,
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(trades.pair, "XXBTZUSD");
        assert_eq!(trades.last, "1793612207465553566");
        let first = &trades.trades[0];
        assert_eq!(first.price, dec!(77934.1));
        assert_eq!(first.side, OrderSide::Buy);
        assert_eq!(first.order_type, OrderType::Limit);
        assert_eq!(first.trade_id, 113876654);
        assert_eq!(trades.trades[1].side, OrderSide::Sell);
        assert_eq!(trades.trades[1].order_type, OrderType::Market);
    }

    #[test]
    fn rejects_an_unknown_side() {
        let trades = decode_trades(
            serde_json::from_str(r#"{"XXBTZUSD":[["1","1",1.5,"x","l","",1]],"last":"1"}"#)
                .unwrap(),
        );
        assert!(matches!(trades, Err(KrakenError::Decode { .. })));
    }

    #[test]
    fn decodes_the_spreads() {
        let spreads = decode_spreads(
            serde_json::from_str(
                r#"{"XXBTZUSD":[[1793611608,"78058.00000","78058.10000"]],"last":1793612207}"#,
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(spreads.pair, "XXBTZUSD");
        assert_eq!(spreads.last, 1793612207);
        assert_eq!(spreads.spreads[0].bid, dec!(78058));
        assert_eq!(spreads.spreads[0].ask, dec!(78058.1));
    }
}
