//! Market data: public, except the level 3 order book
//!
//! Each submodule holds the models of an endpoint and the call itself, the facades being
//! [`crate::PublicApi`] and [`crate::PrivateApi`].

pub(crate) mod assets;
pub(crate) mod ohlc;
pub(crate) mod pairs;
pub(crate) mod status;
pub(crate) mod ticker;
pub(crate) mod time;

pub use assets::{Asset, AssetClass, AssetStatus, AssetsRequest};
pub use ohlc::{Candle, CandleInterval, Candles, CandlesRequest};
pub use pairs::{AssetPair, AssetPairStatus, FeeTier};
pub use status::{
    Incident, IncidentImpact, IncidentStatus, IncidentStep, IncidentStepType, MaintenanceEvent,
    MaintenancePhase, OrderSubmission, RecommendedAction, SystemState, SystemStatus,
};
pub use ticker::{PriceLevel, Ticker, TradeLevel, Window, WindowCount};
pub use time::ServerTime;
