use super::{FundingScope, ScopeQuery};
use crate::KrakenError;
use kraken_network::{FundingClient, FundingMethod};
use serde::{Deserialize, Serialize};

/// The beneficiary of a withdrawal address (travel rule)
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Beneficiary {
    /// "sender" (the user) or "other"
    #[serde(default)]
    pub recipient: Option<String>,
    /// "individual" or "business"
    #[serde(default)]
    pub typ: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub state_province: Option<String>,
    #[serde(default)]
    pub address_1: Option<String>,
    #[serde(default)]
    pub address_2: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    /// The virtual asset service provider of the counterparty
    #[serde(default)]
    pub counterparty_vasp_name: Option<String>,
}

/// A bank account, a card or a payment account of a fiat withdrawal method
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WithdrawalAccount {
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub account_type: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub bank: Option<String>,
    #[serde(default)]
    pub bank_code: Option<String>,
    #[serde(default)]
    pub bic: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub branch_code: Option<String>,
    #[serde(default)]
    pub bank_address: Option<String>,
    #[serde(default)]
    pub bsb: Option<String>,
    #[serde(default)]
    pub beneficiary: Option<Beneficiary>,
    #[serde(default)]
    pub card_number: Option<String>,
    #[serde(default)]
    pub card_number_last: Option<String>,
    #[serde(default)]
    pub card_type: Option<String>,
    #[serde(default)]
    pub cvc: Option<String>,
    #[serde(default)]
    pub expire_month: Option<String>,
    #[serde(default)]
    pub expire_year: Option<String>,
    #[serde(default)]
    pub iban: Option<String>,
    #[serde(default)]
    pub masked_iban: Option<String>,
    #[serde(default)]
    pub intermediary_address: Option<String>,
    #[serde(default)]
    pub intermediary_bank: Option<String>,
    #[serde(default)]
    pub intermediary_branch: Option<String>,
    #[serde(default)]
    pub intermediary_routing: Option<String>,
    #[serde(default)]
    pub intermediary_swift: Option<String>,
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default)]
    pub merchant_reference: Option<String>,
    #[serde(default)]
    pub name_on_account: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub routing: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub swift: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub third_party_email: Option<String>,
    #[serde(default)]
    pub transaction_id: Option<String>,
    #[serde(default)]
    pub transit: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    /// Proves the ownership of the address
    #[serde(default)]
    pub signature: Option<String>,
}

/// Where a withdrawal goes: `{"crypto": {...}}` or `{"fiat": {...}}`
// a decoded answer, not moved around: no Box, which the bindings can't carry
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum WithdrawalAddressDetails {
    Crypto {
        address: String,
        #[serde(default)]
        tag: Option<String>,
        #[serde(default)]
        memo: Option<String>,
        #[serde(default)]
        beneficiary: Option<Beneficiary>,
    },
    Fiat {
        #[serde(flatten)]
        account: WithdrawalAccount,
    },
}

/// A saved withdrawal address
///
/// <https://docs.kraken.com/api/docs/rest-api/list-funding-addresses>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WithdrawalAddress {
    /// e.g. "AB7J4FF-BGM7G-V2JMIH"
    pub address_id: String,
    pub scope: FundingScope,
    #[serde(default)]
    pub address_details: Option<WithdrawalAddressDetails>,
    /// In the example of the documentation, not in its schema
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// "third_party": the address belongs to a third party
    #[serde(default)]
    pub modifier: Option<String>,
    /// Verified, ready to be used
    pub verified: bool,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-addresses>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WithdrawalAddresses {
    pub addresses: Vec<WithdrawalAddress>,
    /// To give as `cursor` for the next page: None on the last one
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-addresses>
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WithdrawalAddressesRequest {
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub scope: Option<FundingScope>,
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
struct AddressesQuery<'a> {
    #[serde(flatten)]
    scope: ScopeQuery<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<&'a str>,
}

/// Requires the "Funds permissions - Query" permission
pub(crate) async fn withdrawal_addresses(
    client: &FundingClient,
    request: &WithdrawalAddressesRequest,
) -> Result<WithdrawalAddresses, KrakenError> {
    let query = AddressesQuery {
        scope: ScopeQuery::new(request.scope.as_ref()),
        limit: request.limit,
        cursor: request.cursor.as_deref(),
        account_id: request.account_id.as_deref(),
    };
    client
        .call(
            FundingMethod::Get,
            "/funding/v1/addresses",
            Some(&query),
            None::<&()>,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_addresses_of_every_scope() {
        // example of the documentation
        let addresses: WithdrawalAddresses = serde_json::from_str(
            r#"{"addresses":[
            {"address_id":"AB7J4FF-BGM7G-V2JMIH","scope":{"network_id":"d9d375da"},
                "address_details":{"crypto":{"address":"0xBef7B36845cA31045E86D0B46DBCac4e6752"}},
                "name":"Personal Wallet","description":"My Ethereum hardware wallet address",
                "verified":true},
            {"address_id":"ABSXEMA-3FXQS-T7Q2FG","scope":{"method_id":"67b765fd"},"verified":false,
                "address_details":{"fiat":{"iban":"DE89370400440532013000",
                    "beneficiary":{"recipient":"sender","typ":"individual"}}},
                "modifier":"third_party"},
            {"address_id":"ABR6SXP-SF6CY-VJMONY","scope":{"network_group_id":"f95acdb7"},
                "verified":true}]}"#,
        )
        .unwrap();
        let [network, method, group] = &addresses.addresses[..] else {
            panic!("{addresses:?}")
        };
        assert_eq!(
            network.scope,
            FundingScope::NetworkId {
                network_id: "d9d375da".into()
            }
        );
        assert_eq!(network.name.as_deref(), Some("Personal Wallet"));
        let Some(WithdrawalAddressDetails::Fiat { account }) = &method.address_details else {
            panic!("{method:?}")
        };
        assert_eq!(
            account.beneficiary.as_ref().unwrap().recipient.as_deref(),
            Some("sender")
        );
        assert!(matches!(group.scope, FundingScope::NetworkGroupId { .. }));
        assert_eq!(group.address_details, None);
    }
}
