use super::FundingAmount;
use crate::account::RebaseMultiplier;
use crate::{KrakenError, lenient};
use kraken_network::{FundingClient, FundingMethod};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FeeDetails {
    /// The fixed part of the fee
    pub base_fee: FundingAmount,
    /// The percentage part of the fee
    #[serde(deserialize_with = "lenient::decimal")]
    pub fee_percentage: Decimal,
}

/// The fee of a deposit or a withdrawal
///
/// <https://docs.kraken.com/api/docs/rest-api/calculate-funding-fees>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingFee {
    pub fee: FundingAmount,
    /// The amount with the fee
    pub gross_amount: FundingAmount,
    /// The amount without the fee
    pub net_amount: FundingAmount,
    pub fee_details: FeeDetails,
    /// Withdrawals: pins the quoted rate for `create_withdrawal`, for 5 minutes
    #[serde(default)]
    pub withdrawal_fee_token: Option<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/calculate-funding-fees>
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingFeeRequest {
    /// A string, to keep the precision of the caller
    pub amount: String,
    /// The amount includes the fee
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub fee_included: Option<bool>,
    /// A quote calculated before, to get its rate again
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub withdrawal_fee_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub account_id: Option<String>,
}

/// The fee of `amount` with the funding method. Requires the "Funds permissions - Query"
/// permission
pub(crate) async fn funding_fee(
    client: &FundingClient,
    method_id: &str,
    request: &FundingFeeRequest,
) -> Result<FundingFee, KrakenError> {
    client
        .call(
            FundingMethod::Get,
            &format!("/funding/v1/fees/{method_id}"),
            Some(request),
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
    fn encodes_the_request() {
        let request = FundingFeeRequest {
            amount: "5".into(),
            fee_included: Some(true),
            withdrawal_fee_token: Some("AAAA,MBri".into()),
            ..Default::default()
        };
        assert_eq!(
            encode(&request).unwrap(),
            "amount=5&fee_included=true&withdrawal_fee_token=AAAA%2CMBri"
        );
    }

    #[test]
    fn decodes_the_fee() {
        // example of the documentation
        let fee: FundingFee = serde_json::from_str(
            r#"{"fee":{"asset":{"class":"currency","name":"USDC"},"amount":"1.00000000"},
            "gross_amount":{"asset":{"class":"currency","name":"USDC"},"amount":"5.00000000"},
            "net_amount":{"asset":{"class":"currency","name":"USDC"},"amount":"4.00000000"},
            "fee_details":{"base_fee":{"asset":{"class":"currency","name":"USDC"},
                "amount":"1.00000000"},"fee_percentage":"0"},
            "withdrawal_fee_token":"AAAAAAAAAAHG33Wc1eES6Qpe"}"#,
        )
        .unwrap();
        assert_eq!(
            fee.gross_amount.amount - fee.fee.amount,
            fee.net_amount.amount
        );
        assert_eq!(fee.fee_details.fee_percentage, dec!(0));
        assert!(fee.withdrawal_fee_token.is_some());
    }
}
