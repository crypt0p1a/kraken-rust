//! Private algorithmic trading: the TWAP strategies, which execute a volume evenly over a period
//! as market or limit child orders

use crate::trading::{CloseTime, OrderSide, OrderType};
use crate::{KrakenError, lenient};
use kraken_network::PrivateClient;
use kraken_network::params::comma_separated;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The only type of strategy today
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum StrategyType {
    Twap,
    /// Sent by Kraken but unknown to this version of the library. Never send it
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum StrategyStatus {
    /// Accepted, waiting for its start
    Pending,
    /// Running
    Open,
    /// Fully executed
    Closed,
    Expired,
    Cancelled,
    #[serde(other)]
    Unknown,
}

/// The TWAP and its execution
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct StrategyDetails {
    pub strategy_type: StrategyType,
    pub side: OrderSide,
    /// Total volume, in the base currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub volume: Decimal,
    /// Volume executed so far
    #[serde(deserialize_with = "lenient::decimal")]
    pub filled_volume: Decimal,
    /// None until the first fill
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub avg_price: Option<Decimal>,
    /// In the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub total_cost: Decimal,
    /// In the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub total_fee: Decimal,
    /// UTC timestamps in seconds
    #[serde(rename = "starttm", deserialize_with = "lenient::number")]
    pub start_time: i64,
    #[serde(rename = "expiretm", deserialize_with = "lenient::number")]
    pub expire_time: i64,
    /// e.g. "BTC/USD"
    pub symbol: String,
    /// "forex", "tokenized_asset" or "synthetic_pair"
    pub asset_class: String,
    /// Type of the child orders: market or limit
    #[serde(rename = "ordertype")]
    pub order_type: OrderType,
    /// Limit price of the child orders
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub price: Option<Decimal>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-open-strategies>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Strategy {
    /// Starts with "AS"
    pub strategy_id: String,
    pub status: StrategyStatus,
    /// More about the status, e.g. "insufficientFunds", "userRequested"
    #[serde(default)]
    pub reason: Option<String>,
    pub details: StrategyDetails,
}

/// Strategies keyed by id → their list, the most recent start first
fn listed(strategies: HashMap<String, Strategy>) -> Vec<Strategy> {
    let mut strategies: Vec<Strategy> = strategies.into_values().collect();
    strategies.sort_by_key(|it| std::cmp::Reverse(it.details.start_time));
    strategies
}

#[derive(Serialize)]
struct OfType {
    #[serde(skip_serializing_if = "Option::is_none")]
    strategy_type: Option<StrategyType>,
}

#[derive(Deserialize)]
struct OpenAnswer {
    open: HashMap<String, Strategy>,
}

/// The pending and running strategies, the most recent start first
pub(crate) async fn open_strategies(
    client: &PrivateClient,
    strategy_type: Option<StrategyType>,
) -> Result<Vec<Strategy>, KrakenError> {
    let answer: OpenAnswer = client
        .call_with("GetOpenStrategies", Some(&OfType { strategy_type }))
        .await?;
    Ok(listed(answer.open))
}

#[derive(Serialize)]
struct StrategyIds<'a> {
    #[serde(serialize_with = "comma_separated")]
    strategy_ids: &'a [String],
}

#[derive(Deserialize)]
struct QueryAnswer {
    #[serde(default)]
    strategies: HashMap<String, Strategy>,
}

/// Up to 20 strategies by id, the most recent start first: the unknown ids are omitted
pub(crate) async fn query_strategies(
    client: &PrivateClient,
    strategy_ids: &[String],
) -> Result<Vec<Strategy>, KrakenError> {
    let answer: QueryAnswer = client
        .call_with("QueryStrategies", Some(&StrategyIds { strategy_ids }))
        .await?;
    Ok(listed(answer.strategies))
}

/// <https://docs.kraken.com/api/docs/rest-api/get-closed-strategies>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ClosedStrategiesRequest {
    /// e.g. "forex": every class when missing. Repeat it on the next pages
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub strategy_type: Option<StrategyType>,
    /// Closed, expired or cancelled: the three when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub status: Option<StrategyStatus>,
    /// UTC timestamp in seconds (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start: Option<i64>,
    /// UTC timestamp in seconds (inclusive, the whole second)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub end: Option<i64>,
    /// Which times `start` and `end` apply to
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub query_time_mode: Option<CloseTime>,
    /// Strategies per page
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub limit: Option<u32>,
    /// The `next_cursor` of the previous page
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cursor: Option<String>,
}

/// A page of the closed, expired and cancelled strategies
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ClosedStrategies {
    /// Most recent first, by closing time (by opening time with the `open` time mode)
    #[serde(rename = "closed", default)]
    pub strategies: Vec<Strategy>,
    /// To give as `cursor` for the next page: None on the last one
    #[serde(default)]
    pub next_cursor: Option<String>,
}

pub(crate) async fn closed_strategies(
    client: &PrivateClient,
    request: &ClosedStrategiesRequest,
) -> Result<ClosedStrategies, KrakenError> {
    client.call_with("GetClosedStrategies", Some(request)).await
}

/// A new TWAP: `volume` executed evenly between its start and its expiry
///
/// The amounts and prices are strings, to keep the precision given by the caller
///
/// <https://docs.kraken.com/api/docs/rest-api/add-strategy>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NewStrategy {
    pub strategy_type: StrategyType,
    /// e.g. "BTC/USD" or "XBTUSD"
    pub symbol: String,
    /// "forex": the only class supported, the standard spot pairs
    pub asset_class: String,
    pub side: OrderSide,
    /// Total volume, in the base currency
    pub volume: String,
    /// Type of the child orders: market or limit
    #[serde(rename = "ordertype")]
    pub order_type: OrderType,
    /// Limit price of the child orders: required by limit, refused by market
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub price: Option<String>,
    /// "0" (now, the default), "<UTC timestamp in seconds>" or "+<seconds from now>": within 30
    /// days
    #[serde(rename = "starttm", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start_time: Option<String>,
    /// "<UTC timestamp in seconds>" or "+<seconds from now>": at least 60 seconds after the
    /// start, within 30 days
    #[serde(rename = "expiretm")]
    pub expire_time: String,
    /// Validates the strategy without creating it
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub validate: Option<bool>,
}

#[derive(Deserialize)]
struct StrategyId {
    strategy_id: String,
}

/// The id of the new strategy, empty when only validated
pub(crate) async fn add_strategy(
    client: &PrivateClient,
    request: &NewStrategy,
) -> Result<String, KrakenError> {
    let answer: StrategyId = client.call_with("AddStrategy", Some(request)).await?;
    Ok(answer.strategy_id)
}

#[derive(Serialize)]
struct Cancel<'a> {
    strategy_id: &'a str,
}

/// Stops the scheduling of the child orders of a pending or open strategy: a child order already
/// sent may still fill. Returns the id of the cancelled strategy
pub(crate) async fn cancel_strategy(
    client: &PrivateClient,
    strategy_id: &str,
) -> Result<String, KrakenError> {
    let answer: StrategyId = client
        .call_with("CancelStrategy", Some(&Cancel { strategy_id }))
        .await?;
    Ok(answer.strategy_id)
}

#[cfg(test)]
pub(crate) const SAMPLE: &str = r#"{
    "ASPBVCI-6MKBA-QE4VF6":{"strategy_id":"ASPBVCI-6MKBA-QE4VF6","status":"pending",
        "details":{"strategy_type":"twap","side":"buy","volume":"0.01000000","filled_volume":"0",
        "avg_price":"","total_cost":"0","total_fee":"0","starttm":"1781169958",
        "expiretm":"1781173558","symbol":"BTC/USD","asset_class":"forex","ordertype":"market"}},
    "ASMBA64-R23M2-J7JRJV":{"strategy_id":"ASMBA64-R23M2-J7JRJV","status":"open",
        "details":{"strategy_type":"twap","side":"sell","volume":"2.00000000",
        "filled_volume":"0.50000000","avg_price":"67050.10","total_cost":"33525.05",
        "total_fee":"53.64","starttm":"1781166400","expiretm":"1781173600","symbol":"BTC/USD",
        "asset_class":"forex","ordertype":"limit","price":"67000.0"}}
}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_type() {
        assert_eq!(
            encode(&OfType {
                strategy_type: Some(StrategyType::Twap)
            })
            .unwrap(),
            "strategy_type=twap"
        );
        assert_eq!(
            encode(&OfType {
                strategy_type: None
            })
            .unwrap(),
            ""
        );
    }

    #[test]
    fn encodes_the_ids() {
        let ids = [
            "ASPBVCI-6MKBA-QE4VF6".to_string(),
            "ASMBA64-R23M2-J7JRJV".to_string(),
        ];
        assert_eq!(
            encode(&StrategyIds { strategy_ids: &ids }).unwrap(),
            "strategy_ids=ASPBVCI-6MKBA-QE4VF6%2CASMBA64-R23M2-J7JRJV"
        );
    }

    #[test]
    fn decodes_the_strategies_most_recent_first() {
        let answer: OpenAnswer = serde_json::from_str(&format!(r#"{{"open":{SAMPLE}}}"#)).unwrap();
        let strategies = listed(answer.open);
        let pending = &strategies[0];
        assert_eq!(pending.status, StrategyStatus::Pending);
        // no fill yet: an empty average price
        assert_eq!(pending.details.avg_price, None);
        assert_eq!(pending.details.order_type, OrderType::Market);
        let open = &strategies[1];
        assert_eq!(open.details.avg_price, Some(dec!(67050.10)));
        assert_eq!(open.details.price, Some(dec!(67000)));
        assert_eq!(open.details.start_time, 1781166400);
    }

    #[test]
    fn encodes_every_filter_of_the_closed_strategies() {
        let request = ClosedStrategiesRequest {
            asset_class: Some("forex".into()),
            strategy_type: Some(StrategyType::Twap),
            status: Some(StrategyStatus::Cancelled),
            start: Some(1781160000),
            end: Some(1781180000),
            query_time_mode: Some(CloseTime::Open),
            limit: Some(2),
            cursor: Some("Q0xPU0VEOjE3ODExNzAwMDA6QVNNQkE2NA".into()),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "asset_class=forex&strategy_type=twap&status=cancelled&start=1781160000\
             &end=1781180000&query_time_mode=open&limit=2&cursor=Q0xPU0VEOjE3ODExNzAwMDA6QVNNQkE2NA"
        );
        assert_eq!(encode(&ClosedStrategiesRequest::default()).unwrap(), "");
    }

    #[test]
    fn encodes_a_new_strategy() {
        let request = NewStrategy {
            strategy_type: StrategyType::Twap,
            symbol: "BTC/USD".into(),
            asset_class: "forex".into(),
            side: OrderSide::Sell,
            volume: "2.0".into(),
            order_type: OrderType::Limit,
            price: Some("67000.0".into()),
            start_time: Some("+60".into()),
            expire_time: "+3660".into(),
            validate: Some(true),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "strategy_type=twap&symbol=BTC%2FUSD&asset_class=forex&side=sell&volume=2.0\
             &ordertype=limit&price=67000.0&starttm=%2B60&expiretm=%2B3660&validate=true"
        );
    }

    #[test]
    fn decodes_the_id_of_a_validated_strategy() {
        let answer: StrategyId = serde_json::from_str(r#"{"strategy_id":""}"#).unwrap();
        assert!(answer.strategy_id.is_empty());
    }

    #[test]
    fn encodes_the_cancellation() {
        assert_eq!(
            encode(&Cancel {
                strategy_id: "ASMBA64-R23M2-J7JRJV"
            })
            .unwrap(),
            "strategy_id=ASMBA64-R23M2-J7JRJV"
        );
    }

    #[test]
    fn decodes_every_status() {
        for (text, status) in [
            ("pending", StrategyStatus::Pending),
            ("open", StrategyStatus::Open),
            ("closed", StrategyStatus::Closed),
            ("expired", StrategyStatus::Expired),
            ("cancelled", StrategyStatus::Cancelled),
            ("paused", StrategyStatus::Unknown),
        ] {
            assert_eq!(
                serde_json::from_value::<StrategyStatus>(text.into()).unwrap(),
                status
            );
        }
    }
}
