//! Private trading: orders of the account, placing new ones
//!
//! - `types`: enums shared by the requests and the answers
//! - `order`: the order model returned by the queries
//! - `query`: open, closed and queried orders
//! - `add`: new orders

pub(crate) mod add;
pub(crate) mod order;
pub(crate) mod query;
mod types;

pub use add::{NewOrder, NewOrderAnswer, NewOrderDescription};
pub use order::{Order, OrderDescription};
pub use query::{
    CloseTime, ClosedOrders, ClosedOrdersRequest, OpenOrdersRequest, QueryOrdersRequest,
};
pub use types::{OrderFlag, OrderSide, OrderStatus, OrderType, TimeInForce};
