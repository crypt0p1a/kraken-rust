use super::{OrderSide, OrderStatus, OrderType, TimeInForce};
use crate::lenient;
use rust_decimal::Decimal;
use serde::Deserialize;

/// <https://docs.kraken.com/api/docs/rest-api/get-open-orders>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct OrderDescription {
    /// Alternate name of the pair, e.g. "XBTUSD"
    pub pair: String,
    /// Asset class of the pair, e.g. "currency"
    #[serde(default)]
    pub aclass: Option<String>,
    #[serde(rename = "type")]
    pub side: OrderSide,
    #[serde(rename = "ordertype")]
    pub order_type: OrderType,
    /// Limit price, or trigger price of the stop / take profit orders
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub price: Option<Decimal>,
    /// Limit price of the stop-loss-limit / take-profit-limit orders
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub price2: Option<Decimal>,
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
    /// UTC timestamps, in seconds (with decimals): times, not amounts, so f64 (exact to the
    /// microsecond for the next centuries)
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
    #[serde(rename = "vol", deserialize_with = "lenient::decimal")]
    pub volume: Decimal,
    #[serde(rename = "vol_exec", deserialize_with = "lenient::decimal")]
    pub volume_executed: Decimal,
    /// Total cost, in the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub cost: Decimal,
    /// Total fee, in the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub fee: Decimal,
    /// Average price, in the quote currency
    #[serde(rename = "price", deserialize_with = "lenient::decimal")]
    pub average_price: Decimal,
    #[serde(
        rename = "stopprice",
        default,
        deserialize_with = "lenient::opt_decimal"
    )]
    pub stop_price: Option<Decimal>,
    #[serde(
        rename = "limitprice",
        default,
        deserialize_with = "lenient::opt_decimal"
    )]
    pub limit_price: Option<Decimal>,
    /// Price the stop / take profit orders are triggered by: "last" or "index"
    #[serde(default)]
    pub trigger: Option<String>,
    /// The order is funded on margin
    #[serde(default)]
    pub margin: Option<bool>,
    /// The order was amended
    #[serde(default)]
    pub amended: Option<bool>,
    /// Sub-account of the trader who placed the order (institutional accounts)
    #[serde(default)]
    pub sender_sub_id: Option<String>,
    #[serde(default)]
    pub time_in_force: Option<TimeInForce>,
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
    use rust_decimal_macros::dec;
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
        assert_eq!(open.description.price, Some(dec!(30010.0)));
        assert_eq!(open.volume_executed, dec!(0.375));
        assert_eq!(open.trades.as_ref().map(Vec::len), Some(1));

        let closed = &orders[1];
        assert_eq!(closed.txid, "OB5VMB-B4U2U-DK2WRW");
        assert!(closed.status.is_final());
        assert_eq!(closed.description.order_type, OrderType::Market);
        assert_eq!(closed.close_time, Some(1688665499.1922));
    }

    #[test]
    fn decodes_the_closed_orders_of_2026() {
        // shape of the answers of ClosedOrders in 2026: time_in_force in lower case, aclass
        let orders = with_txids(
            serde_json::from_str::<HashMap<String, Order>>(
                r#"{"O2FFEU-CVWWB-CX2255":{"refid":null,"userref":null,"status":"closed",
                    "opentm":1515391432.96142,"starttm":0,"expiretm":0,"closetm":1515391481.968431,
                    "descr":{"pair":"ETHEUR","aclass":"forex","type":"buy","ordertype":"limit",
                        "price":"936.91","price2":"0","leverage":"none",
                        "order":"buy 0.16000000 ETHEUR @ limit 936.91","close":""},
                    "vol":"0.16000000","vol_exec":"0.16000000","cost":"149.90560",
                    "fee":"0.17989","price":"936.91","oflags":"fciq","time_in_force":"gtc",
                    "reason":null,"misc":"","trades":["TQ4XKY-QR5ZE-JXGOGJ"]}}"#,
            )
            .unwrap(),
        );

        let order = &orders[0];
        assert_eq!(order.description.aclass.as_deref(), Some("forex"));
        assert_eq!(order.time_in_force, Some(TimeInForce::Gtc));
        assert_eq!(order.userref, None);
        assert_eq!(order.stop_price, None);
        assert_eq!(order.cost, dec!(149.9056));
    }

    #[test]
    fn decodes_the_optional_fields_of_the_documentation() {
        let order: Order = serde_json::from_str(
            r#"{"status":"open","opentm":1688666559.8974,"descr":{"pair":"XBTUSD","type":"sell",
                "ordertype":"stop-loss-limit","price":"25000","price2":"24900","order":"sell"},
                "vol":"1","vol_exec":"0","cost":"0","fee":"0","price":"0","stopprice":"25000",
                "limitprice":"24900","trigger":"index","margin":false,"amended":true,
                "sender_sub_id":"desk-1","cl_ord_id":"6d1b345e-2821-40e2-ad83-4ecb18a06876",
                "time_in_force":"GTD","expiretm":1688670000,"misc":"","oflags":"fcib"}"#,
        )
        .unwrap();

        assert_eq!(order.trigger.as_deref(), Some("index"));
        assert_eq!(order.margin, Some(false));
        assert_eq!(order.amended, Some(true));
        assert_eq!(order.sender_sub_id.as_deref(), Some("desk-1"));
        assert_eq!(order.time_in_force, Some(TimeInForce::Gtd));
        assert_eq!(order.stop_price, Some(dec!(25000.0)));
        assert_eq!(order.description.order_type, OrderType::StopLossLimit);
        assert_eq!(order.description.price2, Some(dec!(24900.0)));
        assert_eq!(order.expire_time, Some(1688670000.0));
    }

    #[test]
    fn keeps_the_exact_amounts_of_the_small_assets() {
        // 20 significant digits: a f64 rounds them, the cost would not be volume × price
        let order: Order = serde_json::from_str(
            r#"{"status":"closed","opentm":1688666559.8974,"descr":{"pair":"SHIBEUR",
                "type":"buy","ordertype":"limit","price":"0.00000123","order":"buy"},
                "vol":"812345678901.12345678","vol_exec":"812345678901.12345678",
                "cost":"999185.1850483818518394","fee":"0","price":"0.00000123"}"#,
        )
        .unwrap();

        assert_eq!(order.volume.to_string(), "812345678901.12345678");
        assert_eq!(order.volume_executed * order.average_price, order.cost);
        assert_eq!(order.description.price, Some(dec!(0.00000123)));
    }
}
