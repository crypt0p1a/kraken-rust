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
    /// online, cancel_only, post_only, limit_only, reduce_only
    #[serde(default)]
    pub status: Option<String>,
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
}
