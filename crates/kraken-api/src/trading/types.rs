use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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

/// Sent in upper case, answered in lower case by ClosedOrders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    /// Good 'til cancelled, the default
    #[serde(alias = "gtc")]
    Gtc,
    /// Immediate or cancel
    #[serde(alias = "ioc")]
    Ioc,
    /// Good 'til date: requires `expire_time`
    #[serde(alias = "gtd")]
    Gtd,
    /// Sent by Kraken but unknown to this version of the library. Never send it
    #[serde(other)]
    Unknown,
}

/// Flags of a new order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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

    #[test]
    fn reads_the_time_in_force_in_both_cases() {
        // upper case in the requests, lower case in the answers of ClosedOrders
        for (text, expected) in [
            ("GTC", TimeInForce::Gtc),
            ("gtc", TimeInForce::Gtc),
            ("IOC", TimeInForce::Ioc),
            ("ioc", TimeInForce::Ioc),
            ("GTD", TimeInForce::Gtd),
            ("gtd", TimeInForce::Gtd),
            ("FOK", TimeInForce::Unknown),
        ] {
            let decoded: TimeInForce = serde_json::from_str(&format!("\"{text}\"")).unwrap();
            assert_eq!(decoded, expected, "{text}");
        }
        assert_eq!(
            serde_json::to_string(&TimeInForce::Gtc).unwrap(),
            r#""GTC""#
        );
    }
}
