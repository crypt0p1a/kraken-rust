use crate::account::{self, ExtendedBalance, TradeVolume};
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
}
