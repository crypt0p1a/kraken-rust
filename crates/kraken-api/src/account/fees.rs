use crate::KrakenError;
use crate::lenient;
use crate::params::Pairs;
use kraken_network::PrivateClient;
use serde::Deserialize;
use std::collections::HashMap;

/// Fee of a pair for the current 30 days volume, in percents
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FeeInfo {
    #[serde(deserialize_with = "lenient::number")]
    pub fee: f64,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub minfee: Option<f64>,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub maxfee: Option<f64>,
    /// None at the last tier
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub nextfee: Option<f64>,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub nextvolume: Option<f64>,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub tiervolume: Option<f64>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-trade-volume>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TradeVolume {
    /// Currency of the volume, e.g. "ZUSD"
    pub currency: String,
    /// 30 days volume
    #[serde(deserialize_with = "lenient::number")]
    pub volume: f64,
    /// Taker fees, keyed by the Kraken name of the requested pairs
    #[serde(default)]
    pub fees: HashMap<String, FeeInfo>,
    /// Maker fees, keyed by the Kraken name of the requested pairs
    #[serde(default)]
    pub fees_maker: HashMap<String, FeeInfo>,
}

/// The fees are only returned for the requested `pairs`
pub(crate) async fn trade_volume(
    client: &PrivateClient,
    pairs: &[String],
) -> Result<TradeVolume, KrakenError> {
    client
        .call_with("TradeVolume", Some(&Pairs { pair: pairs }))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_trade_volume() {
        let volume: TradeVolume = serde_json::from_str(
            r#"{"currency":"ZUSD","volume":"200709587.4223",
                "fees":{"XXBTZUSD":{"fee":"0.1000","minfee":"0.1000","maxfee":"0.2600",
                    "nextfee":null,"nextvolume":null,"tiervolume":"10000000.0000"}},
                "fees_maker":{"XXBTZUSD":{"fee":"0.0000","minfee":"0.0000","maxfee":"0.1600",
                    "nextfee":null,"nextvolume":null,"tiervolume":"10000000.0000"}}}"#,
        )
        .unwrap();

        assert_eq!(volume.volume, 200709587.4223);
        assert_eq!(volume.fees["XXBTZUSD"].fee, 0.1);
        assert_eq!(volume.fees_maker["XXBTZUSD"].maxfee, Some(0.16));
        assert_eq!(volume.fees["XXBTZUSD"].nextfee, None);
    }
}
