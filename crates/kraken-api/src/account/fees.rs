use crate::KrakenError;
use crate::lenient;
use crate::params::Pairs;
use kraken_network::PrivateClient;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;

/// Fee of a pair for the current 30 days volume, in percents
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FeeInfo {
    #[serde(deserialize_with = "lenient::decimal")]
    pub fee: Decimal,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub minfee: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub maxfee: Option<Decimal>,
    /// None at the last tier
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub nextfee: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub nextvolume: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub tiervolume: Option<Decimal>,
    /// Futures volume required by the next tier, None at the last tier
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub nextfuturesvolume: Option<Decimal>,
}

/// What the volume of the account is computed from
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct VolumeInputs {
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub domain_spot_volume_30d: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub domain_futures_volume_30d: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub domain_assets_on_platform: Option<Decimal>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-trade-volume>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TradeVolume {
    /// Currency of the volume, e.g. "ZUSD"
    pub currency: String,
    /// Asset class of the volume, e.g. "currency"
    #[serde(default)]
    pub asset_class: Option<String>,
    #[serde(default)]
    pub inputs: Option<VolumeInputs>,
    /// 30 days volume
    #[serde(deserialize_with = "lenient::decimal")]
    pub volume: Decimal,
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
    use rust_decimal_macros::dec;

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

        assert_eq!(volume.volume, dec!(200709587.4223));
        assert_eq!(volume.fees["XXBTZUSD"].fee, dec!(0.1));
        assert_eq!(volume.fees_maker["XXBTZUSD"].maxfee, Some(dec!(0.16)));
        assert_eq!(volume.fees["XXBTZUSD"].nextfee, None);
        assert_eq!(volume.inputs, None);
    }

    #[test]
    fn decodes_the_fields_added_by_kraken() {
        // shape of the answers of 2026
        let volume: TradeVolume = serde_json::from_str(
            r#"{"currency":"ZUSD","asset_class":"currency","volume":"1234.5",
                "inputs":{"domain_spot_volume_30d":"1234.5","domain_assets_on_platform":"42.1",
                    "domain_futures_volume_30d":"0"},
                "fees":{"XXBTZUSD":{"fee":"0.4000","minfee":"0.0800","maxfee":"0.4000",
                    "nextfee":"0.3500","tiervolume":"0.0000","nextvolume":"10000.0000",
                    "nextfuturesvolume":"100000.0000"}},
                "fees_maker":{"XXBTZUSD":{"fee":"0.2500","minfee":"0.0000","maxfee":"0.2500",
                    "nextfee":"0.2000","tiervolume":"0.0000","nextvolume":"10000.0000",
                    "nextfuturesvolume":"100000.0000"}}}"#,
        )
        .unwrap();

        assert_eq!(volume.asset_class.as_deref(), Some("currency"));
        let inputs = volume.inputs.unwrap();
        assert_eq!(inputs.domain_spot_volume_30d, Some(dec!(1234.5)));
        assert_eq!(inputs.domain_assets_on_platform, Some(dec!(42.1)));
        assert_eq!(inputs.domain_futures_volume_30d, Some(dec!(0.0)));
        assert_eq!(
            volume.fees["XXBTZUSD"].nextfuturesvolume,
            Some(dec!(100000.0))
        );
        assert_eq!(volume.fees_maker["XXBTZUSD"].nextfee, Some(dec!(0.2)));
    }

    #[test]
    fn decodes_the_volume_without_fees() {
        let volume: TradeVolume =
            serde_json::from_str(r#"{"currency":"ZUSD","volume":"0.0000"}"#).unwrap();
        assert!(volume.fees.is_empty() && volume.fees_maker.is_empty());
        assert_eq!(volume.volume, dec!(0.0));
    }

    #[test]
    fn requires_the_currency_the_volume_and_the_fee() {
        assert!(serde_json::from_str::<TradeVolume>(r#"{"volume":"1"}"#).is_err());
        assert!(serde_json::from_str::<TradeVolume>(r#"{"currency":"ZUSD"}"#).is_err());
        assert!(serde_json::from_str::<FeeInfo>(r#"{"minfee":"0.1"}"#).is_err());
    }
}
