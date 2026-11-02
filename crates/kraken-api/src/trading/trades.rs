use super::{OrderSide, OrderType};
use crate::account::RebaseMultiplier;
use crate::{Cursor, KrakenError, lenient};
use kraken_network::PrivateClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A trade (fill) of the account
///
/// <https://docs.kraken.com/api/docs/rest-api/get-trade-history>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Trade {
    /// Kraken identifier of the trade: the key of the answers, copied here
    #[serde(skip)]
    pub txid: String,
    /// The order of the trade
    #[serde(rename = "ordertxid")]
    pub order_txid: String,
    /// The position of the trade
    #[serde(rename = "postxid", default)]
    pub position_txid: Option<String>,
    /// Kraken name of the pair, e.g. "XXBTZUSD"
    pub pair: String,
    /// UTC timestamp in seconds, with decimals
    #[serde(deserialize_with = "lenient::number")]
    pub time: f64,
    #[serde(rename = "type")]
    pub side: OrderSide,
    /// Type of the order
    #[serde(rename = "ordertype")]
    pub order_type: OrderType,
    /// Type of the order of the trade, when converted from `order_type`
    #[serde(rename = "tradeordertype", default)]
    pub trade_order_type: Option<OrderType>,
    /// Average price, in the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub price: Decimal,
    /// In the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub cost: Decimal,
    /// In the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub fee: Decimal,
    /// In the base currency
    #[serde(rename = "vol", deserialize_with = "lenient::decimal")]
    pub volume: Decimal,
    /// Initial margin, in the quote currency
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub margin: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_string")]
    pub leverage: Option<String>,
    /// Comma separated: closing, ...
    #[serde(default)]
    pub misc: String,
    /// The ledger entries of the trade, when requested with `ledgers`
    #[serde(default)]
    pub ledgers: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient::number")]
    pub trade_id: i64,
    /// The account was the maker, not the taker
    #[serde(default)]
    pub maker: Option<bool>,
    /// Asset class of the pair, e.g. "forex"
    #[serde(default)]
    pub aclass: Option<String>,
    /// External execution identifier, if any
    #[serde(default)]
    pub ext_exec_id: Option<String>,
    /// The trades opening a position: "open" or "closed"
    #[serde(rename = "posstatus", default)]
    pub position_status: Option<String>,
    /// Average price of the closed part of the position
    #[serde(rename = "cprice", default, deserialize_with = "lenient::opt_decimal")]
    pub closed_price: Option<Decimal>,
    #[serde(rename = "ccost", default, deserialize_with = "lenient::opt_decimal")]
    pub closed_cost: Option<Decimal>,
    #[serde(rename = "cfee", default, deserialize_with = "lenient::opt_decimal")]
    pub closed_fee: Option<Decimal>,
    #[serde(rename = "cvol", default, deserialize_with = "lenient::opt_decimal")]
    pub closed_volume: Option<Decimal>,
    #[serde(rename = "cmargin", default, deserialize_with = "lenient::opt_decimal")]
    pub closed_margin: Option<Decimal>,
    /// Net profit / loss of the closed part of the position
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub net: Option<Decimal>,
    /// The trades closing the position
    #[serde(default)]
    pub trades: Option<Vec<String>>,
}

/// Trades keyed by txid → trades holding their txid, most recent first
pub(crate) fn with_txids(trades: HashMap<String, Trade>) -> Vec<Trade> {
    let mut trades: Vec<Trade> = trades
        .into_iter()
        .map(|(txid, trade)| Trade { txid, ..trade })
        .collect();
    trades.sort_by(|a, b| b.time.total_cmp(&a.time));
    trades
}

/// Which trades of the margin positions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum TradeHistoryType {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "any position")]
    AnyPosition,
    #[serde(rename = "closed position")]
    ClosedPosition,
    #[serde(rename = "closing position")]
    ClosingPosition,
    #[serde(rename = "no position")]
    NoPosition,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-trade-history>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TradesHistoryRequest {
    /// All when missing
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub trade_type: Option<TradeHistoryType>,
    /// Includes the trades of the positions
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub trades: Option<bool>,
    /// UTC timestamp in seconds or trade txid (exclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start: Option<i64>,
    /// UTC timestamp in seconds or trade txid (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub end: Option<i64>,
    /// Offset of the page: deprecated by Kraken, prefer `with_cursor`
    #[serde(rename = "ofs", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub offset: Option<i32>,
    /// No count of the trades: faster for the accounts with many trades
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub without_count: Option<bool>,
    /// Consolidates the trades by taker trade
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub consolidate_taker: Option<bool>,
    /// Includes the ledger entries of each trade
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub ledgers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
    /// Asset class of `pair`, e.g. "forex"
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub aclass: Option<String>,
    /// Only the trades of this pair, e.g. "XBTUSD"
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub pair: Option<String>,
    /// Trades per page, 1 to 100
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub limit: Option<u32>,
    /// Pagination by cursor instead of `offset`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub with_cursor: Option<bool>,
    /// The `cursor.next` of the previous page: requires `with_cursor`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cursor: Option<String>,
}

/// A page of trades
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TradesHistory {
    /// Most recent first
    pub trades: Vec<Trade>,
    /// Trades matching the request: 0 with `without_count` or `with_cursor`
    pub count: i64,
    /// Only with `with_cursor`, when more trades are available
    pub cursor: Option<Cursor>,
}

#[derive(Deserialize)]
struct HistoryAnswer {
    #[serde(default)]
    trades: HashMap<String, Trade>,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    count: Option<i64>,
    #[serde(default)]
    cursor: Option<Cursor>,
}

pub(crate) async fn trades_history(
    client: &PrivateClient,
    request: &TradesHistoryRequest,
) -> Result<TradesHistory, KrakenError> {
    let answer: HistoryAnswer = client.call_with("TradesHistory", Some(request)).await?;
    Ok(TradesHistory {
        trades: with_txids(answer.trades),
        count: answer.count.unwrap_or_default(),
        cursor: answer.cursor,
    })
}

#[cfg(test)]
pub(crate) const SAMPLE: &str = r#"{
    "THVRQM-33VKH-UCI7BS":{"ordertxid":"OQCLML-BW3P3-BUCMWZ","postxid":"TKH2SE-M7IF5-CFI7LT",
        "pair":"XXBTZUSD","time":1688667796.8802,"type":"buy","ordertype":"limit",
        "price":"30010.00000","cost":"616.79361","fee":"1.60366","vol":"0.02055308",
        "margin":"0.00000","leverage":"0","misc":"","trade_id":93748276,"maker":true,
        "ledgers":["LQ64ZJ-7WE4M-ODNVSY"],"aclass":"forex"},
    "TCWJEG-FL4SZ-3FKGH6":{"ordertxid":"OB5VMB-B4U2U-DK2WRW","postxid":"TKH2SE-M7IF5-CFI7LT",
        "pair":"XXBTZUSD","time":1688667769.6396,"type":"sell","ordertype":"market",
        "price":"30000.80000","cost":"7500.20000","fee":"19.50052","vol":"0.25000000",
        "margin":"0.00000","misc":"","trade_id":93748270,"maker":false,
        "posstatus":"closed","cprice":"30100.0","ccost":"7525.0","cfee":"19.56","cvol":"0.25",
        "cmargin":"0","net":"24.80","trades":["TZX2WP-XSEOP-FP7WYR"]}
}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_every_parameter() {
        let request = TradesHistoryRequest {
            trade_type: Some(TradeHistoryType::ClosingPosition),
            trades: Some(true),
            start: Some(1),
            end: Some(2),
            offset: Some(0),
            without_count: Some(true),
            consolidate_taker: Some(false),
            ledgers: Some(true),
            rebase_multiplier: Some(RebaseMultiplier::Base),
            aclass: Some("forex".into()),
            pair: Some("XBTUSD".into()),
            limit: Some(100),
            with_cursor: Some(true),
            cursor: Some("next page".into()),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "type=closing+position&trades=true&start=1&end=2&ofs=0&without_count=true\
             &consolidate_taker=false&ledgers=true&rebase_multiplier=base&aclass=forex\
             &pair=XBTUSD&limit=100&with_cursor=true&cursor=next+page"
        );
        assert_eq!(encode(&TradesHistoryRequest::default()).unwrap(), "");
    }

    #[test]
    fn sends_every_type() {
        for (trade_type, text) in [
            (TradeHistoryType::All, "all"),
            (TradeHistoryType::AnyPosition, "any position"),
            (TradeHistoryType::ClosedPosition, "closed position"),
            (TradeHistoryType::ClosingPosition, "closing position"),
            (TradeHistoryType::NoPosition, "no position"),
        ] {
            assert_eq!(
                serde_json::to_value(trade_type).unwrap(),
                serde_json::Value::from(text)
            );
        }
    }

    #[test]
    fn decodes_the_trades_most_recent_first() {
        let answer: HistoryAnswer = serde_json::from_str(&format!(
            r#"{{"trades":{SAMPLE},"count":2,"cursor":{{"next":"abc"}}}}"#
        ))
        .unwrap();
        assert_eq!(answer.count, Some(2));
        assert_eq!(answer.cursor.unwrap().next.as_deref(), Some("abc"));

        let trades = with_txids(answer.trades);
        assert_eq!(trades[0].txid, "THVRQM-33VKH-UCI7BS");
        assert_eq!(trades[0].side, OrderSide::Buy);
        assert_eq!(trades[0].volume, dec!(0.02055308));
        assert_eq!(trades[0].maker, Some(true));
        let closing = &trades[1];
        assert_eq!(closing.order_type, OrderType::Market);
        assert_eq!(closing.net, Some(dec!(24.8)));
        assert_eq!(
            closing.trades.as_deref(),
            Some(&["TZX2WP-XSEOP-FP7WYR".to_string()][..])
        );
    }

    #[test]
    fn decodes_a_page_without_count() {
        // with_cursor: no count, no cursor on the last page
        let answer: HistoryAnswer = serde_json::from_str(r#"{"trades":{},"count":null}"#).unwrap();
        assert_eq!(answer.count, None);
        assert!(answer.cursor.is_none());
    }

    #[test]
    fn requires_the_amounts() {
        assert!(
            serde_json::from_str::<Trade>(
                r#"{"ordertxid":"O","pair":"P","time":1,"type":"buy","ordertype":"limit",
                "price":"1","cost":"1","fee":"0","trade_id":1}"#
            )
            .is_err(),
            "vol is required"
        );
    }
}
