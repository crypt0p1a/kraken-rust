use crate::KrakenError;
use kraken_network::PublicClient;
use serde::Deserialize;

/// Trading mode of the exchange
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum SystemState {
    /// Every order type may be submitted and trades can occur
    Online,
    /// No order can be submitted or cancelled
    Maintenance,
    /// The open orders can be cancelled, no new order can be submitted
    CancelOnly,
    /// Only the post-only limit orders can be submitted
    PostOnly,
    /// Sent by Kraken but unknown to this version of the library
    #[serde(other)]
    Unknown,
}

/// How close a scheduled maintenance is
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum MaintenancePhase {
    Announced,
    #[serde(rename = "reminder_24h")]
    Reminder24h,
    #[serde(rename = "approaching_30m")]
    Approaching30m,
    #[serde(rename = "imminent_5m")]
    Imminent5m,
    #[serde(rename = "final_warning_30s")]
    FinalWarning30s,
    #[serde(other)]
    Unknown,
}

/// Whether submitting orders is advisable during the current phase: guidance only, the state of
/// the engine is [`SystemStatus::status`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum OrderSubmission {
    Allowed,
    Discouraged,
    Blocked,
    #[serde(other)]
    Unknown,
}

/// What a client should do during the current phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum RecommendedAction {
    Continue,
    ReduceActivity,
    CancelOpenOrders,
    StopNewOrders,
    #[serde(other)]
    Unknown,
}

/// A scheduled maintenance
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct MaintenanceEvent {
    /// Stable across the phases
    pub event_id: i64,
    pub title: String,
    /// RFC 3339
    pub expected_start_utc: String,
    /// RFC 3339
    pub expected_end_utc: String,
    /// Seconds until `expected_start_utc` when the answer was served (it may be cached)
    pub time_to_start_s: i64,
    pub phase: MaintenancePhase,
    /// e.g. "spot_trading", "spot_rest", "spot_ws"
    #[serde(default)]
    pub affected_services: Vec<String>,
    pub order_submission: OrderSubmission,
    pub recommended_action: RecommendedAction,
    /// RFC 3339: the deadline to cancel the open orders
    #[serde(default)]
    pub cancel_before_utc: Option<String>,
    /// The event on the status page of Kraken
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    /// The cause isn't known yet
    Investigating,
    Identified,
    /// A fix is deployed, under watch
    Monitoring,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum IncidentImpact {
    None,
    Minor,
    Major,
    Critical,
    #[serde(other)]
    Unknown,
}

/// Forecast action of an incident
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum IncidentStepType {
    ExpectedRestart,
    ExpectedCancelOnly,
    ExpectedPostOnly,
    ExpectedOnline,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct IncidentStep {
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(rename = "type")]
    pub step_type: IncidentStepType,
    /// RFC 3339
    pub expected_at_utc: String,
}

/// An unresolved incident of the status page
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Incident {
    pub event_id: i64,
    pub title: String,
    pub incident_status: IncidentStatus,
    pub impact: IncidentImpact,
    #[serde(default)]
    pub affected_services: Vec<String>,
    /// RFC 3339
    pub started_at_utc: String,
    /// Oldest first
    #[serde(default)]
    pub next_steps: Vec<IncidentStep>,
    #[serde(default)]
    pub source_url: Option<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-system-status>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct SystemStatus {
    pub status: SystemState,
    /// RFC 3339
    pub timestamp: String,
    /// The maintenances of the next 72 hours, soonest first
    #[serde(default)]
    pub upcoming_maintenance: Vec<MaintenanceEvent>,
    #[serde(default)]
    pub emergency: Vec<Incident>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-maintenance-schedule>
#[derive(Deserialize)]
struct MaintenanceSchedule {
    /// Missing when there is none
    #[serde(default)]
    events: Vec<MaintenanceEvent>,
}

pub(crate) async fn system_status(client: &PublicClient) -> Result<SystemStatus, KrakenError> {
    client.call("SystemStatus").await
}

/// The maintenances of the next 7 days, soonest first
pub(crate) async fn maintenance_schedule(
    client: &PublicClient,
) -> Result<Vec<MaintenanceEvent>, KrakenError> {
    let schedule: MaintenanceSchedule = client.call("MaintenanceSchedule").await?;
    Ok(schedule.events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_nominal_status() {
        let status: SystemStatus =
            serde_json::from_str(r#"{"status":"online","timestamp":"2026-11-02T09:36:47Z"}"#)
                .unwrap();
        assert_eq!(status.status, SystemState::Online);
        assert!(status.upcoming_maintenance.is_empty());
        assert!(status.emergency.is_empty());
    }

    #[test]
    fn decodes_every_state() {
        for (text, state) in [
            ("online", SystemState::Online),
            ("maintenance", SystemState::Maintenance),
            ("cancel_only", SystemState::CancelOnly),
            ("post_only", SystemState::PostOnly),
            ("halted", SystemState::Unknown),
        ] {
            assert_eq!(
                serde_json::from_value::<SystemState>(text.into()).unwrap(),
                state
            );
        }
    }

    #[test]
    fn decodes_a_maintenance_and_an_incident() {
        // examples of the documentation
        let status: SystemStatus = serde_json::from_str(
            r#"{"status":"cancel_only","timestamp":"2026-05-08T14:28:00Z",
            "upcoming_maintenance":[{"event_id":21,"title":"Scheduled Maintenance - Website",
                "expected_start_utc":"2026-05-11T09:00:00Z","expected_end_utc":"2026-05-11T10:00:00Z",
                "time_to_start_s":1740,"phase":"approaching_30m","affected_services":["spot_trading"],
                "order_submission":"allowed","recommended_action":"reduce_activity",
                "cancel_before_utc":"2026-05-11T08:55:00Z",
                "source_url":"https://status.kraken.com/incidents/b7k2r9wqmn41"}],
            "emergency":[{"event_id":4821,"title":"Elevated API error rates",
                "incident_status":"identified","impact":"critical",
                "affected_services":["spot_ws","spot_rest","spot_fix"],
                "started_at_utc":"2026-05-08T14:23:11Z",
                "next_steps":[{"applies_to":["spot_trading"],"type":"expected_restart",
                    "expected_at_utc":"2026-05-08T14:30:00Z"}],
                "source_url":"https://stspg.io/c378t4f7rh0n"}]}"#,
        )
        .unwrap();

        assert_eq!(status.status, SystemState::CancelOnly);
        let maintenance = &status.upcoming_maintenance[0];
        assert_eq!(maintenance.phase, MaintenancePhase::Approaching30m);
        assert_eq!(maintenance.time_to_start_s, 1740);
        assert_eq!(
            maintenance.recommended_action,
            RecommendedAction::ReduceActivity
        );
        assert_eq!(maintenance.order_submission, OrderSubmission::Allowed);
        let incident = &status.emergency[0];
        assert_eq!(incident.impact, IncidentImpact::Critical);
        assert_eq!(incident.incident_status, IncidentStatus::Identified);
        assert_eq!(
            incident.next_steps[0].step_type,
            IncidentStepType::ExpectedRestart
        );
    }

    #[test]
    fn decodes_every_phase() {
        for (text, phase) in [
            ("announced", MaintenancePhase::Announced),
            ("reminder_24h", MaintenancePhase::Reminder24h),
            ("approaching_30m", MaintenancePhase::Approaching30m),
            ("imminent_5m", MaintenancePhase::Imminent5m),
            ("final_warning_30s", MaintenancePhase::FinalWarning30s),
            ("later", MaintenancePhase::Unknown),
        ] {
            assert_eq!(
                serde_json::from_value::<MaintenancePhase>(text.into()).unwrap(),
                phase
            );
        }
    }

    #[test]
    fn an_empty_schedule_has_no_events() {
        // what Kraken answers when nothing is scheduled
        let schedule: MaintenanceSchedule = serde_json::from_str("{}").unwrap();
        assert!(schedule.events.is_empty());
    }
}
