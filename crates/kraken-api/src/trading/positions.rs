use super::{OrderSide, OrderType};
use crate::account::RebaseMultiplier;
use crate::{KrakenError, lenient};
use kraken_network::PrivateClient;
use kraken_network::params::comma_separated;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// An open margin position
///
/// <https://docs.kraken.com/api/docs/rest-api/get-open-positions>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Position {
    /// Kraken identifier of the position (its opening trade): the key of the answer, copied here
    #[serde(skip)]
    pub txid: String,
    /// The order of the position
    #[serde(rename = "ordertxid")]
    pub order_txid: String,
    /// Asset class of the position
    #[serde(rename = "class", default)]
    pub position_class: Option<String>,
    /// "open" or "closed"
    #[serde(rename = "posstatus")]
    pub status: String,
    /// Kraken name of the pair, e.g. "XXBTZUSD"
    pub pair: String,
    /// UTC timestamp in seconds of the opening trade
    #[serde(deserialize_with = "lenient::number")]
    pub time: f64,
    #[serde(rename = "type")]
    pub side: OrderSide,
    #[serde(rename = "ordertype")]
    pub order_type: OrderType,
    /// Opening cost, in the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub cost: Decimal,
    /// Opening fee, in the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub fee: Decimal,
    /// Opening volume, in the base currency
    #[serde(rename = "vol", deserialize_with = "lenient::decimal")]
    pub volume: Decimal,
    #[serde(rename = "vol_closed", deserialize_with = "lenient::decimal")]
    pub volume_closed: Decimal,
    /// Initial margin, in the quote currency
    #[serde(deserialize_with = "lenient::decimal")]
    pub margin: Decimal,
    /// Current value of the rest of the position: with `docalcs`
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub value: Option<Decimal>,
    /// Unrealized profit / loss of the rest of the position: with `docalcs`
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub net: Option<Decimal>,
    /// Funding cost and term, e.g. "0.0100% per 4 hours"
    #[serde(default)]
    pub terms: Option<String>,
    /// UTC timestamp in seconds of the next rollover fee
    #[serde(
        rename = "rollovertm",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub rollover_time: Option<i64>,
    /// Comma separated
    #[serde(default)]
    pub misc: String,
    /// Comma separated flags of the opening order
    #[serde(default)]
    pub oflags: String,
}

/// The positions of a pair, consolidated
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ConsolidatedPosition {
    pub pair: String,
    #[serde(rename = "class")]
    pub position_class: String,
    /// Positions consolidated here
    #[serde(deserialize_with = "lenient::number")]
    pub positions: i64,
    #[serde(rename = "type")]
    pub side: OrderSide,
    /// Average leverage, or "n/a"
    pub leverage: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub cost: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub fee: Decimal,
    #[serde(rename = "vol", deserialize_with = "lenient::decimal")]
    pub volume: Decimal,
    #[serde(rename = "vol_closed", deserialize_with = "lenient::decimal")]
    pub volume_closed: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub margin: Decimal,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub value: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub net: Option<Decimal>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-open-positions>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct OpenPositionsRequest {
    /// Only these positions: all of them when empty
    #[serde(rename = "txid", serialize_with = "comma_separated")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "uniffi", uniffi(default = []))]
    pub txids: Vec<String>,
    /// Computes the value and the profit / loss of each position
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub docalcs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
}

/// The positions, most recent first
pub(crate) async fn open_positions(
    client: &PrivateClient,
    request: &OpenPositionsRequest,
) -> Result<Vec<Position>, KrakenError> {
    let positions: HashMap<String, Position> =
        client.call_with("OpenPositions", Some(request)).await?;
    let mut positions: Vec<Position> = positions
        .into_iter()
        .map(|(txid, position)| Position { txid, ..position })
        .collect();
    positions.sort_by(|a, b| b.time.total_cmp(&a.time));
    Ok(positions)
}

#[derive(Serialize)]
struct ByMarket {
    consolidation: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    docalcs: Option<bool>,
}

/// A list, or an empty object without positions, like the positions keyed by txid
fn consolidated(result: Value) -> Result<Vec<ConsolidatedPosition>, KrakenError> {
    match result {
        Value::Object(object) if object.is_empty() => Ok(vec![]),
        result => serde_json::from_value(result).map_err(KrakenError::decode),
    }
}

/// The positions consolidated by pair
pub(crate) async fn open_positions_by_market(
    client: &PrivateClient,
    docalcs: Option<bool>,
) -> Result<Vec<ConsolidatedPosition>, KrakenError> {
    let request = ByMarket {
        consolidation: "market",
        docalcs,
    };
    consolidated(client.call_with("OpenPositions", Some(&request)).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_requests() {
        let request = OpenPositionsRequest {
            txids: vec!["TF5GVO-T7ZZ2-6NBKBI".into(), "T24DOR-TAFLM-ID3NYP".into()],
            docalcs: Some(true),
            rebase_multiplier: None,
        };
        assert_eq!(
            encode(&request).unwrap(),
            "txid=TF5GVO-T7ZZ2-6NBKBI%2CT24DOR-TAFLM-ID3NYP&docalcs=true"
        );
        assert_eq!(encode(&OpenPositionsRequest::default()).unwrap(), "");
        assert_eq!(
            encode(&ByMarket {
                consolidation: "market",
                docalcs: None
            })
            .unwrap(),
            "consolidation=market"
        );
    }

    #[test]
    fn decodes_the_positions_with_a_signed_net() {
        // example of the documentation: "+154186.9728"
        let positions: HashMap<String, Position> = serde_json::from_str(
            r#"{"TF5GVO-T7ZZ2-6NBKBI":{"ordertxid":"OLWNFG-LLH4R-D6SFFP","posstatus":"open",
            "pair":"XXBTZUSD","time":1605280097.8294,"type":"buy","ordertype":"limit",
            "cost":"104610.52842","fee":"289.06565","vol":"8.82412861","vol_closed":"0.20200000",
            "margin":"20922.10568","value":"258797.5","net":"+154186.9728",
            "terms":"0.0100% per 4 hours","rollovertm":"1616672637","misc":"","oflags":""}}"#,
        )
        .unwrap();
        let position = &positions["TF5GVO-T7ZZ2-6NBKBI"];
        assert_eq!(position.net, Some(dec!(154186.9728)));
        assert_eq!(position.volume_closed, dec!(0.202));
        assert_eq!(position.rollover_time, Some(1616672637));
        assert_eq!(position.side, OrderSide::Buy);
    }

    #[test]
    fn decodes_the_consolidated_positions() {
        let positions: Vec<ConsolidatedPosition> = serde_json::from_str(
            r#"[{"pair":"XXBTZUSD","class":"margin","positions":"3","type":"buy",
            "leverage":"5.0","cost":"250367.29698","fee":"624.30622","vol":"16.82412871",
            "vol_closed":"0.20200000","margin":"50073.45939","value":"498921.5",
            "net":"-1253.1234"}]"#,
        )
        .unwrap();
        assert_eq!(positions[0].positions, 3);
        assert_eq!(positions[0].net, Some(dec!(-1253.1234)));
    }

    #[test]
    fn no_consolidated_position_is_an_empty_list() {
        assert!(consolidated(serde_json::json!({})).unwrap().is_empty());
        assert!(consolidated(serde_json::json!([])).unwrap().is_empty());
        assert!(consolidated(serde_json::json!({"a": 1})).is_err());
    }
}
