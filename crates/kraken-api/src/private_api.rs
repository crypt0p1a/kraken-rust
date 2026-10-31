use crate::account::{self, ExtendedBalance, TradeVolume};
use crate::trading::{
    self, ClosedOrders, ClosedOrdersRequest, NewOrder, NewOrderAnswer, OpenOrdersRequest, Order,
    QueryOrdersRequest,
};
use crate::{KrakenError, RateLimiter, RestOptions};
use kraken_network::PrivateClient;
use std::collections::HashMap;
use std::sync::Arc;

/// Private API of Kraken Spot, signed with the API key and its base64 secret
pub struct PrivateApi {
    client: PrivateClient,
}

impl PrivateApi {
    pub fn new(
        api_key: String,
        api_secret: String,
        options: RestOptions,
        rate_limiter: Arc<RateLimiter>,
    ) -> Result<Self, KrakenError> {
        Ok(Self {
            client: PrivateClient::new(api_key, &api_secret, options, rate_limiter)?,
        })
    }

    // account

    /// Balances per asset, with the amounts held by the open orders
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-extended-balance>
    pub async fn extended_balance(&self) -> Result<HashMap<String, ExtendedBalance>, KrakenError> {
        account::balance::extended_balance(&self.client).await
    }

    /// 30 days volume, and the current fees of the `pairs`
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-trade-volume>
    pub async fn trade_volume(&self, pairs: &[String]) -> Result<TradeVolume, KrakenError> {
        account::fees::trade_volume(&self.client, pairs).await
    }

    // trading

    /// Open orders, most recent first
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-open-orders>
    pub async fn open_orders(
        &self,
        request: &OpenOrdersRequest,
    ) -> Result<Vec<Order>, KrakenError> {
        trading::query::open_orders(&self.client, request).await
    }

    /// The open orders of a pair, given by its alternate name (e.g. "XBTUSD")
    pub async fn open_orders_for_pair(&self, pair: &str) -> Result<Vec<Order>, KrakenError> {
        let mut orders = self.open_orders(&OpenOrdersRequest::default()).await?;
        orders.retain(|order| order.description.pair == pair);
        Ok(orders)
    }

    /// A page of 50 closed orders, most recent first
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-closed-orders>
    pub async fn closed_orders(
        &self,
        request: &ClosedOrdersRequest,
    ) -> Result<ClosedOrders, KrakenError> {
        trading::query::closed_orders(&self.client, request).await
    }

    /// Open or closed orders, by Kraken identifier
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-orders-info>
    pub async fn query_orders(
        &self,
        request: &QueryOrdersRequest,
    ) -> Result<Vec<Order>, KrakenError> {
        trading::query::query_orders(&self.client, request).await
    }

    /// The order with this Kraken identifier
    pub async fn order(&self, txid: &str) -> Result<Option<Order>, KrakenError> {
        let request = QueryOrdersRequest {
            txids: vec![txid.to_string()],
            trades: None,
            userref: None,
        };

        Ok(self
            .query_orders(&request)
            .await?
            .into_iter()
            .find(|order| order.txid == txid))
    }

    /// Places an order. Prefer setting a cl_ord_id, to find the order again if the answer is lost
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/add-order>
    pub async fn new_order(&self, request: &NewOrder) -> Result<NewOrderAnswer, KrakenError> {
        trading::add::add_order(&self.client, request).await
    }
}
