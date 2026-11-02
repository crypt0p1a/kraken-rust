use crate::KrakenError;
use kraken_network::{FundingClient, FundingMethod};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingNetwork {
    pub network_id: String,
    /// e.g. "Arbitrum One"
    pub name: String,
}

/// Networks sharing the same address format, e.g. "EVM"
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NetworkGroup {
    pub network_group_id: String,
    pub name: String,
    #[serde(default)]
    pub network_ids: Vec<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-funding-networks>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingNetworks {
    #[serde(default)]
    pub network_groups: Vec<NetworkGroup>,
    #[serde(default)]
    pub networks: Vec<FundingNetwork>,
}

#[derive(Serialize)]
struct NetworksQuery<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<&'a str>,
}

/// Requires the "Funds permissions - Query" permission
pub(crate) async fn funding_networks(
    client: &FundingClient,
    account_id: Option<&str>,
) -> Result<FundingNetworks, KrakenError> {
    client
        .call(
            FundingMethod::Get,
            "/funding/v1/networks",
            Some(&NetworksQuery { account_id }),
            None::<&()>,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_networks_and_their_groups() {
        // example of the documentation
        let networks: FundingNetworks = serde_json::from_str(
            r#"{"network_groups":[{"network_group_id":"f95acdb7-48fb-4441-b5b4-843d3bf60e61",
            "name":"EVM","network_ids":["2c6eeff9-bd5f-4c09-982d-a5855fa9f0dd"]}],
            "networks":[{"network_id":"2c6eeff9-bd5f-4c09-982d-a5855fa9f0dd","name":"Arbitrum Nova"}]}"#,
        )
        .unwrap();
        assert_eq!(
            networks.network_groups[0].network_ids[0],
            networks.networks[0].network_id
        );
    }
}
