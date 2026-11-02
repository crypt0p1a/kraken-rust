use crate::market::{
    self, AssetPair, Candles, CandlesRequest, MaintenanceEvent, ServerTime, SystemStatus, Ticker,
};
use crate::{KrakenError, RateLimiter, RestOptions};
use kraken_network::PublicClient;
use std::collections::HashMap;
use std::sync::Arc;

/// Public API of Kraken Spot, no credentials required
pub struct PublicApi {
    client: PublicClient,
}

impl PublicApi {
    pub fn new(options: RestOptions, rate_limiter: Arc<RateLimiter>) -> Result<Self, KrakenError> {
        Ok(Self {
            client: PublicClient::new(options, rate_limiter)?,
        })
    }

    /// Current time of the server
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-server-time>
    pub async fn server_time(&self) -> Result<ServerTime, KrakenError> {
        market::time::server_time(&self.client).await
    }

    /// Trading mode of the exchange, with the maintenances of the next 72 hours and the incidents
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-system-status>
    pub async fn system_status(&self) -> Result<SystemStatus, KrakenError> {
        market::status::system_status(&self.client).await
    }

    /// The maintenances of the next 7 days, soonest first
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-maintenance-schedule>
    pub async fn maintenance_schedule(&self) -> Result<Vec<MaintenanceEvent>, KrakenError> {
        market::status::maintenance_schedule(&self.client).await
    }

    /// Tickers of the pairs (e.g. "XBTUSD"), keyed by their Kraken name (e.g. "XXBTZUSD")
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-ticker-information>
    pub async fn tickers(&self, pairs: &[String]) -> Result<HashMap<String, Ticker>, KrakenError> {
        market::ticker::tickers(&self.client, pairs).await
    }

    /// Parameters of the trading pairs, all of them when `pairs` is empty
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-tradable-asset-pairs>
    pub async fn asset_pairs(
        &self,
        pairs: &[String],
    ) -> Result<HashMap<String, AssetPair>, KrakenError> {
        market::pairs::asset_pairs(&self.client, pairs).await
    }

    /// OHLC candles of a pair, the last one being still open
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-ohlc-data>
    pub async fn candles(&self, request: &CandlesRequest) -> Result<Candles, KrakenError> {
        market::ohlc::candles(&self.client, request).await
    }
}
