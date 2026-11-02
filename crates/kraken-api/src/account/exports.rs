use crate::KrakenError;
use kraken_network::PrivateClient;
use kraken_network::params::comma_separated;
use serde::{Deserialize, Serialize};

/// The data of a report
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum ExportReport {
    Trades,
    Ledgers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "UPPERCASE")]
pub enum ExportFormat {
    Csv,
    Tsv,
}

/// <https://docs.kraken.com/api/docs/rest-api/add-export>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ExportRequest {
    pub report: ExportReport,
    /// CSV when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub format: Option<ExportFormat>,
    pub description: String,
    /// The fields of the report, e.g. ["ordertxid", "time", "price"]: all of them when empty
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "comma_separated"
    )]
    #[cfg_attr(feature = "uniffi", uniffi(default = []))]
    pub fields: Vec<String>,
    /// UTC timestamp in seconds of the start: the 1st of the current month when missing
    #[serde(rename = "starttm", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start_time: Option<i64>,
    /// UTC timestamp in seconds of the end: now when missing
    #[serde(rename = "endtm", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub end_time: Option<i64>,
}

#[derive(Deserialize)]
struct ExportId {
    id: String,
}

/// The identifier of the requested report. Requires the "Export data" permission
pub(crate) async fn add_export(
    client: &PrivateClient,
    request: &ExportRequest,
) -> Result<String, KrakenError> {
    let answer: ExportId = client.call_with("AddExport", Some(request)).await?;
    Ok(answer.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;

    #[test]
    fn encodes_the_export_request() {
        let request = ExportRequest {
            report: ExportReport::Ledgers,
            format: Some(ExportFormat::Tsv),
            description: "my ledgers".into(),
            fields: vec!["refid".into(), "time".into()],
            start_time: Some(1688169600),
            end_time: Some(1688774400),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "report=ledgers&format=TSV&description=my+ledgers&fields=refid%2Ctime\
             &starttm=1688169600&endtm=1688774400"
        );
        let request = ExportRequest {
            report: ExportReport::Trades,
            format: None,
            description: "trades".into(),
            fields: vec![],
            start_time: None,
            end_time: None,
        };
        assert_eq!(
            encode(&request).unwrap(),
            "report=trades&description=trades"
        );
    }

    #[test]
    fn decodes_the_identifier() {
        let answer: ExportId = serde_json::from_str(r#"{"id":"TCJA"}"#).unwrap();
        assert_eq!(answer.id, "TCJA");
    }
}
