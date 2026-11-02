use crate::Decimal;
use crate::account::{
    self, ApiKeyInfo, ApiKeyInfoRequest, BalanceRequest, CreditLines, Export, ExportReport,
    ExportRequest, ExtendedBalance, LedgerEntry, Ledgers, LedgersRequest, QueryLedgersRequest,
    RemoveExportType, TradeBalance, TradeBalanceRequest, TradeVolume, WalletAccounts,
};
use crate::algo::{
    self, ClosedStrategies, ClosedStrategiesRequest, NewStrategy, Strategy, StrategyType,
};
use crate::earn::{
    self, EarnAllocations, EarnAllocationsRequest, EarnFunds, EarnStrategies, EarnStrategiesRequest,
};
use crate::funding::{
    self, AddressDetails, ClaimedAddresses, ClaimedAddressesRequest, CreatedWithdrawal,
    DepositLimits, Deposits, DepositsRequest, FundingAsset, FundingAssets, FundingDirection,
    FundingFee, FundingFeeRequest, FundingLimitsRequest, FundingMethods, FundingMethodsRequest,
    FundingNetworks, NewWithdrawal, NewWithdrawalAddress, SavedAddress, WithdrawalAddresses,
    WithdrawalAddressesRequest, WithdrawalLimits, Withdrawals, WithdrawalsRequest,
};
use crate::market::{self, AssetClass, Level3OrderBook, Level3OrderBookRequest};
use crate::subaccounts::{self, AccountTransfer, NewSubaccount, TransferResult};
use crate::trading::{
    self, AddOrderBatch, AmendOrder, BatchOrderAnswer, CancelAllAfter, CancelledOrders,
    ClosedOrders, ClosedOrdersRequest, ConsolidatedPosition, EditOrder, EditedOrder, NewOrder,
    NewOrderAnswer, OpenOrdersRequest, OpenPositionsRequest, Order, OrderAmends,
    OrderAmendsRequest, OrderReference, Position, QueryOrdersRequest, QueryTradesRequest, Trade,
    TradesHistory, TradesHistoryRequest, WebSocketsToken,
};
use crate::{KrakenError, RateLimiter, RestOptions};
use kraken_network::{FundingClient, PrivateClient};
use std::collections::HashMap;
use std::sync::Arc;

/// Private API of Kraken Spot, signed with the API key and its base64 secret
pub struct PrivateApi {
    client: PrivateClient,
    /// The same key, on the funding API
    funding: FundingClient,
}

impl PrivateApi {
    pub fn new(
        api_key: String,
        api_secret: String,
        options: RestOptions,
        rate_limiter: Arc<RateLimiter>,
    ) -> Result<Self, KrakenError> {
        Ok(Self {
            funding: FundingClient::new(
                api_key.clone(),
                &api_secret,
                options.clone(),
                rate_limiter.clone(),
            )?,
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

    /// Requests a report of the trades or of the ledgers, returns its identifier. Requires the
    /// "Export data" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/add-export>
    pub async fn add_export(&self, request: &ExportRequest) -> Result<String, KrakenError> {
        account::exports::add_export(&self.client, request).await
    }

    /// The requested reports of the trades or of the ledgers. Requires the "Export data"
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/export-status>
    pub async fn export_status(&self, report: ExportReport) -> Result<Vec<Export>, KrakenError> {
        account::exports::export_status(&self.client, report).await
    }

    /// The zip archive of a processed report. Requires the "Export data" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/retrieve-export>
    pub async fn retrieve_export(&self, id: &str) -> Result<Vec<u8>, KrakenError> {
        account::exports::retrieve_export(&self.client, id).await
    }

    /// Cancels a queued or processing report, or deletes a processed one: whether it was removed.
    /// Requires the "Export data" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/remove-export>
    pub async fn remove_export(
        &self,
        id: &str,
        removal: RemoveExportType,
    ) -> Result<bool, KrakenError> {
        account::exports::remove_export(&self.client, id, removal).await
    }

    /// A page of 50 ledger entries at most, most recent first. Requires the "Query ledger entries"
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-ledgers>
    pub async fn ledgers(&self, request: &LedgersRequest) -> Result<Ledgers, KrakenError> {
        account::ledgers::ledgers(&self.client, request).await
    }

    /// Ledger entries by id, most recent first. Requires the "Query ledger entries" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-ledgers-info>
    pub async fn query_ledgers(
        &self,
        request: &QueryLedgersRequest,
    ) -> Result<Vec<LedgerEntry>, KrakenError> {
        account::ledgers::query_ledgers(&self.client, request).await
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

    /// Cancels an open order, or the open orders of a user reference
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/cancel-order>
    pub async fn cancel_order(
        &self,
        order: &OrderReference,
    ) -> Result<CancelledOrders, KrakenError> {
        trading::cancel::cancel_order(&self.client, order).await
    }

    /// Cancels every open order of the account
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/cancel-all-orders>
    pub async fn cancel_all(&self) -> Result<CancelledOrders, KrakenError> {
        trading::cancel::cancel_all(&self.client).await
    }

    /// Dead man's switch: cancels every open order in `timeout` seconds (less than 86400), unless
    /// called again before. 0 disables it
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/cancel-all-orders-after>
    pub async fn cancel_all_orders_after(
        &self,
        timeout: u32,
    ) -> Result<CancelAllAfter, KrakenError> {
        trading::cancel::cancel_all_orders_after(&self.client, timeout).await
    }

    /// Cancels up to 50 open orders
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/cancel-order-batch>
    pub async fn cancel_order_batch(
        &self,
        orders: &[OrderReference],
    ) -> Result<CancelledOrders, KrakenError> {
        trading::cancel::cancel_order_batch(&self.client, orders).await
    }

    /// Amends an open order in place, keeping its identifiers and its priority: the identifier of
    /// the amend
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/amend-order>
    pub async fn amend_order(&self, request: &AmendOrder) -> Result<String, KrakenError> {
        trading::amends::amend_order(&self.client, request).await
    }

    /// Replaces an open order by a new one, with a new txid: prefer `amend_order`, which keeps it
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/edit-order>
    pub async fn edit_order(&self, request: &EditOrder) -> Result<EditedOrder, KrakenError> {
        trading::edit::edit_order(&self.client, request).await
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

    /// The token of the private feeds of the WebSocket API. Requires the "WebSocket interface -
    /// On" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-websockets-token>
    pub async fn websockets_token(&self) -> Result<WebSocketsToken, KrakenError> {
        trading::websocket::websockets_token(&self.client).await
    }

    /// Places 2 to 15 orders of a pair, validated as a whole: the answer of each order, in the
    /// order of the request
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/add-order-batch>
    pub async fn add_order_batch(
        &self,
        request: &AddOrderBatch,
    ) -> Result<Vec<BatchOrderAnswer>, KrakenError> {
        trading::batch::add_order_batch(&self.client, request).await
    }

    // algo

    /// The pending and running TWAP strategies, the most recent start first
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-open-strategies>
    pub async fn open_strategies(
        &self,
        strategy_type: Option<StrategyType>,
    ) -> Result<Vec<Strategy>, KrakenError> {
        algo::open_strategies(&self.client, strategy_type).await
    }

    /// Up to 20 strategies by id, the most recent start first: the unknown ids are omitted
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/query-strategies>
    pub async fn query_strategies(
        &self,
        strategy_ids: &[String],
    ) -> Result<Vec<Strategy>, KrakenError> {
        algo::query_strategies(&self.client, strategy_ids).await
    }

    /// A page of the closed, expired and cancelled strategies, most recent first
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-closed-strategies>
    pub async fn closed_strategies(
        &self,
        request: &ClosedStrategiesRequest,
    ) -> Result<ClosedStrategies, KrakenError> {
        algo::closed_strategies(&self.client, request).await
    }

    /// Creates a TWAP strategy: its id, empty when only validated
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/add-strategy>
    pub async fn add_strategy(&self, request: &NewStrategy) -> Result<String, KrakenError> {
        algo::add_strategy(&self.client, request).await
    }

    /// Cancels a pending or open strategy: a child order already sent may still fill
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/cancel-strategy>
    pub async fn cancel_strategy(&self, strategy_id: &str) -> Result<String, KrakenError> {
        algo::cancel_strategy(&self.client, strategy_id).await
    }

    // earn

    /// A page of the Earn strategies available to the account. Requires a valid key, no
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-strategies>
    pub async fn earn_strategies(
        &self,
        request: &EarnStrategiesRequest,
    ) -> Result<EarnStrategies, KrakenError> {
        earn::strategies(&self.client, request).await
    }

    /// The allocations of the account to the Earn strategies, by state, with their rewards.
    /// Requires the "Query Funds" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-allocations>
    pub async fn earn_allocations(
        &self,
        request: &EarnAllocationsRequest,
    ) -> Result<EarnAllocations, KrakenError> {
        earn::allocations(&self.client, request).await
    }

    /// Starts allocating funds to an Earn strategy: asynchronous, followed by
    /// `earn_allocate_status`. Requires the "Earn Funds" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/allocate-strategy>
    pub async fn earn_allocate(&self, funds: &EarnFunds) -> Result<bool, KrakenError> {
        earn::allocate(&self.client, funds).await
    }

    /// Whether the last allocation to an Earn strategy is still in progress. Requires the "Earn
    /// Funds" or "Query Funds" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-allocate-strategy-status>
    pub async fn earn_allocate_status(&self, strategy_id: &str) -> Result<bool, KrakenError> {
        earn::allocate_status(&self.client, strategy_id).await
    }

    /// Starts deallocating funds from an Earn strategy: asynchronous, followed by
    /// `earn_deallocate_status`. Requires the "Earn Funds" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/deallocate-strategy>
    pub async fn earn_deallocate(&self, funds: &EarnFunds) -> Result<bool, KrakenError> {
        earn::deallocate(&self.client, funds).await
    }

    /// Whether the last deallocation from an Earn strategy is still in progress. Requires the
    /// "Earn Funds" or "Query Funds" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/get-deallocate-strategy-status>
    pub async fn earn_deallocate_status(&self, strategy_id: &str) -> Result<bool, KrakenError> {
        earn::deallocate_status(&self.client, strategy_id).await
    }

    // subaccounts

    /// Creates a trading subaccount: institutional accounts only, with a key of the master
    /// account having the "Funds permissions - Withdraw" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/create-subaccount>
    pub async fn create_subaccount(&self, subaccount: &NewSubaccount) -> Result<bool, KrakenError> {
        subaccounts::create_subaccount(&self.client, subaccount).await
    }

    /// Transfers funds between the master account and a subaccount: institutional accounts only,
    /// with a key of the master account having the "Funds permissions - Withdraw" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/account-transfer>
    pub async fn account_transfer(
        &self,
        transfer: &AccountTransfer,
    ) -> Result<TransferResult, KrakenError> {
        subaccounts::account_transfer(&self.client, transfer).await
    }

    // funding

    /// The assets which can be deposited or withdrawn, by class. Requires the "Funds permissions
    /// - Query" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-funding-assets>
    pub async fn funding_assets(
        &self,
        direction: FundingDirection,
        asset_class: Option<AssetClass>,
        account_id: Option<&str>,
    ) -> Result<FundingAssets, KrakenError> {
        funding::assets::funding_assets(&self.funding, direction, asset_class, account_id).await
    }

    /// A page of the ways to deposit or withdraw: methods, networks, fees, minimums and maximums.
    /// Requires the "Funds permissions - Query" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-funding-methods>
    pub async fn funding_methods(
        &self,
        direction: FundingDirection,
        request: &FundingMethodsRequest,
    ) -> Result<FundingMethods, KrakenError> {
        funding::methods::funding_methods(&self.funding, direction, request).await
    }

    /// The funding networks, and their groups sharing an address format. Requires the "Funds
    /// permissions - Query" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-funding-networks>
    pub async fn funding_networks(
        &self,
        account_id: Option<&str>,
    ) -> Result<FundingNetworks, KrakenError> {
        funding::networks::funding_networks(&self.funding, account_id).await
    }

    /// The fee of an amount with a funding method; for a withdrawal, a token pinning its rate for
    /// 5 minutes. Requires the "Funds permissions - Query" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/calculate-funding-fees>
    pub async fn funding_fee(
        &self,
        method_id: &str,
        request: &FundingFeeRequest,
    ) -> Result<FundingFee, KrakenError> {
        funding::fees::funding_fee(&self.funding, method_id, request).await
    }

    /// The deposit limits of an asset, by funding method. Requires the "Funds permissions - Query"
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-funding-deposit-limits>
    pub async fn deposit_limits(
        &self,
        asset: &FundingAsset,
        request: &FundingLimitsRequest,
    ) -> Result<Vec<DepositLimits>, KrakenError> {
        funding::limits::deposit_limits(&self.funding, asset, request).await
    }

    /// The balance available for withdrawal and the withdrawal limits of an asset, by funding
    /// method. Requires the "Funds permissions - Query" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-funding-withdrawal-limits>
    pub async fn withdrawal_limits(
        &self,
        asset: &FundingAsset,
        request: &FundingLimitsRequest,
    ) -> Result<WithdrawalLimits, KrakenError> {
        funding::limits::withdrawal_limits(&self.funding, asset, request).await
    }

    /// Claims a new deposit address of a funding method. Requires the "Funds permissions -
    /// Deposit" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/claim-funding-deposit-address>
    pub async fn claim_deposit_address(
        &self,
        method_id: &str,
        account_id: Option<&str>,
    ) -> Result<AddressDetails, KrakenError> {
        funding::deposits::claim_deposit_address(&self.funding, method_id, account_id).await
    }

    /// A page of the claimed deposit addresses. Requires the "Funds permissions - Query"
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-funding-claimed-addresses-v2>
    pub async fn claimed_deposit_addresses(
        &self,
        request: &ClaimedAddressesRequest,
    ) -> Result<ClaimedAddresses, KrakenError> {
        funding::deposits::claimed_addresses(&self.funding, request).await
    }

    /// A page of the deposits, most recent first. Requires the "Funds permissions - Query"
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-funding-deposits>
    pub async fn deposits(&self, request: &DepositsRequest) -> Result<Deposits, KrakenError> {
        funding::deposits::deposits(&self.funding, request).await
    }

    /// A page of the saved withdrawal addresses. Requires the "Funds permissions - Query"
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-funding-addresses>
    pub async fn withdrawal_addresses(
        &self,
        request: &WithdrawalAddressesRequest,
    ) -> Result<WithdrawalAddresses, KrakenError> {
        funding::addresses::withdrawal_addresses(&self.funding, request).await
    }

    /// Saves a crypto withdrawal address. Requires the "Funds permissions - Add withdrawal
    /// addresses" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/create-funding-address>
    pub async fn create_withdrawal_address(
        &self,
        address: &NewWithdrawalAddress,
        account_id: Option<&str>,
    ) -> Result<SavedAddress, KrakenError> {
        funding::addresses::create_withdrawal_address(&self.funding, address, account_id).await
    }

    /// Changes the description of a saved withdrawal address: whether it is verified. Requires
    /// the "Funds permissions - Add withdrawal addresses" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/update-funding-address>
    pub async fn update_withdrawal_address(
        &self,
        address_id: &str,
        description: &str,
        account_id: Option<&str>,
    ) -> Result<bool, KrakenError> {
        funding::addresses::update_withdrawal_address(
            &self.funding,
            address_id,
            description,
            account_id,
        )
        .await
    }

    /// Deletes a saved withdrawal address: whether it was deleted. Requires the "Funds
    /// permissions - Add withdrawal addresses" permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/delete-funding-address>
    pub async fn delete_withdrawal_address(
        &self,
        address_id: &str,
        account_id: Option<&str>,
    ) -> Result<bool, KrakenError> {
        funding::addresses::delete_withdrawal_address(&self.funding, address_id, account_id).await
    }

    /// A page of the withdrawals, most recent first. Requires the "Funds permissions - Query"
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/list-funding-withdrawals>
    pub async fn withdrawals(
        &self,
        request: &WithdrawalsRequest,
    ) -> Result<Withdrawals, KrakenError> {
        funding::withdrawals::withdrawals(&self.funding, request).await
    }

    /// Withdraws funds to a saved address. Requires the "Funds permissions - Withdraw"
    /// permission
    ///
    /// <https://docs.kraken.com/api/docs/rest-api/create-funding-withdrawal>
    pub async fn create_withdrawal(
        &self,
        withdrawal: &NewWithdrawal,
        account_id: Option<&str>,
    ) -> Result<CreatedWithdrawal, KrakenError> {
        funding::withdrawals::create_withdrawal(&self.funding, withdrawal, account_id).await
    }
}
