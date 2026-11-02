use super::{
    AccountQuery, AddressDetails, AssetFilterQuery, FundingAmount, FundingScope, PageQuery,
    ScopeQuery,
};
use crate::KrakenError;
use crate::account::RebaseMultiplier;
use crate::market::AssetClass;
use kraken_network::{FundingClient, FundingMethod};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Claim<'a> {
    method_id: &'a str,
}

#[derive(Deserialize)]
struct Claimed {
    address_details: AddressDetails,
}

/// A new deposit address of the funding method. Requires the "Funds permissions - Deposit"
/// permission; refused beyond the limit of addresses of the method (`TooManyDepositAddresses`)
pub(crate) async fn claim_deposit_address(
    client: &FundingClient,
    method_id: &str,
    account_id: Option<&str>,
) -> Result<AddressDetails, KrakenError> {
    let claimed: Claimed = client
        .call(
            FundingMethod::Put,
            "/funding/v1/deposit/address",
            Some(&AccountQuery { account_id }),
            Some(&Claim { method_id }),
        )
        .await?;
    Ok(claimed.address_details)
}

/// A claimed deposit address
///
/// <https://docs.kraken.com/api/docs/rest-api/list-funding-claimed-addresses-v2>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ClaimedAddress {
    #[serde(default)]
    pub id: Option<String>,
    /// The method of the address: the requested one with a method scope
    #[serde(default)]
    pub method_id: Option<String>,
    /// The method whose address family this one shares: in the example of the documentation,
    /// not in its schema
    #[serde(default)]
    pub shares_addresses_with_method_id: Option<String>,
    /// RFC 3339
    #[serde(default)]
    pub last_used: Option<String>,
    /// RFC 3339
    #[serde(default)]
    pub expire_time: Option<String>,
    pub address_details: AddressDetails,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-claimed-addresses-v2>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ClaimedAddresses {
    #[serde(default)]
    pub addresses: Vec<ClaimedAddress>,
    /// To give as `cursor` for the next page: None on the last one
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-claimed-addresses-v2>
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ClaimedAddressesRequest {
    /// A method or a network
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub scope: Option<FundingScope>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
    /// Requires `asset_class`
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_name: Option<String>,
    /// Addresses per page, up to 500
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub limit: Option<u32>,
    /// The `next_cursor` of the previous page, alone: without the other parameters
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cursor: Option<String>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub account_id: Option<String>,
}

#[derive(Serialize)]
struct ClaimedQuery<'a> {
    #[serde(flatten)]
    scope: ScopeQuery<'a>,
    #[serde(flatten)]
    asset: AssetFilterQuery<'a>,
    #[serde(flatten)]
    page: PageQuery<'a>,
}

/// Requires the "Funds permissions - Query" permission
pub(crate) async fn claimed_addresses(
    client: &FundingClient,
    request: &ClaimedAddressesRequest,
) -> Result<ClaimedAddresses, KrakenError> {
    let query = ClaimedQuery {
        scope: ScopeQuery::new(request.scope.as_ref()),
        asset: AssetFilterQuery::new(request.asset_class, request.asset_name.as_deref())?,
        page: PageQuery {
            limit: request.limit,
            cursor: request.cursor.as_deref(),
            account_id: request.account_id.as_deref(),
        },
    };
    client
        .call(
            FundingMethod::Get,
            "/funding/v2/deposit/addresses",
            Some(&query),
            None::<&()>,
        )
        .await
}

/// The settlement of a deposit or a withdrawal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Initial,
    Pending,
    Settled,
    Success,
    Failure,
    #[serde(other)]
    Unknown,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-deposits>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Deposit {
    pub deposit_id: String,
    pub method_id: String,
    #[serde(default)]
    pub network_id: Option<String>,
    pub status: TransferStatus,
    /// None while unknown
    #[serde(default)]
    pub amount: Option<FundingAmount>,
    /// None until the deposit is complete
    #[serde(default)]
    pub fee: Option<FundingAmount>,
    /// RFC 3339
    #[serde(default)]
    pub create_time: Option<String>,
    /// The hash of the transaction on the blockchain
    #[serde(default)]
    pub onchain_transaction: Option<String>,
}

/// A page of deposits, most recent first
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Deposits {
    #[serde(default)]
    pub deposits: Vec<Deposit>,
    /// To give as `cursor` for the next page: None on the last one
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-deposits>
///
/// The filter by status isn't supported: the encoding of its list and range isn't documented
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct DepositsRequest {
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub scope: Option<FundingScope>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
    /// Requires `asset_class`
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_name: Option<String>,
    /// RFC 3339: created at or after
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start_time: Option<String>,
    /// RFC 3339
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub end_time: Option<String>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
    /// Deposits per page, up to 500
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub limit: Option<u32>,
    /// The `next_cursor` of the previous page, alone: without the other parameters
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cursor: Option<String>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub account_id: Option<String>,
}

#[derive(Serialize)]
struct DepositsQuery<'a> {
    #[serde(flatten)]
    scope: ScopeQuery<'a>,
    #[serde(flatten)]
    asset: AssetFilterQuery<'a>,
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
pub(crate) async fn deposits(
    client: &FundingClient,
    request: &DepositsRequest,
) -> Result<Deposits, KrakenError> {
    let query = DepositsQuery {
        scope: ScopeQuery::new(request.scope.as_ref()),
        asset: AssetFilterQuery::new(request.asset_class, request.asset_name.as_deref())?,
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
            "/funding/v1/deposits",
            Some(&query),
            None::<&()>,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_both_kinds_of_addresses() {
        let crypto: Claimed = serde_json::from_str(
            r#"{"address_details":{"crypto":{"address":"AikyzpZZcNAeovd6XnyEaEe6YB1tPKkXZF37rUWD"}}}"#,
        )
        .unwrap();
        assert!(matches!(
            crypto.address_details,
            AddressDetails::Crypto { tag: None, .. }
        ));

        let fiat: Claimed = serde_json::from_str(
            r#"{"address_details":{"fiat":{"iban":"DE89370400440532013000","bic":"COBADEFFXXX",
            "name_on_account":"Payward Ltd.","memo":"ABCD 1234"}}}"#,
        )
        .unwrap();
        let AddressDetails::Fiat { account } = fiat.address_details else {
            panic!("a fiat account")
        };
        assert_eq!(account.iban.as_deref(), Some("DE89370400440532013000"));
        assert_eq!(account.memo.as_deref(), Some("ABCD 1234"));
    }

    #[test]
    fn encodes_the_scope_and_the_asset_as_deep_objects() {
        let request = ClaimedAddressesRequest {
            scope: Some(FundingScope::NetworkId {
                network_id: "d9d375da".into(),
            }),
            asset_class: Some(AssetClass::Currency),
            asset_name: Some("USDC".into()),
            limit: Some(50),
            ..Default::default()
        };
        let query = ClaimedQuery {
            scope: ScopeQuery::new(request.scope.as_ref()),
            asset: AssetFilterQuery::new(request.asset_class, request.asset_name.as_deref())
                .unwrap(),
            page: PageQuery {
                limit: request.limit,
                cursor: None,
                account_id: None,
            },
        };
        assert_eq!(
            kraken_network::params::encode(&query).unwrap(),
            "scope%5Bnetwork_id%5D=d9d375da&asset%5Bclass%5D=currency&asset%5Bname%5D=USDC&limit=50"
        );
        assert!(AssetFilterQuery::new(None, Some("USDC")).is_err());
    }

    #[test]
    fn decodes_the_claimed_addresses() {
        // example of the documentation
        let addresses: ClaimedAddresses = serde_json::from_str(
            r#"{"addresses":[{"method_id":"27ede8db-804b-4d91-8e25-46b7b9668730",
            "shares_addresses_with_method_id":"a001231f-488e-48c0-b36c-6e0d2c1ee247",
            "address_details":{"crypto":{"address":"0x0d5cff23d40bcc6b98537c7ad5a839a247f546"}}}]}"#,
        )
        .unwrap();
        assert!(
            addresses.addresses[0]
                .shares_addresses_with_method_id
                .is_some()
        );
        assert_eq!(addresses.next_cursor, None);
    }

    #[test]
    fn decodes_the_deposits() {
        // example of the documentation
        let deposits: Deposits = serde_json::from_str(
            r#"{"deposits":[{"deposit_id":"FTcQ4qW-fWGQbQwUfqdnZo4dsMn1ao",
            "method_id":"3e7f8072-cc6d-4394-982a-5f4ca6ab27dd",
            "network_id":"b336ce74-8d60-42b8-8714-b1095e06b711","status":"success",
            "amount":{"asset":{"class":"currency","name":"USDC"},"amount":"20.00000000"},
            "fee":{"asset":{"class":"currency","name":"USDC"},"amount":"0.00000000"},
            "create_time":"2026-07-01T08:31:33Z",
            "onchain_transaction":"0d0a8f941a1ee0caeca3396757c847e0a3fa326d6994c320b4f283db99cd5e48"},
            {"deposit_id":"FT2","method_id":"m","status":"initial"}],"next_cursor":"gqNr"}"#,
        )
        .unwrap();
        assert_eq!(deposits.deposits[0].status, TransferStatus::Success);
        // not known yet
        assert_eq!(deposits.deposits[1].amount, None);
        assert_eq!(deposits.next_cursor.as_deref(), Some("gqNr"));
    }

    #[test]
    fn encodes_the_claim() {
        assert_eq!(
            serde_json::to_string(&Claim {
                method_id: "27ede8db-804b-4d91-8e25-46b7b9668730"
            })
            .unwrap(),
            r#"{"method_id":"27ede8db-804b-4d91-8e25-46b7b9668730"}"#
        );
    }
}
