//! Typed response models.
//!
//! Design rules, so that a new SnapTrade field or a brokerage quirk never
//! breaks decoding:
//!
//! * Ids are newtypes ([`AccountId`], [`ConnectionId`], ...).
//! * Money, prices and quantities are [`Decimal`] (text-backed, exact).
//! * Timestamps are `chrono::DateTime<Utc>`; calendar dates are `NaiveDate`.
//! * Enumerations are `#[non_exhaustive]` and have an `Unknown(String)`
//!   variant that keeps the raw value.
//! * Almost every field is optional (brokerages differ); only the primary id
//!   of a record is required.
//! * Every struct carries an `extra` map with the fields this crate does not
//!   model yet, so nothing is lost. For the whole response, call
//!   [`Client::get`](crate::Client::get) or [`Client::get_as`](crate::Client::get_as).
//!
//! Models derive `Serialize` so they can be written back out (exports).
//! Their `Debug` output can contain account data: do not log it.

mod account;
mod activity;
mod connection;
mod datetime;
mod decimal;
mod enums;
mod ids;
mod position;
mod symbol;
#[cfg(feature = "trading")]
mod trading;
mod user;

pub use account::{Account, AccountBalanceSummary, Balance, SyncDetail, SyncStatus, TotalValue};
pub use activity::Activity;
pub use connection::{Brokerage, Connection};
pub use decimal::{Decimal, ParseDecimalError};
pub use enums::{ActivityType, ConnectionType, OptionType};
pub use ids::{
    AccountId, ActivityId, BrokerageOrderId, ConnectionId, SymbolId, TradeId, UserId, UserSecret,
};
pub use position::{AllPositions, OptionPosition, OptionPositionSymbol, Position, PositionSymbol};
pub use symbol::{Currency, Exchange, OptionSymbol, SecurityType, Symbol};
#[cfg(feature = "trading")]
pub use trading::{
    Order, OrderAction, OrderImpact, OrderStatus, OrderType, Quote, RecentOrders, TimeInForce,
    TradeImpact, TradePreview,
};
pub use user::{ApiStatus, LoginRedirect, UserRegistration};

/// Fields the crate does not model, keyed by their JSON name.
pub type Extra = std::collections::BTreeMap<String, serde_json::Value>;

#[cfg(feature = "trading")]
pub(crate) use enums::string_enum;
