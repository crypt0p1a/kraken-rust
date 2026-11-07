use serde::Deserialize;

/// Cursor of the paginated answers: give `next` to the following request
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Cursor {
    /// None when there is no further page
    #[serde(default)]
    pub next: Option<String>,
}
