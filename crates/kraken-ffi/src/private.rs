use super::KrakenRateLimiter;
use kraken_spot_api::account::{ExtendedBalance, TradeVolume};
use kraken_spot_api::trading::{
    ClosedOrders, ClosedOrdersRequest, NewOrder, NewOrderAnswer, OpenOrdersRequest, Order,
    QueryOrdersRequest,
};
use kraken_spot_api::{KrakenError, RestOptions};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(uniffi::Object)]
pub struct KrakenPrivateApi {
    inner: kraken_spot_api::PrivateApi,
}

#[uniffi::export(async_runtime = "tokio")]
impl KrakenPrivateApi {
    #[uniffi::constructor]
    pub fn new(
        api_key: String,
        api_secret: String,
        options: RestOptions,
        rate_limiter: Arc<KrakenRateLimiter>,
    ) -> Result<Arc<Self>, KrakenError> {
        Ok(Arc::new(Self {
            inner: kraken_spot_api::PrivateApi::new(
                api_key,
                api_secret,
                options,
                rate_limiter.inner.clone(),
            )?,
        }))
    }

    pub async fn extended_balance(&self) -> Result<HashMap<String, ExtendedBalance>, KrakenError> {
        self.inner.extended_balance().await
    }

    pub async fn trade_volume(&self, pairs: Vec<String>) -> Result<TradeVolume, KrakenError> {
        self.inner.trade_volume(&pairs).await
    }

    pub async fn open_orders(&self, request: OpenOrdersRequest) -> Result<Vec<Order>, KrakenError> {
        self.inner.open_orders(&request).await
    }

    pub async fn open_orders_for_pair(&self, pair: String) -> Result<Vec<Order>, KrakenError> {
        self.inner.open_orders_for_pair(&pair).await
    }

    pub async fn closed_orders(
        &self,
        request: ClosedOrdersRequest,
    ) -> Result<ClosedOrders, KrakenError> {
        self.inner.closed_orders(&request).await
    }

    pub async fn query_orders(
        &self,
        request: QueryOrdersRequest,
    ) -> Result<Vec<Order>, KrakenError> {
        self.inner.query_orders(&request).await
    }

    pub async fn order(&self, txid: String) -> Result<Option<Order>, KrakenError> {
        self.inner.order(&txid).await
    }

    pub async fn new_order(&self, request: NewOrder) -> Result<NewOrderAnswer, KrakenError> {
        self.inner.new_order(&request).await
    }
}
