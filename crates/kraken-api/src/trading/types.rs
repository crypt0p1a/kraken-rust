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
    /// Fill or kill: entirely filled at once, or cancelled
    #[serde(alias = "fok")]
    Fok,
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

/// Price signal triggering the stop-loss and take-profit orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum Trigger {
    /// The last trade, the default
    Last,
    Index,
}

/// What happens when an order would trade against another order of the account
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "kebab-case")]
pub enum SelfTradePrevention {
    /// The arriving order is cancelled, the default
    CancelNewest,
    /// The resting order is cancelled
    CancelOldest,
    CancelBoth,
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
    fn encodes_the_trigger_and_the_self_trade_prevention() {
        assert_eq!(
            serde_json::to_string(&Trigger::Index).unwrap(),
            r#""index""#
        );
        for (prevention, text) in [
            (SelfTradePrevention::CancelNewest, r#""cancel-newest""#),
            (SelfTradePrevention::CancelOldest, r#""cancel-oldest""#),
            (SelfTradePrevention::CancelBoth, r#""cancel-both""#),
        ] {
            assert_eq!(serde_json::to_string(&prevention).unwrap(), text);
        }
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
            ("FOK", TimeInForce::Fok),
            ("fok", TimeInForce::Fok),
            ("GTX", TimeInForce::Unknown),
        ] {
            let decoded: TimeInForce = serde_json::from_str(&format!("\"{text}\"")).unwrap();
            assert_eq!(decoded, expected, "{text}");
        }
        assert_eq!(
            serde_json::to_string(&TimeInForce::Gtc).unwrap(),
            r#""GTC""#
        );
    }

    #[test]
    fn encodes_every_order_type_like_the_api() {
        for (order_type, text) in [
            (OrderType::Market, "market"),
            (OrderType::Limit, "limit"),
            (OrderType::Iceberg, "iceberg"),
            (OrderType::StopLoss, "stop-loss"),
            (OrderType::TakeProfit, "take-profit"),
            (OrderType::StopLossLimit, "stop-loss-limit"),
            (OrderType::TakeProfitLimit, "take-profit-limit"),
            (OrderType::TrailingStop, "trailing-stop"),
            (OrderType::TrailingStopLimit, "trailing-stop-limit"),
            (OrderType::SettlePosition, "settle-position"),
        ] {
            let json = format!("\"{text}\"");
            assert_eq!(serde_json::to_string(&order_type).unwrap(), json);
            assert_eq!(
                serde_json::from_str::<OrderType>(&json).unwrap(),
                order_type
            );
        }
        assert_eq!(
            serde_json::from_str::<OrderType>(r#""brand-new""#).unwrap(),
            OrderType::Unknown
        );
    }

    #[test]
    fn decodes_every_status_and_tells_the_final_ones() {
        for (text, status, is_final) in [
            ("pending", OrderStatus::Pending, false),
            ("open", OrderStatus::Open, false),
            ("closed", OrderStatus::Closed, true),
            ("canceled", OrderStatus::Canceled, true),
            ("expired", OrderStatus::Expired, true),
            ("unknown-status", OrderStatus::Unknown, false),
        ] {
            let decoded: OrderStatus = serde_json::from_str(&format!("\"{text}\"")).unwrap();
            assert_eq!(decoded, status, "{text}");
            assert_eq!(decoded.is_final(), is_final, "{text}");
        }
    }

    #[test]
    fn decodes_both_sides_only() {
        assert_eq!(
            serde_json::from_str::<OrderSide>(r#""buy""#).unwrap(),
            OrderSide::Buy
        );
        assert_eq!(
            serde_json::from_str::<OrderSide>(r#""sell""#).unwrap(),
            OrderSide::Sell
        );
        assert!(serde_json::from_str::<OrderSide>(r#""BUY""#).is_err());
    }

    #[test]
    fn names_the_flags_like_their_serialization() {
        for flag in [
            OrderFlag::Post,
            OrderFlag::Fcib,
            OrderFlag::Fciq,
            OrderFlag::Nompp,
            OrderFlag::Viqc,
        ] {
            assert_eq!(
                serde_json::to_string(&flag).unwrap(),
                format!("\"{}\"", flag.as_ref())
            );
        }
    }
}
