use super::{FundingAmount, FundingAsset, FundingDirection};
use crate::account::RebaseMultiplier;
use crate::market::AssetClass;
use crate::{KrakenError, lenient};
use kraken_network::{FundingClient, FundingMethod};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};

/// The fees of a funding method, per transaction
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct MethodFees {
    pub base: FundingAmount,
    /// Percentage of the amount
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub percentage: Option<Decimal>,
    /// The fee is included in the amount of the transaction
    pub included: bool,
    #[serde(default)]
    pub min: Option<FundingAmount>,
    #[serde(default)]
    pub max: Option<FundingAmount>,
}

/// The network of a funding method
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct MethodNetwork {
    pub network_id: String,
    pub network_name: String,
    /// Contract of the token on the network
    #[serde(default)]
    pub contract_address: Option<String>,
    /// Symbol of the asset on the network
    #[serde(default)]
    pub on_chain_asset_symbol: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum AddressGenerationStatus {
    Unsupported,
    Unlimited,
    /// Up to `limit` addresses
    Limited,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AddressGeneration {
    pub status: AddressGenerationStatus,
    /// The maximum of addresses, when limited
    #[serde(default)]
    pub limit: Option<i32>,
}

/// The details of a deposit method
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct DepositMethodDetails {
    /// The method whose deposit addresses this one shares
    #[serde(default)]
    pub shares_addresses_with_method_id: Option<String>,
    /// Charged when the first address of the method is claimed
    #[serde(default)]
    pub address_setup_fee: Option<FundingAmount>,
    #[serde(default)]
    pub address_generation: Option<AddressGeneration>,
    /// Card methods only: false when only the debit cards are allowed
    #[serde(default)]
    pub allow_credit_cards: Option<bool>,
    /// The deposits of the method are exempted from its withdrawal hold
    #[serde(default)]
    pub exempted_withdrawal_hold: Option<bool>,
}

/// A way to deposit or withdraw an asset
///
/// <https://docs.kraken.com/api/docs/rest-api/list-funding-methods>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingMethodInfo {
    pub method_id: String,
    /// e.g. "Standard"
    pub method_name: String,
    pub asset: FundingAsset,
    /// Per transaction
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub minimum_amount: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub maximum_amount: Option<Decimal>,
    pub fees: MethodFees,
    #[serde(default)]
    pub network: Option<MethodNetwork>,
    /// Only for the deposit methods
    #[serde(default)]
    pub deposit: Option<DepositMethodDetails>,
    /// A withdrawal method: its details have no documented field
    #[serde(default, deserialize_with = "present")]
    pub withdrawal: bool,
}

fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    Ok(Option::<serde::de::IgnoredAny>::deserialize(deserializer)?.is_some())
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-methods>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingMethods {
    #[serde(default)]
    pub methods: Vec<FundingMethodInfo>,
    /// To give as `cursor` for the next page: None on the last one
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-methods>
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingMethodsRequest {
    /// Only the methods of this class of assets
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
    /// Only the methods of this asset: requires `asset_class`
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_name: Option<String>,
    /// Tokenized assets: the fees and limits in rebased or base units
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
    /// Methods per page
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub limit: Option<u32>,
    /// The `next_cursor` of the previous page, alone: without the other parameters
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cursor: Option<String>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub account_id: Option<String>,
}

/// The query: `asset` is a deep object, `asset[class]` and `asset[name]`
#[derive(Serialize)]
struct MethodsQuery<'a> {
    #[serde(rename = "asset[class]", skip_serializing_if = "Option::is_none")]
    asset_class: Option<AssetClass>,
    #[serde(rename = "asset[name]", skip_serializing_if = "Option::is_none")]
    asset_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rebase_multiplier: Option<RebaseMultiplier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<&'a str>,
}

impl<'a> From<&'a FundingMethodsRequest> for MethodsQuery<'a> {
    fn from(request: &'a FundingMethodsRequest) -> Self {
        MethodsQuery {
            asset_class: request.asset_class,
            asset_name: request.asset_name.as_deref(),
            rebase_multiplier: request.rebase_multiplier,
            limit: request.limit,
            cursor: request.cursor.as_deref(),
            account_id: request.account_id.as_deref(),
        }
    }
}

/// Requires the "Funds permissions - Query" permission
pub(crate) async fn funding_methods(
    client: &FundingClient,
    direction: FundingDirection,
    request: &FundingMethodsRequest,
) -> Result<FundingMethods, KrakenError> {
    if request.asset_name.is_some() && request.asset_class.is_none() {
        return Err(KrakenError::InvalidArgument {
            reason: "the asset filter requires its class".into(),
        });
    }
    client
        .call(
            FundingMethod::Get,
            &format!("/funding/v1/methods/{}", direction.as_str()),
            Some(&MethodsQuery::from(request)),
            None::<&()>,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_asset_as_a_deep_object() {
        let request = FundingMethodsRequest {
            asset_class: Some(AssetClass::Currency),
            asset_name: Some("USDC".into()),
            limit: Some(20),
            ..Default::default()
        };
        assert_eq!(
            encode(&MethodsQuery::from(&request)).unwrap(),
            "asset%5Bclass%5D=currency&asset%5Bname%5D=USDC&limit=20"
        );
    }

    #[test]
    fn decodes_the_methods() {
        // example of the documentation, plus a withdrawal method
        let methods: FundingMethods = serde_json::from_str(
            r#"{"methods":[
            {"asset":{"class":"currency","name":"USDC"},
                "method_id":"27ede8db-804b-4d91-8e25-46b7b9668730","method_name":"Standard",
                "minimum_amount":"2","fees":{"base":{"asset":{"class":"currency","name":"USDC"},
                "amount":"0.00000000"},"included":true},
                "network":{"network_id":"d9d375da-44b7-4be1-8a00-8b281acfe366",
                    "network_name":"Ethereum",
                    "contract_address":"0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                    "on_chain_asset_symbol":"USDC"},
                "deposit":{"address_generation":{"status":"limited","limit":5},
                    "shares_addresses_with_method_id":"a001231f-488e-48c0-b36c-6e0d2c1ee247"}},
            {"asset":{"class":"currency","name":"BTC"},"method_id":"m2","method_name":"Bitcoin",
                "fees":{"base":{"asset":{"class":"currency","name":"BTC"},"amount":"0.0001"},
                "percentage":"0.1","included":false},"withdrawal":{}}],
            "next_cursor":"abc"}"#,
        )
        .unwrap();
        let usdc = &methods.methods[0];
        assert_eq!(usdc.minimum_amount, Some(dec!(2)));
        assert!(usdc.fees.included && !usdc.withdrawal);
        let generation = usdc.deposit.as_ref().unwrap().address_generation.unwrap();
        assert_eq!(generation.status, AddressGenerationStatus::Limited);
        assert_eq!(generation.limit, Some(5));
        let bitcoin = &methods.methods[1];
        assert!(bitcoin.withdrawal && bitcoin.deposit.is_none());
        assert_eq!(bitcoin.fees.percentage, Some(dec!(0.1)));
        assert_eq!(methods.next_cursor.as_deref(), Some("abc"));
    }
}
