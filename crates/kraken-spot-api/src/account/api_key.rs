use crate::KrakenError;
use crate::lenient;
use kraken_network::PrivateClient;
use serde::{Deserialize, Serialize};

/// The API key of the request. Its documentation names the fields in snake case and its example
/// in camel case: both are read
///
/// <https://docs.kraken.com/api/docs/rest-api/get-api-key-info>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ApiKeyInfo {
    /// Label of the key
    #[serde(alias = "apiKeyName")]
    pub api_key_name: String,
    /// The public key, not its secret
    #[serde(alias = "apiKey")]
    pub api_key: String,
    /// Current nonce of the key
    #[serde(deserialize_with = "lenient::string")]
    pub nonce: String,
    /// Custom nonce window, 0 when not configured
    #[serde(
        alias = "nonceWindow",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub nonce_window: Option<i64>,
    /// e.g. "query-funds", "query-open-trades", "modify-trades", "withdraw-funds"
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Internal IBAN of the account
    #[serde(default)]
    pub iban: Option<String>,
    /// UTC timestamp in seconds of the expiry, 0 when never
    #[serde(
        alias = "validUntil",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub valid_until: Option<i64>,
    /// UTC timestamp in seconds of the earliest queryable date, 0 when not restricted
    #[serde(alias = "queryFrom", default, deserialize_with = "lenient::opt_number")]
    pub query_from: Option<i64>,
    /// UTC timestamp in seconds of the latest queryable date, 0 when not restricted
    #[serde(alias = "queryTo", default, deserialize_with = "lenient::opt_number")]
    pub query_to: Option<i64>,
    #[serde(
        alias = "createdTime",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub created_time: Option<i64>,
    #[serde(
        alias = "modifiedTime",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub modified_time: Option<i64>,
    /// The allowed IP addresses or ranges, empty when not restricted
    #[serde(alias = "ipAllowlist", default)]
    pub ip_allowlist: Vec<String>,
    /// None when never used
    #[serde(alias = "lastUsed", default, deserialize_with = "lenient::opt_number")]
    pub last_used: Option<i64>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-api-key-info>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ApiKeyInfoRequest {
    /// Two-factor password, when the key has one
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub otp: Option<String>,
}

/// Requires no permission
pub(crate) async fn api_key_info(
    client: &PrivateClient,
    request: &ApiKeyInfoRequest,
) -> Result<ApiKeyInfo, KrakenError> {
    client.call_with("GetApiKeyInfo", Some(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;

    #[test]
    fn encodes_the_request() {
        let request = ApiKeyInfoRequest {
            otp: Some("123456".into()),
        };
        assert_eq!(encode(&request).unwrap(), "otp=123456");
        assert_eq!(encode(&ApiKeyInfoRequest::default()).unwrap(), "");
    }

    #[test]
    fn decodes_the_example_in_camel_case() {
        let info: ApiKeyInfo = serde_json::from_str(
            r#"{"apiKeyName":"my-api-key","apiKey":"4/SDrDBcOOPnm3nPlNfEMMJDeRcIVqPz+QhRxIodyZbI9po/aVRiHsgX",
            "nonce":"1772627060997","nonceWindow":0,
            "permissions":["query-funds","withdraw-funds","query-open-trades","modify-trades"],
            "iban":"AA88 N84G WOAK NMOI","validUntil":"0","queryFrom":"0","queryTo":"0",
            "createdTime":"1772542900","modifiedTime":"1772543095","ipAllowlist":[],
            "lastUsed":"1772627061"}"#,
        )
        .unwrap();
        assert_eq!(info.api_key_name, "my-api-key");
        assert_eq!(info.nonce, "1772627060997");
        assert_eq!(info.permissions.len(), 4);
        assert_eq!(info.valid_until, Some(0));
        assert_eq!(info.last_used, Some(1772627061));
    }

    #[test]
    fn decodes_the_schema_in_snake_case() {
        let info: ApiKeyInfo = serde_json::from_str(
            r#"{"api_key_name":"bot","api_key":"key","nonce":1772627060997,"nonce_window":0,
            "permissions":["query-funds"],"valid_until":"0","created_time":"1772542900",
            "ip_allowlist":["10.0.0.0/8"],"last_used":null}"#,
        )
        .unwrap();
        assert_eq!(info.api_key_name, "bot");
        assert_eq!(info.nonce, "1772627060997");
        assert_eq!(info.ip_allowlist, ["10.0.0.0/8"]);
        assert_eq!(info.last_used, None);
    }
}
