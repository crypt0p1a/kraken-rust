use crate::market::{
    self, Asset, AssetPair, AssetsRequest, Candles, CandlesRequest, GroupedOrderBook,
    GroupedOrderBookRequest, MaintenanceEvent, OrderBook, OrderBookRequest, RecentSpreads,
    RecentSpreadsRequest, RecentTrades, RecentTradesRequest, ServerTime, SystemStatus, Ticker,
};
use crate::transparency::{self, PostTradeData, PostTradeRequest, PreTradeData};
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

    /// The assets, keyed by their Kraken name (e.g. "XXBT")
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-asset-info>
    pub async fn assets(
        &self,
        request: &AssetsRequest,
    ) -> Result<HashMap<String, Asset>, KrakenError> {
        market::assets::assets(&self.client, request).await
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

    /// Level 2 order book of a pair: the aggregated volume of each price level
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-order-book>
    pub async fn order_book(&self, request: &OrderBookRequest) -> Result<OrderBook, KrakenError> {
        market::book::order_book(&self.client, request).await
    }

    /// Order book of a pair grouped over ranges of ticks
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-grouped-order-book>
    pub async fn grouped_order_book(
        &self,
        request: &GroupedOrderBookRequest,
    ) -> Result<GroupedOrderBook, KrakenError> {
        market::book::grouped_order_book(&self.client, request).await
    }

    /// The last trades of a pair, 1000 by default
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-recent-trades>
    pub async fn recent_trades(
        &self,
        request: &RecentTradesRequest,
    ) -> Result<RecentTrades, KrakenError> {
        market::trades::recent_trades(&self.client, request).await
    }

    /// The last ~200 best bids and asks of a pair
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-recent-spreads>
    pub async fn recent_spreads(
        &self,
        request: &RecentSpreadsRequest,
    ) -> Result<RecentSpreads, KrakenError> {
        market::trades::recent_spreads(&self.client, request).await
    }

    /// OHLC candles of a pair, the last one being still open
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-ohlc-data>
    pub async fn candles(&self, request: &CandlesRequest) -> Result<Candles, KrakenError> {
        market::ohlc::candles(&self.client, request).await
    }

    // transparency

    /// The top 10 levels of each side of the order book of `symbol` (e.g. "BTC/USD")
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-pre-trade-data>
    pub async fn pre_trade(&self, symbol: &str) -> Result<PreTradeData, KrakenError> {
        transparency::pre_trade(&self.client, symbol).await
    }

    /// The trades of the exchange, the last 1000 of every pair by default
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-post-trade-data>
    pub async fn post_trade(
        &self,
        request: &PostTradeRequest,
    ) -> Result<PostTradeData, KrakenError> {
        transparency::post_trade(&self.client, request).await
    }
}
