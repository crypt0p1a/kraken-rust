//! Private trading: orders of the account, placing new ones
//!
//! - `types`: enums shared by the requests and the answers
//! - `order`: the order model returned by the queries
//! - `query`: open, closed and queried orders
//! - `add`: new orders
//! - `trades`: the trades (fills) of the account
//! - `amends`: the versions of an amended order
//! - `cancel`: the cancellations of open orders
//! - `positions`: the open margin positions
//! - `websocket`: the token of the private WebSocket feeds

pub(crate) mod add;
pub(crate) mod amends;
pub(crate) mod cancel;
pub(crate) mod order;
pub(crate) mod positions;
pub(crate) mod query;
pub(crate) mod trades;
mod types;
pub(crate) mod websocket;

pub use add::{NewOrder, NewOrderAnswer, NewOrderDescription};
pub use amends::{AmendOrder, AmendType, OrderAmend, OrderAmends, OrderAmendsRequest};
pub use cancel::{CancelAllAfter, CancelledOrders, OrderReference};
pub use order::{Order, OrderDescription};
pub use positions::{ConsolidatedPosition, OpenPositionsRequest, Position};
pub use query::{
    CloseTime, ClosedOrders, ClosedOrdersRequest, OpenOrdersRequest, QueryOrdersRequest,
};
pub use trades::{
    QueryTradesRequest, Trade, TradeHistoryType, TradesHistory, TradesHistoryRequest,
};
pub use types::{OrderFlag, OrderSide, OrderStatus, OrderType, TimeInForce};
pub use websocket::WebSocketsToken;
