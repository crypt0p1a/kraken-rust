use crate::KrakenError;
use crate::lenient;
use crate::params::Pairs;
use kraken_network::PublicClient;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

/// Fee applied from a 30 days volume
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FeeTier {
    /// 30 days volume, in `fee_volume_currency`
    pub volume: f64,
    /// Percentage, e.g. 0.26 for 0.26%
    pub percent: f64,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-tradable-asset-pairs>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AssetPair {
    /// Alternate name, e.g. "XBTUSD": the name to give to the other calls
    pub altname: String,
    /// Name used by the WebSocket API, e.g. "XBT/USD"
    #[serde(default)]
    pub wsname: Option<String>,
    #[serde(default)]
    pub aclass_base: Option<String>,
    /// Asset of the base, e.g. "XXBT"
    pub base: String,
    #[serde(default)]
    pub aclass_quote: Option<String>,
    /// Asset of the quote, e.g. "ZUSD"
    pub quote: String,
    /// Unit of the volume, e.g. "unit"
    #[serde(default)]
    pub lot: Option<String>,
    /// Decimals of the price
    #[serde(deserialize_with = "lenient::number")]
    pub pair_decimals: i32,
    /// Decimals of the cost (price × volume), in the quote
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub cost_decimals: Option<i32>,
    /// Decimals of the volume, in the base
    #[serde(deserialize_with = "lenient::number")]
    pub lot_decimals: i32,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub lot_multiplier: Option<i32>,
    #[serde(default)]
    pub leverage_buy: Vec<i32>,
    #[serde(default)]
    pub leverage_sell: Vec<i32>,
    /// Taker fees
    #[serde(default, deserialize_with = "fee_tiers")]
    pub fees: Vec<FeeTier>,
    #[serde(default, deserialize_with = "fee_tiers")]
    pub fees_maker: Vec<FeeTier>,
    #[serde(default)]
    pub fee_volume_currency: Option<String>,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub margin_call: Option<i32>,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub margin_stop: Option<i32>,
    /// Minimum volume of an order, in the base
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub ordermin: Option<f64>,
    /// Minimum cost of an order, in the quote
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub costmin: Option<f64>,
    /// Minimum increment of the price
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub tick_size: Option<f64>,
    #[serde(default)]
    pub status: Option<AssetPairStatus>,
    /// Where the orders are executed, e.g. "international"
    #[serde(default)]
    pub execution_venue: Option<String>,
    /// Maximum long margin position, in the base. Only for the pairs with margin
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub long_position_limit: Option<i64>,
    /// Maximum short margin position, in the base. Only for the pairs with margin
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub short_position_limit: Option<i64>,
}

/// Trading status of a pair
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum AssetPairStatus {
    Online,
    CancelOnly,
    PostOnly,
    LimitOnly,
    ReduceOnly,
    /// Sent by Kraken but unknown to this version of the library
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct RawFeeTier(
    #[serde(deserialize_with = "lenient::number")] f64,
    #[serde(deserialize_with = "lenient::number")] f64,
);

/// The tiers are sent as [[volume, percent], ...]
fn fee_tiers<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<FeeTier>, D::Error> {
    Ok(Vec::<RawFeeTier>::deserialize(deserializer)?
        .into_iter()
        .map(|RawFeeTier(volume, percent)| FeeTier { volume, percent })
        .collect())
}

/// Keyed by the Kraken name of the pair (e.g. "XXBTZUSD"). All the pairs when `pairs` is empty
pub(crate) async fn asset_pairs(
    client: &PublicClient,
    pairs: &[String],
) -> Result<HashMap<String, AssetPair>, KrakenError> {
    client
        .call_with("AssetPairs", Some(&Pairs { pair: pairs }))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_asset_pairs() {
        let pairs: HashMap<String, AssetPair> = serde_json::from_str(
            r#"{"XXBTZUSD":{"altname":"XBTUSD","wsname":"XBT/USD","aclass_base":"currency",
                "base":"XXBT","aclass_quote":"currency","quote":"ZUSD","lot":"unit",
                "cost_decimals":5,"pair_decimals":1,"lot_decimals":8,"lot_multiplier":1,
                "leverage_buy":[2,3,4,5],"leverage_sell":[2,3,4,5],
                "fees":[[0,0.4],[10000,0.35]],"fees_maker":[[0,0.25],[10000,0.2]],
                "fee_volume_currency":"ZUSD","margin_call":80,"margin_stop":40,
                "ordermin":"0.00005","costmin":"0.5","tick_size":"0.1","status":"online"}}"#,
        )
        .unwrap();

        let pair = &pairs["XXBTZUSD"];
        assert_eq!(pair.altname, "XBTUSD");
        assert_eq!(pair.lot_decimals, 8);
        assert_eq!(
            pair.fees_maker[1],
            FeeTier {
                volume: 10000.0,
                percent: 0.2
            }
        );
        assert_eq!(pair.ordermin, Some(0.00005));
        assert_eq!(pair.tick_size, Some(0.1));
    }

    #[test]
    fn decodes_the_fields_added_by_kraken() {
        // shape of the answers of 2026, with and without margin
        let pairs: HashMap<String, AssetPair> = serde_json::from_str(
            r#"{"XXBTZUSD":{"altname":"XBTUSD","wsname":"XBT/USD","aclass_base":"currency",
                "base":"XXBT","aclass_quote":"currency","quote":"ZUSD","lot":"unit",
                "cost_decimals":5,"pair_decimals":1,"lot_decimals":8,"lot_multiplier":1,
                "leverage_buy":[2,3],"leverage_sell":[2,3],"fees":[],"fees_maker":[],
                "fee_volume_currency":"ZUSD","margin_call":80,"margin_stop":40,
                "ordermin":"0.00006","costmin":"0.5","tick_size":"0.1","status":"online",
                "execution_venue":"international","long_position_limit":350,
                "short_position_limit":250},
            "0GEUR":{"altname":"0GEUR","wsname":"0G/EUR","aclass_base":"currency","base":"0G",
                "aclass_quote":"currency","quote":"ZEUR","lot":"unit5","cost_decimals":5,
                "pair_decimals":4,"lot_decimals":8,"lot_multiplier":1,"leverage_buy":[],
                "leverage_sell":[],"fees":[],"fees_maker":[],"fee_volume_currency":"ZUSD",
                "margin_call":80,"margin_stop":40,"ordermin":"1","costmin":"0.45",
                "tick_size":"0.0001","status":"cancel_only","execution_venue":"international"}}"#,
        )
        .unwrap();

        let btc = &pairs["XXBTZUSD"];
        assert_eq!(btc.lot.as_deref(), Some("unit"));
        assert_eq!(btc.status, Some(AssetPairStatus::Online));
        assert_eq!(btc.execution_venue.as_deref(), Some("international"));
        assert_eq!(btc.long_position_limit, Some(350));
        assert_eq!(btc.short_position_limit, Some(250));
        assert!(btc.fees.is_empty());

        let other = &pairs["0GEUR"];
        assert_eq!(other.status, Some(AssetPairStatus::CancelOnly));
        assert_eq!(other.long_position_limit, None);
    }

    #[test]
    fn decodes_every_status() {
        for (text, expected) in [
            ("online", AssetPairStatus::Online),
            ("cancel_only", AssetPairStatus::CancelOnly),
            ("post_only", AssetPairStatus::PostOnly),
            ("limit_only", AssetPairStatus::LimitOnly),
            ("reduce_only", AssetPairStatus::ReduceOnly),
            ("delisted", AssetPairStatus::Unknown),
        ] {
            let decoded: AssetPairStatus = serde_json::from_str(&format!("\"{text}\"")).unwrap();
            assert_eq!(decoded, expected, "{text}");
        }
    }

    #[test]
    fn decodes_a_pair_with_only_the_required_fields() {
        let pair: AssetPair = serde_json::from_str(
            r#"{"altname":"XBTUSD","base":"XXBT","quote":"ZUSD","pair_decimals":"1",
                "lot_decimals":8}"#,
        )
        .unwrap();
        assert_eq!(pair.pair_decimals, 1);
        assert!(pair.fees.is_empty() && pair.leverage_buy.is_empty());
        assert_eq!(pair.status, None);
        assert_eq!(pair.ordermin, None);
    }

    #[test]
    fn requires_the_identity_and_the_decimals() {
        for field in ["altname", "base", "quote", "pair_decimals", "lot_decimals"] {
            let mut value: serde_json::Value = serde_json::from_str(
                r#"{"altname":"XBTUSD","base":"XXBT","quote":"ZUSD","pair_decimals":1,
                    "lot_decimals":8}"#,
            )
            .unwrap();
            value.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<AssetPair>(value).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn rejects_the_fee_tiers_of_another_size() {
        let pair = serde_json::from_str::<AssetPair>(
            r#"{"altname":"XBTUSD","base":"XXBT","quote":"ZUSD","pair_decimals":1,
                "lot_decimals":8,"fees":[[0,0.26,1]]}"#,
        );
        assert!(pair.is_err());
    }
}
