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

Update them when Kraken changes its answers, which the live contract tests detect.
