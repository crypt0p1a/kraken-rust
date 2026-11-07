use crate::{KrakenError, lenient};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum ExportStatus {
    Queued,
    Processing,
    /// Ready to be retrieved
    Processed,
    #[serde(other)]
    Unknown,
}

/// A requested report
///
/// <https://docs.kraken.com/api/docs/rest-api/export-status>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Export {
    pub id: String,
    #[serde(rename = "descr")]
    pub description: String,
    /// "CSV" or "TSV"
    pub format: String,
    pub report: ExportReport,
    #[serde(default)]
    pub subtype: Option<String>,
    pub status: ExportStatus,
    /// e.g. "EExport:Canceled", "EExport:Exported size too big"
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub flags: Option<String>,
    /// The fields of the report, e.g. "all"
    #[serde(default)]
    pub fields: Option<String>,
    /// UTC timestamps in seconds
    #[serde(
        rename = "createdtm",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub created_time: Option<i64>,
    #[serde(rename = "expiretm", default, deserialize_with = "lenient::opt_number")]
    pub expire_time: Option<i64>,
    /// When its processing began
    #[serde(rename = "starttm", default, deserialize_with = "lenient::opt_number")]
    pub start_time: Option<i64>,
    #[serde(
        rename = "completedtm",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub completed_time: Option<i64>,
    /// Start of the data of the report
    #[serde(
        rename = "datastarttm",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub data_start_time: Option<i64>,
    /// End of the data of the report
    #[serde(
        rename = "dataendtm",
        default,
        deserialize_with = "lenient::opt_number"
    )]
    pub data_end_time: Option<i64>,
    #[serde(rename = "endtm", default, deserialize_with = "lenient::opt_number")]
    pub end_time: Option<i64>,
    #[serde(default)]
    pub aclass: Option<String>,
    #[serde(default)]
    pub asset: Option<String>,
    #[serde(default)]
    pub asset_classes: Option<Vec<String>>,
    /// Pending deletion
    #[serde(default)]
    pub delete: Option<bool>,
}

#[derive(Serialize)]
struct Report {
    report: ExportReport,
}

/// The reports of a kind. Requires the "Export data" permission
pub(crate) async fn export_status(
    client: &PrivateClient,
    report: ExportReport,
) -> Result<Vec<Export>, KrakenError> {
    client
        .call_with("ExportStatus", Some(&Report { report }))
        .await
}

#[derive(Serialize)]
struct Id<'a> {
    id: &'a str,
}

/// The zip archive of a processed report. Requires the "Export data" permission
pub(crate) async fn retrieve_export(
    client: &PrivateClient,
    id: &str,
) -> Result<Vec<u8>, KrakenError> {
    client.call_bytes("RetrieveExport", Some(&Id { id })).await
}

/// How to remove a report
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum RemoveExportType {
    /// A queued or processing report
    Cancel,
    /// A processed report
    Delete,
}

#[derive(Serialize)]
struct Removal<'a> {
    id: &'a str,
    #[serde(rename = "type")]
    removal: RemoveExportType,
}

/// `{"delete": true}` or `{"cancel": true}`, depending on the removal
#[derive(Deserialize)]
struct Removed {
    #[serde(default)]
    delete: Option<bool>,
    #[serde(default)]
    cancel: Option<bool>,
}

/// Whether the report was removed. Requires the "Export data" permission
pub(crate) async fn remove_export(
    client: &PrivateClient,
    id: &str,
    removal: RemoveExportType,
) -> Result<bool, KrakenError> {
    let removed: Removed = client
        .call_with("RemoveExport", Some(&Removal { id, removal }))
        .await?;
    Ok(removed.delete.or(removed.cancel).unwrap_or(false))
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
    fn encodes_the_status_request() {
        assert_eq!(
            encode(&Report {
                report: ExportReport::Trades
            })
            .unwrap(),
            "report=trades"
        );
    }

    #[test]
    fn decodes_the_exports() {
        // example of the documentation
        let exports: Vec<Export> = serde_json::from_str(
            r#"[{"id":"VSKC","descr":"my_trades_1","format":"CSV","report":"trades",
            "subtype":"all","status":"Processed","flags":"0","fields":"all",
            "createdtm":"1688669085","expiretm":"1688878685","starttm":"1688669093",
            "completedtm":"1688669093","datastarttm":"1683556800","dataendtm":"1688669085",
            "aclass":"forex","asset":"all"}]"#,
        )
        .unwrap();
        let export = &exports[0];
        assert_eq!(export.status, ExportStatus::Processed);
        assert_eq!(export.report, ExportReport::Trades);
        assert_eq!(export.completed_time, Some(1688669093));
        assert_eq!(export.error, None);
        assert_eq!(
            serde_json::from_value::<ExportStatus>("Archived".into()).unwrap(),
            ExportStatus::Unknown
        );
    }

    #[test]
    fn encodes_the_identifier() {
        assert_eq!(encode(&Id { id: "TCJA" }).unwrap(), "id=TCJA");
    }

    #[test]
    fn encodes_the_removal() {
        assert_eq!(
            encode(&Removal {
                id: "TCJA",
                removal: RemoveExportType::Cancel
            })
            .unwrap(),
            "id=TCJA&type=cancel"
        );
        assert_eq!(
            encode(&Removal {
                id: "TCJA",
                removal: RemoveExportType::Delete
            })
            .unwrap(),
            "id=TCJA&type=delete"
        );
    }

    #[test]
    fn decodes_both_removals() {
        let deleted: Removed = serde_json::from_str(r#"{"delete":true}"#).unwrap();
        assert_eq!(deleted.delete.or(deleted.cancel), Some(true));
        let cancelled: Removed = serde_json::from_str(r#"{"cancel":false}"#).unwrap();
        assert_eq!(cancelled.delete.or(cancelled.cancel), Some(false));
    }

    #[test]
    fn decodes_the_identifier() {
        let answer: ExportId = serde_json::from_str(r#"{"id":"TCJA"}"#).unwrap();
        assert_eq!(answer.id, "TCJA");
    }
}
