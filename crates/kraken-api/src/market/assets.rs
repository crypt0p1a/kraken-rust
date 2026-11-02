use crate::KrakenError;
use crate::lenient;
use kraken_network::PublicClient;
use kraken_network::params::comma_separated;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Class of an asset: the crypto and fiat currencies, or the tokenized assets (xStocks)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    Currency,
    TokenizedAsset,
    /// Sent by Kraken but unknown to this version of the library. Never send it
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    Enabled,
    DepositOnly,
    WithdrawalOnly,
    FundingTemporarilyDisabled,
    #[serde(other)]
    Unknown,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-asset-info>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Asset {
    pub aclass: AssetClass,
    /// Alternate name, e.g. "XBT" for "XXBT"
    pub altname: String,
    /// Decimals of the amounts kept by Kraken
    #[serde(deserialize_with = "lenient::number")]
    pub decimals: i32,
    /// Decimals shown by the frontends
    #[serde(deserialize_with = "lenient::number")]
    pub display_decimals: i32,
    /// Value as margin collateral, e.g. 0.995
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub collateral_value: Option<Decimal>,
    /// Maximum value of the asset counted as collateral, in USD
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub max_collateral_balance_usd_value: Option<Decimal>,
    pub status: AssetStatus,
    /// Interest rate of a margin borrow of this asset
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub margin_rate: Option<Decimal>,
    /// Tokenized assets: units of the underlying per token
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub token_multiplier: Option<Decimal>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-asset-info>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AssetsRequest {
    /// e.g. ["XBT", "ETH"]: all of them when empty
    #[serde(
        rename = "asset",
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "comma_separated"
    )]
    #[cfg_attr(feature = "uniffi", uniffi(default = []))]
    pub assets: Vec<String>,
    /// [`AssetClass::Currency`] when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub aclass: Option<AssetClass>,
}

/// Keyed by the Kraken name of the asset, e.g. "XXBT"
pub(crate) async fn assets(
    client: &PublicClient,
    request: &AssetsRequest,
) -> Result<HashMap<String, Asset>, KrakenError> {
    client.call_with("Assets", Some(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_request() {
        let request = AssetsRequest {
            assets: vec!["XBT".into(), "ETH".into()],
            aclass: Some(AssetClass::TokenizedAsset),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "asset=XBT%2CETH&aclass=tokenized_asset"
        );
        assert_eq!(encode(&AssetsRequest::default()).unwrap(), "");
    }

    #[test]
    fn decodes_the_assets() {
        // as sent by Kraken: collateral_value a number, margin_rate a string
        let assets: HashMap<String, Asset> = serde_json::from_str(
            r#"{"XXBT":{"aclass":"currency","altname":"XBT","decimals":10,"display_decimals":5,
                "collateral_value":1,"status":"enabled","margin_rate":"0.02"},
                "AALx":{"aclass":"tokenized_asset","altname":"AALx","decimals":8,
                "display_decimals":6,"status":"withdrawal_only","token_multiplier":"1"}}"#,
        )
        .unwrap();

        let bitcoin = &assets["XXBT"];
        assert_eq!(bitcoin.aclass, AssetClass::Currency);
        assert_eq!(bitcoin.decimals, 10);
        assert_eq!(bitcoin.collateral_value, Some(dec!(1)));
        assert_eq!(bitcoin.margin_rate, Some(dec!(0.02)));
        let stock = &assets["AALx"];
        assert_eq!(stock.aclass, AssetClass::TokenizedAsset);
        assert_eq!(stock.status, AssetStatus::WithdrawalOnly);
        assert_eq!(stock.token_multiplier, Some(dec!(1)));
        assert_eq!(stock.margin_rate, None);
    }

    #[test]
    fn keeps_the_unknown_values() {
        let asset: Asset = serde_json::from_str(
            r#"{"aclass":"bond","altname":"B","decimals":"2","display_decimals":2,"status":"frozen"}"#,
        )
        .unwrap();
        assert_eq!(asset.aclass, AssetClass::Unknown);
        assert_eq!(asset.status, AssetStatus::Unknown);
        assert_eq!(asset.decimals, 2);
    }
}
