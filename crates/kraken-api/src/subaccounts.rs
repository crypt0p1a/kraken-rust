//! Private subaccounts: for the institutional accounts only, called with a key of the master
//! account having the "Funds permissions - Withdraw" permission

use crate::KrakenError;
use kraken_network::PrivateClient;
use serde::Serialize;

/// <https://docs.kraken.com/api/docs/rest-api/create-subaccount>
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NewSubaccount {
    pub username: String,
    pub email: String,
}

/// Whether the subaccount was created
pub(crate) async fn create_subaccount(
    client: &PrivateClient,
    subaccount: &NewSubaccount,
) -> Result<bool, KrakenError> {
    client.call_with("CreateSubaccount", Some(subaccount)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;

    #[test]
    fn encodes_the_new_subaccount() {
        let subaccount = NewSubaccount {
            username: "bot-1".into(),
            email: "bot+1@example.com".into(),
        };
        assert_eq!(
            encode(&subaccount).unwrap(),
            "username=bot-1&email=bot%2B1%40example.com"
        );
    }
}
