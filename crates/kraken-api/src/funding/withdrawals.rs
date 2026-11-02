use super::{
    AccountQuery, AssetFilterQuery, FundingAmount, FundingAsset, FundingScope, PageQuery,
    ScopeQuery,
};
use crate::account::RebaseMultiplier;
use crate::market::AssetClass;
use crate::{KrakenError, lenient};
use kraken_network::{FundingClient, FundingMethod};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum WithdrawalStatus {
    Pending,
    Success,
    Failed,
    /// Sent by Kraken but unknown to this version of the library. Never send it
    #[serde(other)]
    Unknown,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-withdrawals>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Withdrawal {
    /// e.g. "FTVZiTI-e02T84mm87JmibnObWNdnW"
    pub withdrawal_id: String,
    #[serde(default)]
    pub amount: Option<FundingAmount>,
    #[serde(default)]
    pub fee: Option<FundingAmount>,
    pub method_id: String,
    pub status: WithdrawalStatus,
    /// RFC 3339
    #[serde(default)]
    pub create_time: Option<String>,
    /// None for the withdrawals without a saved address
    #[serde(default)]
    pub address_id: Option<String>,
    #[serde(default)]
    pub onchain_transaction: Option<String>,
    /// Index of the output of the Bitcoin transaction
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub utxo_vout: Option<u64>,
}

/// A page of withdrawals, most recent first
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Withdrawals {
    #[serde(default)]
    pub withdrawals: Vec<Withdrawal>,
    /// To give as `cursor` for the next page: None on the last one
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-withdrawals>
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WithdrawalsRequest {
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub scope: Option<FundingScope>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
    /// Requires `asset_class`
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_name: Option<String>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub status: Option<WithdrawalStatus>,
    /// RFC 3339: created at or after
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start_time: Option<String>,
    /// RFC 3339
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub end_time: Option<String>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
    /// Withdrawals per page, up to 500
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub limit: Option<u32>,
    /// The `next_cursor` of the previous page, alone: without the other parameters
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cursor: Option<String>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub account_id: Option<String>,
}

#[derive(Serialize)]
struct WithdrawalsQuery<'a> {
    #[serde(flatten)]
    scope: ScopeQuery<'a>,
    #[serde(flatten)]
    asset: AssetFilterQuery<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<WithdrawalStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_time: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rebase_multiplier: Option<RebaseMultiplier>,
    #[serde(flatten)]
    page: PageQuery<'a>,
}

/// Requires the "Funds permissions - Query" permission
pub(crate) async fn withdrawals(
    client: &FundingClient,
    request: &WithdrawalsRequest,
) -> Result<Withdrawals, KrakenError> {
    let query = WithdrawalsQuery {
        scope: ScopeQuery::new(request.scope.as_ref()),
        asset: AssetFilterQuery::new(request.asset_class, request.asset_name.as_deref())?,
        status: request.status,
        start_time: request.start_time.as_deref(),
        end_time: request.end_time.as_deref(),
        rebase_multiplier: request.rebase_multiplier,
        page: PageQuery {
            limit: request.limit,
            cursor: request.cursor.as_deref(),
            account_id: request.account_id.as_deref(),
        },
    };
    client
        .call(
            FundingMethod::Get,
            "/funding/v1/withdrawals",
            Some(&query),
            None::<&()>,
        )
        .await
}

/// The fee of a new withdrawal
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum WithdrawalFee {
    /// The current fee, refused above `max_fee` (in the asset of the withdrawal) when given
    Current { max_fee: Option<String> },
    /// The rate of a quote of `funding_fee`, valid 5 minutes
    Quoted { token: String },
}

/// A withdrawal to a saved address
///
/// <https://docs.kraken.com/api/docs/rest-api/create-funding-withdrawal>
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NewWithdrawal {
    /// A method, or a network: refused by Kraken when the asset has several methods on it
    pub scope: FundingScope,
    /// The saved address, from `withdrawal_addresses`
    pub address_id: String,
    pub asset: FundingAsset,
    /// A string, to keep the precision of the caller
    pub amount: String,
    pub fee: WithdrawalFee,
    /// The amount includes the fee
    pub fee_included: bool,
    /// Tokenized assets: the amounts in rebased or base units
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
    /// Refused unless it is the address saved under `address_id`
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub expected_address: Option<String>,
}

#[derive(Serialize)]
struct AssetAmount<'a> {
    asset: &'a FundingAsset,
    amount: &'a str,
}

#[derive(Serialize)]
struct WrappedAmount<'a> {
    asset_amount: AssetAmount<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rebase_multiplier: Option<RebaseMultiplier>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum FeeChoice<'a> {
    CurrentFee {
        #[serde(skip_serializing_if = "Option::is_none")]
        max_fee: Option<WrappedAmount<'a>>,
    },
    QuotedFee {
        token: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        rebase_multiplier: Option<RebaseMultiplier>,
    },
}

#[derive(Serialize)]
struct FeeBody<'a> {
    #[serde(flatten)]
    choice: FeeChoice<'a>,
    fee_included: bool,
}

/// The JSON body of a withdrawal
#[derive(Serialize)]
struct WithdrawalBody<'a> {
    scope: &'a FundingScope,
    address_id: &'a str,
    amount: WrappedAmount<'a>,
    fee: FeeBody<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_address: Option<&'a str>,
}

impl<'a> From<&'a NewWithdrawal> for WithdrawalBody<'a> {
    fn from(withdrawal: &'a NewWithdrawal) -> Self {
        let amount = |amount: &'a str| WrappedAmount {
            asset_amount: AssetAmount {
                asset: &withdrawal.asset,
                amount,
            },
            rebase_multiplier: withdrawal.rebase_multiplier,
        };
        let choice = match &withdrawal.fee {
            WithdrawalFee::Current { max_fee } => FeeChoice::CurrentFee {
                max_fee: max_fee.as_deref().map(amount),
            },
            WithdrawalFee::Quoted { token } => FeeChoice::QuotedFee {
                token,
                rebase_multiplier: withdrawal.rebase_multiplier,
            },
        };
        WithdrawalBody {
            scope: &withdrawal.scope,
            address_id: &withdrawal.address_id,
            amount: amount(&withdrawal.amount),
            fee: FeeBody {
                choice,
                fee_included: withdrawal.fee_included,
            },
            expected_address: withdrawal.expected_address.as_deref(),
        }
    }
}

/// An amount of the answer: `{"asset_amount": {...}, "rebase_multiplier"}`
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WithdrawalAmount {
    pub asset_amount: FundingAmount,
    #[serde(default)]
    pub rebase_multiplier: Option<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/create-funding-withdrawal>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct CreatedWithdrawal {
    pub withdrawal_id: String,
    /// Sent, after the fee
    pub net_amount: WithdrawalAmount,
    /// Deducted from the account, the fee included
    pub gross_amount: WithdrawalAmount,
    pub fee: WithdrawalAmount,
    /// The withdrawal waits for this approval before being processed
    #[serde(default)]
    pub approval_request_id: Option<String>,
}

/// Requires the "Funds permissions - Withdraw" permission
pub(crate) async fn create_withdrawal(
    client: &FundingClient,
    withdrawal: &NewWithdrawal,
    account_id: Option<&str>,
) -> Result<CreatedWithdrawal, KrakenError> {
    client
        .call(
            FundingMethod::Post,
            "/funding/v1/withdrawals",
            Some(&AccountQuery { account_id }),
            Some(&WithdrawalBody::from(withdrawal)),
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_filters() {
        let query = WithdrawalsQuery {
            scope: ScopeQuery::new(None),
            asset: AssetFilterQuery::new(Some(AssetClass::TokenizedAsset), Some("TSLAx")).unwrap(),
            status: Some(WithdrawalStatus::Failed),
            start_time: Some("2026-08-01T00:00:00Z"),
            end_time: None,
            rebase_multiplier: Some(RebaseMultiplier::Base),
            page: PageQuery {
                limit: None,
                cursor: None,
                account_id: None,
            },
        };
        assert_eq!(
            encode(&query).unwrap(),
            "asset%5Bclass%5D=tokenized_asset&asset%5Bname%5D=TSLAx&status=failed\
             &start_time=2026-08-01T00%3A00%3A00Z&rebase_multiplier=base"
        );
    }

    fn usdc_withdrawal(fee: WithdrawalFee) -> NewWithdrawal {
        NewWithdrawal {
            scope: FundingScope::MethodId {
                method_id: "d4ec4d52-b159-428e-ba64-f45455a978a1".into(),
            },
            address_id: "ABR6SXP-SF6CY-VJMONY".into(),
            asset: FundingAsset {
                class: AssetClass::Currency,
                name: "USDC".into(),
            },
            amount: "5".into(),
            fee,
            fee_included: true,
            rebase_multiplier: None,
            expected_address: None,
        }
    }

    #[test]
    fn encodes_a_withdrawal_with_a_quoted_fee() {
        // example of the documentation
        let withdrawal = usdc_withdrawal(WithdrawalFee::Quoted {
            token: "YOUR_WITHDRAWAL_FEE_TOKEN".into(),
        });
        assert_eq!(
            serde_json::to_value(WithdrawalBody::from(&withdrawal)).unwrap(),
            serde_json::json!({"scope": {"method_id": "d4ec4d52-b159-428e-ba64-f45455a978a1"},
                "address_id": "ABR6SXP-SF6CY-VJMONY",
                "amount": {"asset_amount": {"asset": {"class": "currency", "name": "USDC"},
                    "amount": "5"}},
                "fee": {"quoted_fee": {"token": "YOUR_WITHDRAWAL_FEE_TOKEN"}, "fee_included": true}})
        );
    }

    #[test]
    fn encodes_a_withdrawal_with_the_current_fee() {
        let capped = usdc_withdrawal(WithdrawalFee::Current {
            max_fee: Some("1.5".into()),
        });
        assert_eq!(
            serde_json::to_value(WithdrawalBody::from(&capped)).unwrap()["fee"],
            serde_json::json!({"current_fee": {"max_fee": {"asset_amount": {"asset":
                {"class": "currency", "name": "USDC"}, "amount": "1.5"}}}, "fee_included": true})
        );
        let uncapped = usdc_withdrawal(WithdrawalFee::Current { max_fee: None });
        assert_eq!(
            serde_json::to_value(WithdrawalBody::from(&uncapped)).unwrap()["fee"],
            serde_json::json!({"current_fee": {}, "fee_included": true})
        );
    }

    #[test]
    fn decodes_the_created_withdrawal() {
        // example of the documentation
        let created: CreatedWithdrawal = serde_json::from_str(
            r#"{"withdrawal_id":"FTVZiTI-e02T84mm87JmibnObWNdnW",
            "net_amount":{"asset_amount":{"asset":{"class":"currency","name":"USDC"},"amount":"4.00000000"}},
            "gross_amount":{"asset_amount":{"asset":{"class":"currency","name":"USDC"},"amount":"5.00000000"}},
            "fee":{"asset_amount":{"asset":{"class":"currency","name":"USDC"},"amount":"1.00000000"}}}"#,
        )
        .unwrap();
        assert_eq!(
            created.gross_amount.asset_amount.amount - created.fee.asset_amount.amount,
            created.net_amount.asset_amount.amount
        );
        assert_eq!(created.approval_request_id, None);
    }

    #[test]
    fn decodes_the_withdrawals() {
        // example of the documentation
        let page: Withdrawals = serde_json::from_str(
            r#"{"withdrawals":[{"withdrawal_id":"FTVZiTI-e02T84mm87JmibnObWNdnW",
            "amount":{"asset":{"class":"currency","name":"USDC"},"amount":"4.00000000"},
            "fee":{"asset":{"class":"currency","name":"USDC"},"amount":"1.00000000"},
            "method_id":"d4ec4d52-b159-428e-ba64-f45455a978a1","status":"pending",
            "create_time":"2026-08-12T10:10:30Z","address_id":"ABR6SXP-SF6CY-VJMONY"},
            {"withdrawal_id":"FT2","method_id":"m","status":"success","utxo_vout":"1",
            "onchain_transaction":"0d0a8f94"}],"next_cursor":"gqNr"}"#,
        )
        .unwrap();
        assert_eq!(page.withdrawals[0].fee.as_ref().unwrap().amount, dec!(1));
        assert_eq!(page.withdrawals[1].utxo_vout, Some(1));
        assert_eq!(page.withdrawals[1].status, WithdrawalStatus::Success);
    }
}
