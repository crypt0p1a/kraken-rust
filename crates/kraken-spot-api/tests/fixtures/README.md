# Fixtures

Answers of the Kraken REST API, decoded strictly by `tests/fixtures.rs` (no field ignored by the
models, no required field missing):

- `time.json`, `ticker.json`, `asset_pairs.json`, `ohlc.json`: recorded from the public API,
  reduced to a few representative pairs (online / cancel_only / post_only, with and without
  margin, a `:BTNL` ticker, a ticker without bid)
- `system_status.json`, `maintenance_schedule.json` (nothing scheduled: `{}`), `assets.json`
  (currencies and tokenized assets, every optional field, a `withdrawal_only` asset),
  `depth.json`, `grouped_book.json`, `trades.json`, `spread.json`, `pre_trade.json`,
  `post_trade.json`: recorded from the public API, reduced to a few entries
- `level3.json`: the example of the documentation of Kraken
- `balance_ex.json`, `trade_volume.json`, `open_orders.json`, `closed_orders.json`,
  `query_orders.json`, `add_order_validate.json`, `add_order.json`: shapes of the real private
  answers with **synthetic values**: no account data is recorded here
- `balance.json`, `credit_lines.json`, `trade_balance.json`, `api_key_info.json`,
  `wallet_accounts.json`, `trades_history.json`, `query_trades.json`, `order_amends.json`,
  `open_positions.json`, `ledgers.json`, `query_ledgers.json`, `add_export.json`,
  `export_status.json`, `remove_export.json`: the documented shapes of the private answers,
  synthetic values; `open_positions.json` and `export_status.json` extend the examples of the
  documentation. Their live contracts, run with the credentials of the CI, check them against
  the real answers (Ledgers and the exports only when the key has their permission)

- `websockets_token.json`, `cancel_order.json`, `cancel_all_orders_after.json`,
  `cancel_order_batch.json`, `amend_order.json`, `edit_order.json`, `add_order_batch.json`: the
  documented shapes of the trading answers, synthetic values. The live contracts never change
  the orders of the account: they validate the new orders (`validate=true`), and cancel, amend
  or edit orders which don't exist; CancelAll and CancelAllOrdersAfter have none

- `open_strategies.json`, `query_strategies.json`, `closed_strategies.json`, `add_strategy.json`
  (algo), `earn_*.json` (Earn), `create_subaccount.json`, `account_transfer.json`
  (subaccounts): the examples of the documentation, extended, synthetic values
- the funding answers (`funding_*.json`, `deposit_limits.json`, `withdrawal_limits.json`,
  `claim_deposit_address.json`, `claimed_addresses.json`, `deposits.json`,
  `withdrawal_addresses.json`, `create_withdrawal_address.json`, `withdrawals.json`,
  `create_withdrawal.json`): plain JSON, without the `{"error", "result"}` envelope of the
  other endpoints, read by `strict::funding_fixture`

Update them when Kraken changes its answers, which the live contract tests detect.
