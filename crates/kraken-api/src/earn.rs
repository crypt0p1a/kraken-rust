//! Private Earn: the strategies earning rewards on an asset (staking, off-chain yield), the
//! allocations of the account and their (de)allocation
//!
//! The (de)allocations are asynchronous: their status is followed with `allocate_status` and
//! `deallocate_status`.

use crate::{KrakenError, lenient};
use kraken_network::PrivateClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// How the funds are locked by a strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LockType {
    /// Not locked
    Flex,
    /// Locked during a bonding and an unbonding period
    Bonded {
        /// Seconds
        #[serde(default)]
        bonding_period: Option<i64>,
        #[serde(default)]
        bonding_period_variable: Option<bool>,
        /// Rewards earned during the bonding, paid after it
        #[serde(default)]
        bonding_rewards: Option<bool>,
        /// Seconds in the exit queue before the unbonding, when greater than 0
        #[serde(default)]
        exit_queue_period: Option<i64>,
        /// Seconds between the payouts of the rewards
        #[serde(default)]
        payout_frequency: Option<i64>,
        /// Seconds to wait to remove the funds
        #[serde(default)]
        unbonding_period: Option<i64>,
        #[serde(default)]
        unbonding_period_variable: Option<bool>,
        /// Rewards earned and paid during the unbonding
        #[serde(default)]
        unbonding_rewards: Option<bool>,
    },
    /// Removed at once
    Instant {
        /// Seconds between the payouts of the rewards
        #[serde(default)]
        payout_frequency: Option<i64>,
    },
    /// Sent by Kraken but unknown to this version of the library
    #[serde(other)]
    Unknown,
}

/// The lock types the strategies can be filtered by
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum LockTypeFilter {
    Flex,
    Bonded,
    Timed,
    Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AutoCompound {
    Disabled,
    Enabled,
    /// Chosen by the user
    Optional {
        /// Enabled by default
        #[serde(default)]
        default: Option<bool>,
    },
    #[serde(other)]
    Unknown,
}

/// Where the rewards come from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum YieldSource {
    Staking,
    OffChain,
    #[serde(other)]
    Unknown,
}

/// Estimate of the yearly yield, from the past rewards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AprEstimate {
    /// Percentage
    #[serde(deserialize_with = "lenient::decimal")]
    pub low: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub high: Decimal,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-strategies>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EarnStrategy {
    /// e.g. "ESRFUO3-Q62XD-WIOIL7"
    pub id: String,
    /// The asset invested
    pub asset: String,
    pub lock_type: LockType,
    #[serde(default)]
    pub yield_source: Option<YieldSource>,
    #[serde(default)]
    pub auto_compound: Option<AutoCompound>,
    #[serde(default)]
    pub apr_estimate: Option<AprEstimate>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub allocation_fee: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub deallocation_fee: Option<Decimal>,
    #[serde(default)]
    pub can_allocate: bool,
    #[serde(default)]
    pub can_deallocate: bool,
    /// Why the account can't allocate, e.g. "tier"
    #[serde(default)]
    pub allocation_restriction_info: Vec<String>,
    /// Maximum allocation of a user: none when missing
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub user_cap: Option<Decimal>,
    /// Minimum (de)allocation, in USD: none when missing
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub user_min_allocation: Option<Decimal>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-strategies>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EarnStrategiesRequest {
    /// Descending when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub ascending: Option<bool>,
    /// Only the strategies of this asset
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset: Option<String>,
    /// The `next_cursor` of the previous page
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cursor: Option<String>,
    /// Strategies per page, possibly capped by Kraken
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub limit: Option<u16>,
    /// Only these lock types: all of them when empty
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "uniffi", uniffi(default = []))]
    pub lock_type: Vec<LockTypeFilter>,
}

/// A page of the strategies available to the account
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EarnStrategies {
    #[serde(default)]
    pub items: Vec<EarnStrategy>,
    /// To give as `cursor` for the next page: None on the last one
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// Sent in JSON: `lock_type` is a list. Requires a valid key, no permission
pub(crate) async fn strategies(
    client: &PrivateClient,
    request: &EarnStrategiesRequest,
) -> Result<EarnStrategies, KrakenError> {
    client.call_json("Earn/Strategies", request).await
}

#[cfg(test)]
pub(crate) const STRATEGIES: &str = r#"{"items":[
    {"id":"ESRFUO3-Q62XD-WIOIL7","asset":"DOT","lock_type":{"type":"instant","payout_frequency":604800},
        "apr_estimate":{"low":"8.0000","high":"12.0000"},"user_min_allocation":"0.01",
        "allocation_fee":"0.0000","deallocation_fee":"0.0000",
        "auto_compound":{"type":"enabled"},"yield_source":{"type":"staking"},
        "can_allocate":true,"can_deallocate":true,"allocation_restriction_info":[]},
    {"id":"ESMWVX6-JAPVY-23L3CV","asset":"ETH","lock_type":{"type":"bonded","bonding_period":604800,
        "bonding_period_variable":false,"bonding_rewards":false,"exit_queue_period":0,
        "payout_frequency":604800,"unbonding_period":2419200,"unbonding_period_variable":false,
        "unbonding_rewards":true},
        "user_cap":"100","allocation_fee":0,"deallocation_fee":0.5,
        "auto_compound":{"type":"optional","default":true},"yield_source":{"type":"off_chain"},
        "can_allocate":false,"can_deallocate":true,"allocation_restriction_info":["tier"]},
    {"id":"ESDQCOL-WTZEU-NU55QF","asset":"USDC","lock_type":{"type":"flex"},
        "auto_compound":{"type":"disabled"},"yield_source":{"type":"staking"},
        "can_allocate":true,"can_deallocate":true}
],"next_cursor":"2"}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn encodes_the_request_in_json() {
        let request = EarnStrategiesRequest {
            ascending: Some(true),
            asset: Some("DOT".into()),
            cursor: Some("2".into()),
            limit: Some(10),
            lock_type: vec![LockTypeFilter::Bonded, LockTypeFilter::Flex],
        };
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            json!({"ascending": true, "asset": "DOT", "cursor": "2", "limit": 10,
                "lock_type": ["bonded", "flex"]})
        );
        assert_eq!(
            serde_json::to_value(EarnStrategiesRequest::default()).unwrap(),
            json!({})
        );
    }

    #[test]
    fn decodes_every_lock_type() {
        let page: EarnStrategies = serde_json::from_str(STRATEGIES).unwrap();
        assert_eq!(page.next_cursor.as_deref(), Some("2"));
        let [instant, bonded, flex] = &page.items[..] else {
            panic!("{page:?}")
        };
        assert_eq!(
            instant.lock_type,
            LockType::Instant {
                payout_frequency: Some(604800)
            }
        );
        assert!(matches!(
            bonded.lock_type,
            LockType::Bonded {
                unbonding_period: Some(2419200),
                unbonding_rewards: Some(true),
                ..
            }
        ));
        assert_eq!(flex.lock_type, LockType::Flex);
        assert_eq!(instant.apr_estimate.unwrap().high, dec!(12));
        // the fees, sent as strings or numbers
        assert_eq!(bonded.deallocation_fee, Some(dec!(0.5)));
        assert_eq!(bonded.allocation_fee, Some(Decimal::ZERO));
        assert_eq!(
            bonded.auto_compound,
            Some(AutoCompound::Optional {
                default: Some(true)
            })
        );
        assert_eq!(bonded.yield_source, Some(YieldSource::OffChain));
        assert_eq!(bonded.allocation_restriction_info, ["tier"]);
        assert_eq!(flex.user_cap, None);
    }

    #[test]
    fn keeps_the_unknown_types() {
        let strategy: EarnStrategy = serde_json::from_str(
            r#"{"id":"E","asset":"SOL","lock_type":{"type":"timed","duration":86400},
            "auto_compound":{"type":"manual"},"yield_source":{"type":"defi"}}"#,
        )
        .unwrap();
        assert_eq!(strategy.lock_type, LockType::Unknown);
        assert_eq!(strategy.auto_compound, Some(AutoCompound::Unknown));
        assert_eq!(strategy.yield_source, Some(YieldSource::Unknown));
    }
}
