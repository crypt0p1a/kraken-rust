//! Private Earn: the strategies earning rewards on an asset (staking, off-chain yield), the
//! allocations of the account and their (de)allocation
//!
//! The (de)allocations are asynchronous: their status is followed with `allocate_status` and
//! `deallocate_status`.

use crate::{KrakenError, lenient};
use kraken_network::PrivateClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};

/// How the funds are locked by a strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum LockType {
    /// Not locked
    Flex,
    /// Locked during a bonding and an unbonding period
    Bonded {
        /// Seconds
        bonding_period: Option<i64>,
        bonding_period_variable: Option<bool>,
        /// Rewards earned during the bonding, paid after it
        bonding_rewards: Option<bool>,
        /// Seconds in the exit queue before the unbonding, when greater than 0
        exit_queue_period: Option<i64>,
        /// Seconds between the payouts of the rewards
        payout_frequency: Option<i64>,
        /// Seconds to wait to remove the funds
        unbonding_period: Option<i64>,
        unbonding_period_variable: Option<bool>,
        /// Rewards earned and paid during the unbonding
        unbonding_rewards: Option<bool>,
    },
    /// Removed at once
    Instant {
        /// Seconds between the payouts of the rewards
        payout_frequency: Option<i64>,
    },
    /// Sent by Kraken but unknown to this version of the library
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum AutoCompound {
    Disabled,
    Enabled,
    /// Chosen by the user
    Optional {
        /// Enabled by default
        default: Option<bool>,
    },
    Unknown,
}

/// Where the rewards come from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum YieldSource {
    Staking,
    OffChain,
    Unknown,
}

/// The objects tagged by `type` are read as flat objects, then mapped to their enum: serde's
/// internally tagged enums buffer their content, out of the reach of the strict decoding of the
/// tests
#[derive(Deserialize)]
struct RawLockType {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    bonding_period: Option<i64>,
    #[serde(default)]
    bonding_period_variable: Option<bool>,
    #[serde(default)]
    bonding_rewards: Option<bool>,
    #[serde(default)]
    exit_queue_period: Option<i64>,
    #[serde(default)]
    payout_frequency: Option<i64>,
    #[serde(default)]
    unbonding_period: Option<i64>,
    #[serde(default)]
    unbonding_period_variable: Option<bool>,
    #[serde(default)]
    unbonding_rewards: Option<bool>,
}

impl<'de> Deserialize<'de> for LockType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawLockType::deserialize(deserializer)?;
        Ok(match raw.kind.as_str() {
            "flex" => LockType::Flex,
            "bonded" => LockType::Bonded {
                bonding_period: raw.bonding_period,
                bonding_period_variable: raw.bonding_period_variable,
                bonding_rewards: raw.bonding_rewards,
                exit_queue_period: raw.exit_queue_period,
                payout_frequency: raw.payout_frequency,
                unbonding_period: raw.unbonding_period,
                unbonding_period_variable: raw.unbonding_period_variable,
                unbonding_rewards: raw.unbonding_rewards,
            },
            "instant" => LockType::Instant {
                payout_frequency: raw.payout_frequency,
            },
            _ => LockType::Unknown,
        })
    }
}

#[derive(Deserialize)]
struct RawAutoCompound {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    default: Option<bool>,
}

impl<'de> Deserialize<'de> for AutoCompound {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawAutoCompound::deserialize(deserializer)?;
        Ok(match raw.kind.as_str() {
            "disabled" => AutoCompound::Disabled,
            "enabled" => AutoCompound::Enabled,
            "optional" => AutoCompound::Optional {
                default: raw.default,
            },
            _ => AutoCompound::Unknown,
        })
    }
}

#[derive(Deserialize)]
struct RawYieldSource {
    #[serde(rename = "type")]
    kind: String,
}

impl<'de> Deserialize<'de> for YieldSource {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match RawYieldSource::deserialize(deserializer)?.kind.as_str() {
                "staking" => YieldSource::Staking,
                "off_chain" => YieldSource::OffChain,
                _ => YieldSource::Unknown,
            },
        )
    }
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

/// An amount in the asset of the allocation, and in the requested `converted_asset`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EarnAmount {
    #[serde(deserialize_with = "lenient::decimal")]
    pub native: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub converted: Decimal,
}

/// An allocation moving to its next state
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TimedAllocation {
    #[serde(deserialize_with = "lenient::decimal")]
    pub native: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub converted: Decimal,
    /// RFC 3339: when the (de)allocation was received
    pub created_at: String,
    /// RFC 3339: when it moves to its next state
    pub expires: String,
}

/// The allocations in a state (bonding, exit queue, unbonding)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AllocationState {
    #[serde(deserialize_with = "lenient::decimal")]
    pub native: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub converted: Decimal,
    #[serde(deserialize_with = "lenient::number")]
    pub allocation_count: i64,
    #[serde(default)]
    pub allocations: Vec<TimedAllocation>,
}

/// The amounts allocated to a strategy, by state
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AllocatedAmounts {
    pub total: EarnAmount,
    /// Pending (de)allocation: negative for a deallocation
    #[serde(default)]
    pub pending: Option<EarnAmount>,
    #[serde(default)]
    pub bonding: Option<AllocationState>,
    #[serde(default)]
    pub exit_queue: Option<AllocationState>,
    #[serde(default)]
    pub unbonding: Option<AllocationState>,
}

/// The current payout period
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EarnPayout {
    /// Earned since the start of the period
    pub accumulated_reward: EarnAmount,
    /// Expected until the payout
    pub estimated_reward: EarnAmount,
    /// RFC 3339
    pub period_start: String,
    /// RFC 3339: the tentative date of the payout
    pub period_end: String,
}

/// The funds of the account in a strategy
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EarnAllocation {
    pub strategy_id: String,
    /// The asset of the native amounts
    pub native_asset: String,
    pub amount_allocated: AllocatedAmounts,
    /// Earned over the whole life of the account
    pub total_rewarded: EarnAmount,
    /// None outside of a payout period
    #[serde(default)]
    pub payout: Option<EarnPayout>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-allocations>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EarnAllocations {
    /// The asset of the converted amounts, e.g. "USD"
    pub converted_asset: String,
    /// In `converted_asset`
    #[serde(deserialize_with = "lenient::decimal")]
    pub total_allocated: Decimal,
    /// Earned over the whole life of the account, in `converted_asset`
    #[serde(deserialize_with = "lenient::decimal")]
    pub total_rewarded: Decimal,
    #[serde(default)]
    pub items: Vec<EarnAllocation>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-allocations>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EarnAllocationsRequest {
    /// Descending when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub ascending: Option<bool>,
    /// The asset of the converted amounts: USD when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub converted_asset: Option<String>,
    /// Omits the strategies used in the past which hold nothing now
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub hide_zero_allocations: Option<bool>,
}

/// Requires the "Query Funds" permission
pub(crate) async fn allocations(
    client: &PrivateClient,
    request: &EarnAllocationsRequest,
) -> Result<EarnAllocations, KrakenError> {
    client.call_with("Earn/Allocations", Some(request)).await
}

/// Funds to (de)allocate
///
/// <https://docs.kraken.com/api/docs/rest-api/allocate-strategy>
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EarnFunds {
    /// The id of an Earn strategy
    pub strategy_id: String,
    /// In the asset of the strategy, a string to keep the precision of the caller
    pub amount: String,
}

/// Starts allocating funds to a strategy: asynchronous, followed by `allocate_status`. Requires
/// the "Earn Funds" permission
pub(crate) async fn allocate(
    client: &PrivateClient,
    funds: &EarnFunds,
) -> Result<bool, KrakenError> {
    client.call_with("Earn/Allocate", Some(funds)).await
}

/// Starts deallocating funds from a strategy: asynchronous, followed by `deallocate_status`.
/// Requires the "Earn Funds" permission
pub(crate) async fn deallocate(
    client: &PrivateClient,
    funds: &EarnFunds,
) -> Result<bool, KrakenError> {
    client.call_with("Earn/Deallocate", Some(funds)).await
}

#[derive(Serialize)]
struct OfStrategy<'a> {
    strategy_id: &'a str,
}

#[derive(Deserialize)]
struct OperationStatus {
    pending: bool,
}

/// Whether the last allocation to the strategy is still in progress. Requires the "Earn Funds"
/// or "Query Funds" permission
pub(crate) async fn allocate_status(
    client: &PrivateClient,
    strategy_id: &str,
) -> Result<bool, KrakenError> {
    let status: OperationStatus = client
        .call_with("Earn/AllocateStatus", Some(&OfStrategy { strategy_id }))
        .await?;
    Ok(status.pending)
}

/// Whether the last deallocation from the strategy is still in progress. Requires the "Earn
/// Funds" or "Query Funds" permission
pub(crate) async fn deallocate_status(
    client: &PrivateClient,
    strategy_id: &str,
) -> Result<bool, KrakenError> {
    let status: OperationStatus = client
        .call_with("Earn/DeallocateStatus", Some(&OfStrategy { strategy_id }))
        .await?;
    Ok(status.pending)
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
    fn encodes_the_allocations_request() {
        let request = EarnAllocationsRequest {
            ascending: Some(false),
            converted_asset: Some("EUR".into()),
            hide_zero_allocations: Some(true),
        };
        assert_eq!(
            kraken_network::params::encode(&request).unwrap(),
            "ascending=false&converted_asset=EUR&hide_zero_allocations=true"
        );
    }

    #[test]
    fn decodes_the_allocations_by_state() {
        let allocations: EarnAllocations = serde_json::from_str(
            r#"{"converted_asset":"USD","total_allocated":"49.2398","total_rewarded":"0.0675",
            "items":[{"strategy_id":"ESDQCOL-WTZEU-NU55QF","native_asset":"ETH",
                "amount_allocated":{
                    "bonding":{"native":"0.0210000000","converted":"39.0645","allocation_count":2,
                        "allocations":[{"created_at":"2023-07-06T10:52:05Z",
                            "expires":"2023-08-19T02:34:05.807Z","native":"0.0010000000",
                            "converted":"1.8602"}]},
                    "pending":{"native":"-0.0010000000","converted":"-1.8602"},
                    "total":{"native":"0.0210000000","converted":"39.0645"}},
                "total_rewarded":{"native":"0","converted":"0.0000"},
                "payout":{"period_start":"2023-07-06T00:00:00Z","period_end":"2023-07-13T00:00:00Z",
                    "accumulated_reward":{"native":"0.0001","converted":"0.1860"},
                    "estimated_reward":{"native":"0.0003","converted":"0.5581"}}}]}"#,
        )
        .unwrap();
        let eth = &allocations.items[0];
        let bonding = eth.amount_allocated.bonding.as_ref().unwrap();
        assert_eq!(bonding.allocation_count, 2);
        assert_eq!(bonding.allocations[0].native, dec!(0.001));
        // a pending deallocation is negative
        assert_eq!(eth.amount_allocated.pending.unwrap().native, dec!(-0.001));
        assert_eq!(eth.amount_allocated.unbonding, None);
        assert_eq!(
            eth.payout.as_ref().unwrap().estimated_reward.converted,
            dec!(0.5581)
        );
        assert_eq!(allocations.total_allocated, dec!(49.2398));
    }

    #[test]
    fn encodes_the_funds() {
        let funds = EarnFunds {
            strategy_id: "ESRFUO3-Q62XD-WIOIL7".into(),
            amount: "4.3".into(),
        };
        assert_eq!(
            kraken_network::params::encode(&funds).unwrap(),
            "strategy_id=ESRFUO3-Q62XD-WIOIL7&amount=4.3"
        );
    }

    #[test]
    fn decodes_the_status_of_an_operation() {
        let status: OperationStatus = serde_json::from_str(r#"{"pending":true}"#).unwrap();
        assert!(status.pending);
        assert_eq!(
            kraken_network::params::encode(&OfStrategy {
                strategy_id: "ESRFUO3-Q62XD-WIOIL7"
            })
            .unwrap(),
            "strategy_id=ESRFUO3-Q62XD-WIOIL7"
        );
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
