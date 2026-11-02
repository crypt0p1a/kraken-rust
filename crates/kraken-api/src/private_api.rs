use crate::Decimal;
use crate::account::{
    self, ApiKeyInfo, ApiKeyInfoRequest, BalanceRequest, CreditLines, ExtendedBalance, Ledgers,
    LedgersRequest, TradeBalance, TradeBalanceRequest, TradeVolume, WalletAccounts,
};
use crate::market::{self, Level3OrderBook, Level3OrderBookRequest};
use crate::trading::{
    self, ClosedOrders, ClosedOrdersRequest, ConsolidatedPosition, NewOrder, NewOrderAnswer,
    OpenOrdersRequest, OpenPositionsRequest, Order, OrderAmends, OrderAmendsRequest, Position,
    QueryOrdersRequest, QueryTradesRequest, Trade, TradesHistory, TradesHistoryRequest,
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

    // market

    /// Level 3 order book of a pair: every order of each price level
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-level-3-order-book>
    pub async fn level3_order_book(
        &self,
        request: &Level3OrderBookRequest,
    ) -> Result<Level3OrderBook, KrakenError> {
        market::book::level3_order_book(&self.client, request).await
    }

    // account

    /// The API key of the requests: name, permissions, restrictions. Requires no permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-api-key-info>
    pub async fn api_key_info(
        &self,
        request: &ApiKeyInfoRequest,
    ) -> Result<ApiKeyInfo, KrakenError> {
        account::api_key::api_key_info(&self.client, request).await
    }

    /// The cash balances, net of the pending withdrawals, keyed by asset (e.g. "XXBT")
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-account-balance>
    pub async fn balance(
        &self,
        request: &BalanceRequest,
    ) -> Result<HashMap<String, Decimal>, KrakenError> {
        account::balance::balance(&self.client, request).await
    }

    /// The credit lines of the account and their usage: VIP accounts only
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-credit-lines>
    pub async fn credit_lines(&self, request: &BalanceRequest) -> Result<CreditLines, KrakenError> {
        account::credit::credit_lines(&self.client, request).await
    }

    /// Balances per asset, with the amounts held by the open orders
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-extended-balance>
    pub async fn extended_balance(&self) -> Result<HashMap<String, ExtendedBalance>, KrakenError> {
        account::balance::extended_balance(&self.client).await
    }

    /// Collateral, margin positions, equity and margin level, in one asset
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-trade-balance>
    pub async fn trade_balance(
        &self,
        request: &TradeBalanceRequest,
    ) -> Result<TradeBalance, KrakenError> {
        account::trade_balance::trade_balance(&self.client, request).await
    }

    /// The wallets of the account, with the `account_id` of each
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-wallet-accounts>
    pub async fn wallet_accounts(&self) -> Result<WalletAccounts, KrakenError> {
        account::wallets::wallet_accounts(&self.client).await
    }

    /// A page of 50 ledger entries at most, most recent first. Requires the "Query ledger entries"
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-ledgers>
    pub async fn ledgers(&self, request: &LedgersRequest) -> Result<Ledgers, KrakenError> {
        account::ledgers::ledgers(&self.client, request).await
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

    /// The versions of an order, the original one first
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-order-amends>
    pub async fn order_amends(
        &self,
        request: &OrderAmendsRequest,
    ) -> Result<OrderAmends, KrakenError> {
        trading::amends::order_amends(&self.client, request).await
    }

    /// Places an order. Prefer setting a cl_ord_id, to find the order again if the answer is lost
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/add-order>
    pub async fn new_order(&self, request: &NewOrder) -> Result<NewOrderAnswer, KrakenError> {
        trading::add::add_order(&self.client, request).await
    }

    /// A page of the trades (fills) of the account, most recent first
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-trade-history>
    pub async fn trades_history(
        &self,
        request: &TradesHistoryRequest,
    ) -> Result<TradesHistory, KrakenError> {
        trading::trades::trades_history(&self.client, request).await
    }

    /// Trades by Kraken identifier, most recent first
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-trades-info>
    pub async fn query_trades(
        &self,
        request: &QueryTradesRequest,
    ) -> Result<Vec<Trade>, KrakenError> {
        trading::trades::query_trades(&self.client, request).await
    }

    /// The open margin positions, most recent first
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-open-positions>
    pub async fn open_positions(
        &self,
        request: &OpenPositionsRequest,
    ) -> Result<Vec<Position>, KrakenError> {
        trading::positions::open_positions(&self.client, request).await
    }

    /// The open margin positions consolidated by pair, with their value and profit / loss when
    /// `docalcs`
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-open-positions>
    pub async fn open_positions_by_market(
        &self,
        docalcs: Option<bool>,
    ) -> Result<Vec<ConsolidatedPosition>, KrakenError> {
        trading::positions::open_positions_by_market(&self.client, docalcs).await
    }
}
