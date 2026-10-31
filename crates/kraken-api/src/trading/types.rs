use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderType {
    Market,
    Limit,
    Iceberg,
    StopLoss,
    TakeProfit,
    StopLossLimit,
    TakeProfitLimit,
    TrailingStop,
    TrailingStopLimit,
    SettlePosition,
    /// Sent by Kraken but unknown to this version of the library
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Pending,
    Open,
    Closed,
    Canceled,
    Expired,
    #[serde(other)]
    Unknown,
}

impl OrderStatus {
    /// The order won't change anymore
    pub fn is_final(self) -> bool {
        matches!(
            self,
            OrderStatus::Closed | OrderStatus::Canceled | OrderStatus::Expired
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    /// Good 'til cancelled, the default
    Gtc,
    /// Immediate or cancel
    Ioc,
    /// Good 'til date: requires `expire_time`
    Gtd,
}

/// Flags of a new order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderFlag {
    /// Post-only: maker only, cancelled if it would match
    Post,
    /// Fee in the base currency
    Fcib,
    /// Fee in the quote currency
    Fciq,
    /// No market price protection
    Nompp,
    /// Volume in the quote currency (market buy orders only)
    Viqc,
}

impl AsRef<str> for OrderFlag {
    fn as_ref(&self) -> &str {
        match self {
            OrderFlag::Post => "post",
            OrderFlag::Fcib => "fcib",
            OrderFlag::Fciq => "fciq",
            OrderFlag::Nompp => "nompp",
            OrderFlag::Viqc => "viqc",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_the_enums_like_the_api() {
        assert_eq!(
            serde_json::to_string(&OrderSide::Sell).unwrap(),
            r#""sell""#
        );
        assert_eq!(
            serde_json::to_string(&OrderType::TakeProfitLimit).unwrap(),
            r#""take-profit-limit""#
        );
        assert_eq!(
            serde_json::to_string(&TimeInForce::Gtd).unwrap(),
            r#""GTD""#
        );
        assert_eq!(
            serde_json::from_str::<OrderStatus>(r#""something-new""#).unwrap(),
            OrderStatus::Unknown
        );
    }
}
