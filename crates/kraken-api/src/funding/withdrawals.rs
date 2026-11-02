use super::{AssetFilterQuery, FundingAmount, FundingScope, ScopeQuery};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<&'a str>,
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
        limit: request.limit,
        cursor: request.cursor.as_deref(),
        account_id: request.account_id.as_deref(),
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
            limit: None,
            cursor: None,
            account_id: None,
        };
        assert_eq!(
            encode(&query).unwrap(),
            "asset%5Bclass%5D=tokenized_asset&asset%5Bname%5D=TSLAx&status=failed\
             &start_time=2026-08-01T00%3A00%3A00Z&rebase_multiplier=base"
        );
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
