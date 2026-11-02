use super::{
    NewOrderDescription, OrderFlag, OrderSide, OrderType, SelfTradePrevention, TimeInForce, Trigger,
};
use crate::KrakenError;
use crate::market::AssetClass;
use kraken_network::PrivateClient;
use kraken_network::params::opt_comma_separated;
use serde::{Deserialize, Serialize};

/// An order of a batch: the pair is the one of the batch
///
/// The amounts and prices are strings, to keep the precision given by the caller
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct BatchOrder {
    #[serde(rename = "type")]
    pub side: OrderSide,
    #[serde(rename = "ordertype")]
    pub order_type: OrderType,
    /// In the base currency
    pub volume: String,
    /// Volume shown in the book, for the iceberg orders
    #[serde(rename = "displayvol", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub display_volume: Option<String>,
    /// Limit price, or trigger price of the stop / take profit orders
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub price: Option<String>,
    /// Limit price of the stop-loss-limit / take-profit-limit orders
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub price2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub trigger: Option<Trigger>,
    /// e.g. "2:1": none when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub leverage: Option<String>,
    /// Only reduces an open margin position
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub reduce_only: Option<bool>,
    #[serde(rename = "stptype", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub self_trade_prevention: Option<SelfTradePrevention>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "opt_comma_separated"
    )]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub oflags: Option<Vec<OrderFlag>>,
    /// GTC when missing
    #[serde(rename = "timeinforce", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub time_in_force: Option<TimeInForce>,
    /// "0" (now), "<UTC timestamp in seconds>" or "+<seconds from now>"
    #[serde(rename = "starttm", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start_time: Option<String>,
    /// Required by GTD: "<UTC timestamp in seconds>" or "+<seconds from now>"
    #[serde(rename = "expiretm", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub expire_time: Option<String>,
    /// Client identifier: a UUID, or up to 18 ASCII characters. Exclusive with `userref`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cl_ord_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub userref: Option<i32>,
}

/// 2 to 15 orders of a pair, validated as a whole: one invalid order rejects the batch
///
/// <https://docs.kraken.com/api/docs/rest-api/add-order-batch>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AddOrderBatch {
    /// Alternate name of the pair, e.g. "XBTUSD"
    pub pair: String,
    pub orders: Vec<BatchOrder>,
    /// [`AssetClass::TokenizedAsset`] for the xStocks
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
    /// RFC 3339: the orders are rejected after it
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub deadline: Option<String>,
    /// Validates the orders without placing them
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub validate: Option<bool>,
    /// IIBAN of the broker (partner) of the orders
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub broker: Option<String>,
}

/// An order of the batch, in the order of the request
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct BatchOrderAnswer {
    #[serde(rename = "descr", default)]
    pub description: Option<NewOrderDescription>,
    /// Kraken identifier, when the order was placed
    #[serde(default)]
    pub txid: Option<String>,
    /// Why this order failed
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct BatchAnswer {
    orders: Vec<BatchOrderAnswer>,
}

/// Sent in JSON, the only way to send a list of orders
pub(crate) async fn add_order_batch(
    client: &PrivateClient,
    request: &AddOrderBatch,
) -> Result<Vec<BatchOrderAnswer>, KrakenError> {
    let answer: BatchAnswer = client.call_json("AddOrderBatch", request).await?;
    Ok(answer.orders)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn limit(side: OrderSide, volume: &str, price: &str) -> BatchOrder {
        BatchOrder {
            side,
            order_type: OrderType::Limit,
            volume: volume.into(),
            display_volume: None,
            price: Some(price.into()),
            price2: None,
            trigger: None,
            leverage: None,
            reduce_only: None,
            self_trade_prevention: None,
            oflags: None,
            time_in_force: None,
            start_time: None,
            expire_time: None,
            cl_ord_id: None,
            userref: None,
        }
    }

    #[test]
    fn encodes_the_batch_in_json() {
        let request = AddOrderBatch {
            pair: "XBTUSD".into(),
            orders: vec![
                BatchOrder {
                    oflags: Some(vec![OrderFlag::Post, OrderFlag::Fciq]),
                    time_in_force: Some(TimeInForce::Gtd),
                    expire_time: Some("+3600".into()),
                    cl_ord_id: Some("mgr1_buy_1".into()),
                    ..limit(OrderSide::Buy, "1.2", "28300.0")
                },
                BatchOrder {
                    order_type: OrderType::StopLossLimit,
                    price2: Some("27000.0".into()),
                    trigger: Some(Trigger::Index),
                    self_trade_prevention: Some(SelfTradePrevention::CancelBoth),
                    reduce_only: Some(false),
                    ..limit(OrderSide::Sell, "1.2", "27100.0")
                },
            ],
            asset_class: None,
            deadline: Some("2026-11-02T10:00:00Z".into()),
            validate: Some(true),
            broker: None,
        };
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            json!({"pair": "XBTUSD", "deadline": "2026-11-02T10:00:00Z", "validate": true,
            "orders": [
                {"type": "buy", "ordertype": "limit", "volume": "1.2", "price": "28300.0",
                    "oflags": "post,fciq", "timeinforce": "GTD", "expiretm": "+3600",
                    "cl_ord_id": "mgr1_buy_1"},
                {"type": "sell", "ordertype": "stop-loss-limit", "volume": "1.2",
                    "price": "27100.0", "price2": "27000.0", "trigger": "index",
                    "reduce_only": false, "stptype": "cancel-both"}
            ]})
        );
    }

    #[test]
    fn decodes_the_answers_of_each_order() {
        // example of the documentation, plus an order failing
        let answer: BatchAnswer = serde_json::from_str(
            r#"{"orders":[
            {"txid":"O5OR23-ADFAD-Y2G61C","descr":{"order":"buy 0.80300000 XBTUSD @ limit 28300.0",
                "close":"close position @ stop loss 27000.0 -> limit 26000.0"}},
            {"error":"EOrder:Insufficient funds"}]}"#,
        )
        .unwrap();
        assert_eq!(
            answer.orders[0].txid.as_deref(),
            Some("O5OR23-ADFAD-Y2G61C")
        );
        assert!(
            answer.orders[0]
                .description
                .as_ref()
                .unwrap()
                .close
                .is_some()
        );
        assert_eq!(
            answer.orders[1].error.as_deref(),
            Some("EOrder:Insufficient funds")
        );
        assert_eq!(answer.orders[1].txid, None);
    }
}
