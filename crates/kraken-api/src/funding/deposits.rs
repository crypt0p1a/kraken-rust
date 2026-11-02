use crate::KrakenError;
use kraken_network::{FundingClient, FundingMethod};
use serde::{Deserialize, Serialize};

/// The bank account of a fiat deposit
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FiatAccount {
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
    pub bank_address: Option<String>,
    #[serde(default)]
    pub bsb: Option<String>,
    #[serde(default)]
    pub iban: Option<String>,
    /// To put in the payment
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default)]
    pub name_on_account: Option<String>,
    #[serde(default)]
    pub routing: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub swift: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub transit: Option<String>,
}

/// Where to send a deposit: `{"crypto": {...}}` or `{"fiat": {...}}`
// a decoded answer, not moved around: no Box, which the bindings can't carry
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum AddressDetails {
    Crypto {
        address: String,
        /// Destination tag, for the networks requiring one
        #[serde(default)]
        tag: Option<String>,
        #[serde(default)]
        memo: Option<String>,
    },
    Fiat {
        #[serde(flatten)]
        account: FiatAccount,
    },
}

#[derive(Serialize)]
struct AccountQuery<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<&'a str>,
}

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
