use super::{FundingAmount, FundingAsset};
use crate::market::AssetClass;
use crate::{KrakenError, lenient};
use kraken_network::{FundingClient, FundingMethod};
use rust_decimal::Decimal;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// An amount of a limit, in the asset of its policy, in USD and in the requested asset
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct LimitAmounts {
    pub policy_asset_amount: FundingAmount,
    #[serde(default)]
    pub usd_amount: Option<FundingAmount>,
    /// In `preferred_asset`, or in the asset of the method
    #[serde(default)]
    pub requested_asset_amount: Option<FundingAmount>,
}

/// The value of a limit: amounts, or a count of transactions (the `attempt` and `success`
/// limits)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum LimitValue {
    Amounts { amounts: LimitAmounts },
    Count { count: Decimal },
}

impl<'de> Deserialize<'de> for LimitValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::Object(amounts) => Ok(LimitValue::Amounts {
                amounts: serde_json::from_value(Value::Object(amounts))
                    .map_err(D::Error::custom)?,
            }),
            count => Ok(LimitValue::Count {
                count: lenient::decimal(count).map_err(D::Error::custom)?,
            }),
        }
    }
}

/// A limit over a time window
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingLimit {
    /// "equiv_amount_usd", "equiv_amount_eur", "equiv_amount_cad", "equiv_amount_gbp" and
    /// "amount": amounts; "attempt" and "success": counts
    pub limit_type: String,
    pub remaining: LimitValue,
    pub maximum: LimitValue,
    #[serde(default)]
    pub used: Option<LimitValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TimedLimit {
    /// Seconds
    #[serde(deserialize_with = "lenient::number")]
    pub time_window: i64,
    pub limit: FundingLimit,
}

/// The deposit limits of a funding method
///
/// <https://docs.kraken.com/api/docs/rest-api/list-funding-deposit-limits>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct DepositLimits {
    pub method_id: String,
    /// The maximum allowed by the limits of amount, not by the counts
    #[serde(default)]
    pub maximum_amount: Option<FundingAmount>,
    #[serde(default)]
    pub limits: Vec<TimedLimit>,
}

#[derive(Deserialize)]
struct DepositLimitsAnswer {
    #[serde(default)]
    deposit_limits: Vec<DepositLimits>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-deposit-limits>
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingLimitsRequest {
    /// The asset of the returned amounts: the one of the method when missing
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub preferred_asset_name: Option<String>,
    /// The class of `preferred_asset_name`
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub preferred_asset_class: Option<AssetClass>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub account_id: Option<String>,
}

/// The query: `preferred_asset` is a deep object
#[derive(Serialize)]
pub(crate) struct LimitsQuery<'a> {
    #[serde(
        rename = "preferred_asset[class]",
        skip_serializing_if = "Option::is_none"
    )]
    preferred_asset_class: Option<AssetClass>,
    #[serde(
        rename = "preferred_asset[name]",
        skip_serializing_if = "Option::is_none"
    )]
    preferred_asset_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<&'a str>,
}

impl<'a> LimitsQuery<'a> {
    pub(crate) fn new(request: &'a FundingLimitsRequest) -> Result<Self, KrakenError> {
        if request.preferred_asset_class.is_some() && request.preferred_asset_name.is_none() {
            return Err(KrakenError::InvalidArgument {
                reason: "the preferred asset requires its name".into(),
            });
        }
        Ok(LimitsQuery {
            preferred_asset_class: request.preferred_asset_class,
            preferred_asset_name: request.preferred_asset_name.as_deref(),
            account_id: request.account_id.as_deref(),
        })
    }
}

/// `/{direction}/{class}/{asset}`
pub(crate) fn limits_path(direction: &str, asset: &FundingAsset) -> Result<String, KrakenError> {
    let class = match asset.class {
        AssetClass::Currency => "currency",
        AssetClass::TokenizedAsset => "tokenized_asset",
        AssetClass::Unknown => {
            return Err(KrakenError::InvalidArgument {
                reason: "an unknown asset class".into(),
            });
        }
    };
    Ok(format!(
        "/funding/v1/limits/{direction}/{class}/{}",
        asset.name
    ))
}

/// The deposit limits of an asset, by funding method. Requires the "Funds permissions - Query"
/// permission
pub(crate) async fn deposit_limits(
    client: &FundingClient,
    asset: &FundingAsset,
    request: &FundingLimitsRequest,
) -> Result<Vec<DepositLimits>, KrakenError> {
    let answer: DepositLimitsAnswer = client
        .call(
            FundingMethod::Get,
            &limits_path("deposit", asset)?,
            Some(&LimitsQuery::new(request)?),
            None::<&()>,
        )
        .await?;
    Ok(answer.deposit_limits)
}

#[cfg(test)]
pub(crate) const LIMIT: &str = r#"{"time_window":"86400","limit":{"limit_type":"equiv_amount_usd",
    "remaining":{"policy_asset_amount":{"asset":{"class":"currency","name":"USD"},"amount":"499995.0013"},
        "usd_amount":{"asset":{"class":"currency","name":"USD"},"amount":"499995.0013"},
        "requested_asset_amount":{"asset":{"class":"currency","name":"USDC"},"amount":"500155.05091629"}},
    "maximum":{"policy_asset_amount":{"asset":{"class":"currency","name":"USD"},"amount":"500000.0000"}},
    "used":{"policy_asset_amount":{"asset":{"class":"currency","name":"USD"},"amount":"4.9987"}}}}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    fn usdc() -> FundingAsset {
        FundingAsset {
            class: AssetClass::Currency,
            name: "USDC".into(),
        }
    }

    #[test]
    fn encodes_the_path_and_the_query() {
        assert_eq!(
            limits_path("deposit", &usdc()).unwrap(),
            "/funding/v1/limits/deposit/currency/USDC"
        );
        let request = FundingLimitsRequest {
            preferred_asset_name: Some("EUR".into()),
            preferred_asset_class: Some(AssetClass::Currency),
            account_id: None,
        };
        assert_eq!(
            encode(&LimitsQuery::new(&request).unwrap()).unwrap(),
            "preferred_asset%5Bclass%5D=currency&preferred_asset%5Bname%5D=EUR"
        );
        let without_name = FundingLimitsRequest {
            preferred_asset_class: Some(AssetClass::Currency),
            ..Default::default()
        };
        assert!(LimitsQuery::new(&without_name).is_err());
    }

    #[test]
    fn decodes_the_amounts_and_the_counts() {
        let amounts: TimedLimit = serde_json::from_str(LIMIT).unwrap();
        assert_eq!(amounts.time_window, 86400);
        let LimitValue::Amounts { amounts: remaining } = &amounts.limit.remaining else {
            panic!("{amounts:?}")
        };
        assert_eq!(
            remaining.requested_asset_amount.as_ref().unwrap().amount,
            dec!(500155.05091629)
        );

        let counts: FundingLimit = serde_json::from_str(
            r#"{"limit_type":"attempt","remaining":"4","maximum":"5","used":"1"}"#,
        )
        .unwrap();
        assert_eq!(counts.maximum, LimitValue::Count { count: dec!(5) });
        assert_eq!(counts.used, Some(LimitValue::Count { count: dec!(1) }));
    }

    #[test]
    fn decodes_the_deposit_limits() {
        // example of the documentation
        let answer: DepositLimitsAnswer = serde_json::from_str(
            r#"{"deposit_limits":[{"method_id":"742656ee-ecc8-4287-a301-5a472ca16c24",
            "maximum_amount":{"asset":{"class":"currency","name":"USDC"},"amount":"100032.01024328"},
            "limits":[]},{"method_id":"98de2213-0e3c-422c-9b08-81078557ad3a","limits":[]}]}"#,
        )
        .unwrap();
        assert_eq!(answer.deposit_limits.len(), 2);
        assert_eq!(answer.deposit_limits[1].maximum_amount, None);
    }
}
