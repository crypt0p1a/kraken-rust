use crate::KrakenError;
use crate::market::AssetClass;
use kraken_network::{FundingClient, FundingMethod};
use serde::{Deserialize, Serialize};

/// Deposits or withdrawals
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum FundingDirection {
    Deposit,
    Withdraw,
}

impl FundingDirection {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            FundingDirection::Deposit => "deposit",
            FundingDirection::Withdraw => "withdraw",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NamedAsset {
    /// e.g. "USDC", "AAPLx"
    pub name: String,
}

/// The assets available for a direction, by class
///
/// <https://docs.kraken.com/api/docs/rest-api/list-funding-assets>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingAssets {
    #[serde(default)]
    pub currency: Vec<NamedAsset>,
    #[serde(default)]
    pub tokenized_asset: Vec<NamedAsset>,
}

#[derive(Serialize)]
pub(crate) struct AssetsQuery<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_class: Option<AssetClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<&'a str>,
}

/// Requires the "Funds permissions - Query" permission
pub(crate) async fn funding_assets(
    client: &FundingClient,
    direction: FundingDirection,
    asset_class: Option<AssetClass>,
    account_id: Option<&str>,
) -> Result<FundingAssets, KrakenError> {
    client
        .call(
            FundingMethod::Get,
            &format!("/funding/v1/assets/{}", direction.as_str()),
            Some(&AssetsQuery {
                asset_class,
                account_id,
            }),
            None::<&()>,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;

    #[test]
    fn encodes_the_query() {
        assert_eq!(
            encode(&AssetsQuery {
                asset_class: Some(AssetClass::TokenizedAsset),
                account_id: Some("WX6V-JUKW-KKPB-QE36"),
            })
            .unwrap(),
            "asset_class=tokenized_asset&account_id=WX6V-JUKW-KKPB-QE36"
        );
        assert_eq!(
            encode(&AssetsQuery {
                asset_class: None,
                account_id: None
            })
            .unwrap(),
            ""
        );
    }

    #[test]
    fn decodes_the_assets() {
        // example of the documentation
        let assets: FundingAssets = serde_json::from_str(
            r#"{"currency":[{"name":"USDC"},{"name":"BTC"}],"tokenized_asset":[{"name":"AAPLx"}]}"#,
        )
        .unwrap();
        assert_eq!(assets.currency[1].name, "BTC");
        assert_eq!(assets.tokenized_asset[0].name, "AAPLx");
        let none: FundingAssets = serde_json::from_str("{}").unwrap();
        assert!(none.currency.is_empty());
    }
}
