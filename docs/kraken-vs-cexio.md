# Kraken compared to CEX.IO

This client is the Kraken counterpart of the Rust crate of
[kotlin-cex-spot-trading](https://github.com/crypt0p1a/kotlin-cex-spot-trading) (CEX.IO), so that
both providers can be merged later.

## Endpoints

| `cex-trading-bot` (CEX.IO) | Kraken         | `PublicApi` / `PrivateApi`            |
|----------------------------|----------------|---------------------------------------|
| `tickers`                  | `Ticker`       | `tickers`                             |
| `pairsInfo`                | `AssetPairs`   | `asset_pairs`                         |
| `candles`                  | `OHLC`         | `candles`                             |
| `getMyCurrentFee`          | `TradeVolume`  | `trade_volume`                        |
| `getMyAccountStatus`       | `BalanceEx`    | `extended_balance`                    |
| `orders` (open)            | `OpenOrders`   | `open_orders`, `open_orders_for_pair` |
| `orders` (archived)        | `ClosedOrders` | `closed_orders`                       |
| `orders` (by id)           | `QueryOrders`  | `query_orders`, `order`               |
| `newOrder`                 | `AddOrder`     | `new_order`                           |
| `serverTime` (tests)       | `Time`         | `server_time`                         |

## Differences to handle when merging the providers

- **Pairs**: Kraken takes an alternate name (`XBTUSD`) and answers with its own name (`XXBTZUSD`),
  CEX.IO uses `BTC-USD` everywhere. `AssetPair.altname` / `wsname` give the mapping; BTC is `XBT`.
- **Accounts**: no Spot sub-accounts on Kraken, the balances are the ones of the API key.
  `ExtendedBalance::available()` is the equivalent of `balance - balanceOnHold`.
- **Client order ids**: `cl_ord_id` is a UUID or at most 18 ASCII characters (CEX.IO accepts
  longer `clientOrderId`s). `userref` is a numeric alternative.
- **Orders**: the answers are keyed by txid, returned here as lists with `Order.txid` filled.
  Closed orders are paginated by 50 (`offset`) and filtered by seconds, not milliseconds.
- **Units**: Kraken timestamps are in seconds (with decimals for the orders), CEX.IO in
  milliseconds. Fees are percents (0.26 = 0.26%) on both.
- **Amounts**: exact `Decimal`s here, `f64` in `cex-spot-trading`: to align on the decimals when
  merging.
- **Candles**: `since` instead of from/to, 720 candles at most, the last one is still open
  (`is_closed = false`).
- **Ticker**: values of today and of the last 24h, no `volumeUSD` / `priceChangePercentage`
  (computable from `open` and `last_trade.price`).
- **Rate limit**: the default `RateLimiter` follows the Starter tier (15 calls, +1 every 3s).
  The order calls have their own, separate limit on Kraken.
- **Nonce**: strictly increasing per API key, created inside the rate limiter queue so that the
  calls leave in order. Don't share an API key between processes.
