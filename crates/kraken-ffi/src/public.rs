use super::KrakenRateLimiter;
use kraken_spot_api::market::{AssetPair, Candles, CandlesRequest, ServerTime, Ticker};
use kraken_spot_api::{KrakenError, RestOptions};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(uniffi::Object)]
pub struct KrakenPublicApi {
    inner: kraken_spot_api::PublicApi,
}

#[uniffi::export(async_runtime = "tokio")]
impl KrakenPublicApi {
    #[uniffi::constructor]
    pub fn new(
        options: RestOptions,
        rate_limiter: Arc<KrakenRateLimiter>,
    ) -> Result<Arc<Self>, KrakenError> {
        Ok(Arc::new(Self {
            inner: kraken_spot_api::PublicApi::new(options, rate_limiter.inner.clone())?,
        }))
    }

    pub async fn server_time(&self) -> Result<ServerTime, KrakenError> {
        self.inner.server_time().await
    }

    pub async fn tickers(
        &self,
        pairs: Vec<String>,
    ) -> Result<HashMap<String, Ticker>, KrakenError> {
        self.inner.tickers(&pairs).await
    }

    pub async fn asset_pairs(
        &self,
        pairs: Vec<String>,
    ) -> Result<HashMap<String, AssetPair>, KrakenError> {
        self.inner.asset_pairs(&pairs).await
    }

    pub async fn candles(&self, request: CandlesRequest) -> Result<Candles, KrakenError> {
        self.inner.candles(&request).await
    }
}
