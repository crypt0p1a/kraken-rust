use super::{OrderSide, OrderStatus, OrderType};
use crate::lenient;
use serde::Deserialize;

/// <https://docs.kraken.com/api/docs/rest-api/get-open-orders>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct OrderDescription {
    /// Alternate name of the pair, e.g. "XBTUSD"
    pub pair: String,
    #[serde(rename = "type")]
    pub side: OrderSide,
    #[serde(rename = "ordertype")]
    pub order_type: OrderType,
    /// Limit price, or trigger price of the stop / take profit orders
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub price: Option<f64>,
    /// Limit price of the stop-loss-limit / take-profit-limit orders
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub price2: Option<f64>,
    #[serde(default, deserialize_with = "lenient::opt_string")]
    pub leverage: Option<String>,
    /// Human readable description, e.g. "buy 1.25000000 XBTUSD @ limit 27500.0"
    pub order: String,
    /// Conditional close order, if any
    #[serde(default, deserialize_with = "lenient::opt_string")]
    pub close: Option<String>,
}

/// An order, as returned by OpenOrders, ClosedOrders and QueryOrders
///
/// <https://docs.kraken.com/api/docs/rest-api/get-open-orders>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Order {
    /// Kraken identifier of the order: the key of the answers, copied here
    #[serde(skip)]
    pub txid: String,
    /// Referral order transaction id that created this order
    #[serde(default, deserialize_with = "lenient::opt_string")]
    pub refid: Option<String>,
    /// Optional numeric, client identifier
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub userref: Option<i64>,
    /// Optional client identifier, alternative to `userref`
    #[serde(default, deserialize_with = "lenient::opt_string")]
    pub cl_ord_id: Option<String>,
    pub status: OrderStatus,
    /// UTC timestamps, in seconds (with decimals)
    #[serde(rename = "opentm", deserialize_with = "lenient::number")]
    pub open_time: f64,
    #[serde(rename = "starttm", default, deserialize_with = "lenient::opt_number")]
    pub start_time: Option<f64>,
    #[serde(rename = "expiretm", default, deserialize_with = "lenient::opt_number")]
    pub expire_time: Option<f64>,
    #[serde(rename = "closetm", default, deserialize_with = "lenient::opt_number")]
    pub close_time: Option<f64>,
    #[serde(rename = "descr")]
    pub description: OrderDescription,
    /// Volume, in the base currency
    #[serde(rename = "vol", deserialize_with = "lenient::number")]
    pub volume: f64,
    #[serde(rename = "vol_exec", deserialize_with = "lenient::number")]
    pub volume_executed: f64,
    /// Total cost, in the quote currency
    #[serde(deserialize_with = "lenient::number")]
    pub cost: f64,
    /// Total fee, in the quote currency
    #[serde(deserialize_with = "lenient::number")]
    pub fee: f64,
    /// Average price, in the quote currency
    #[serde(rename = "price", deserialize_with = "lenient::number")]
    pub average_price: f64,
    #[serde(
        rename = "stopprice",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub stop_price: Option<f64>,
    #[serde(
        rename = "limitprice",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub limit_price: Option<f64>,
    /// Comma separated: stopped, touched, liquidated, partial
    #[serde(default)]
    pub misc: String,
    /// Comma separated order flags, e.g. "fciq,post"
    #[serde(default)]
    pub oflags: String,
    /// Identifiers of the trades, when requested with `trades`
    #[serde(default)]
    pub trades: Option<Vec<String>>,
    /// Why the order was closed / cancelled
    #[serde(default, deserialize_with = "lenient::opt_string")]
    pub reason: Option<String>,
}

/// Orders keyed by txid → orders holding their txid, most recent first
pub(crate) fn with_txids(orders: std::collections::HashMap<String, Order>) -> Vec<Order> {
    let mut orders: Vec<Order> = orders
        .into_iter()
        .map(|(txid, order)| Order { txid, ..order })
        .collect();
    orders.sort_by(|a, b| b.open_time.total_cmp(&a.open_time));
    orders
}

#[cfg(test)]
pub(crate) const SAMPLE: &str = r#"{"OQCLML-BW3P3-BUCMWZ":{"refid":null,"userref":0,
    "cl_ord_id":"mgr1_buy_1","status":"open","opentm":1688666559.8974,"starttm":0,"expiretm":0,
    "descr":{"pair":"XBTUSD","type":"buy","ordertype":"limit","price":"30010.0","price2":"0",
        "leverage":"none","order":"buy 1.25000000 XBTUSD @ limit 30010.0","close":""},
    "vol":"1.25000000","vol_exec":"0.37500000","cost":"11253.7","fee":"0.00000","price":"30010.0",
    "stopprice":"0.00000","limitprice":"0.00000","misc":"","oflags":"fciq",
    "trades":["TCCCTY-WE2O6-P3NB37"]},
    "OB5VMB-B4U2U-DK2WRW":{"refid":null,"userref":45,"status":"closed","reason":null,
    "opentm":1688665496.7808,"closetm":1688665499.1922,"starttm":0,"expiretm":0,
    "descr":{"pair":"XBTUSD","type":"sell","ordertype":"market","price":"0","price2":"0",
        "leverage":"none","order":"sell 0.25000000 XBTUSD @ market","close":""},
    "vol":"0.25000000","vol_exec":"0.25000000","cost":"7500.2","fee":"19.5","price":"30000.8",
    "stopprice":"0.00000","limitprice":"0.00000","misc":"","oflags":"fcib"}}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn decodes_the_orders_with_their_txid() {
        let orders = with_txids(serde_json::from_str::<HashMap<String, Order>>(SAMPLE).unwrap());

        assert_eq!(orders.len(), 2);
        let open = &orders[0];
        assert_eq!(open.txid, "OQCLML-BW3P3-BUCMWZ");
        assert_eq!(open.cl_ord_id.as_deref(), Some("mgr1_buy_1"));
        assert_eq!(open.status, OrderStatus::Open);
        assert_eq!(open.description.side, OrderSide::Buy);
        assert_eq!(open.description.price, Some(30010.0));
        assert_eq!(open.volume_executed, 0.375);
        assert_eq!(open.trades.as_ref().map(Vec::len), Some(1));

        let closed = &orders[1];
        assert_eq!(closed.txid, "OB5VMB-B4U2U-DK2WRW");
        assert!(closed.status.is_final());
        assert_eq!(closed.description.order_type, OrderType::Market);
        assert_eq!(closed.close_time, Some(1688665499.1922));
    }
}
