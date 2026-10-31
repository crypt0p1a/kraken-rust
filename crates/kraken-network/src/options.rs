/// Options of the HTTP client
#[derive(Debug, Clone, PartialEq)]
pub struct RestOptions {
    /// Timeout of the connection, of the socket and of the whole request, in milliseconds
    pub timeout_ms: u64,
    /// Rejects the invalid TLS certificates
    pub reject_unauthorized: bool,
    pub host: String,
}

impl Default for RestOptions {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            reject_unauthorized: true,
            host: "https://api.kraken.com".into(),
        }
    }
}
