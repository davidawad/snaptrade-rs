//! Order and trading models. Only compiled with the `trading` feature.
//!
//! These types describe responses (and the enum values used in requests).
//! Constructing one does not send anything.

use super::datetime::opt_datetime;
use super::symbol::symbol_or_ticker;
use super::{string_enum, BrokerageOrderId, Decimal, Extra, OptionSymbol, Symbol, TradeId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

string_enum! {
    /// Order side.
    pub enum OrderAction {
        Buy => "BUY",
        Sell => "SELL",
        BuyToCover => "BUY_COVER",
        SellShort => "SELL_SHORT",
        BuyToOpen => "BUY_OPEN",
        BuyToClose => "BUY_CLOSE",
        SellToOpen => "SELL_OPEN",
        SellToClose => "SELL_CLOSE",
    }
}

string_enum! {
    /// Order type.
    pub enum OrderType {
        Limit => "Limit",
        Market => "Market",
        StopLimit => "StopLimit",
        Stop => "Stop",
    }
}

string_enum! {
    /// How long an order stays live.
    pub enum TimeInForce {
        Day => "Day",
        GoodTilCanceled => "GTC",
        FillOrKill => "FOK",
        ImmediateOrCancel => "IOC",
    }
}

string_enum! {
    /// Lifecycle state of an order.
    pub enum OrderStatus {
        None => "NONE",
        Pending => "PENDING",
        Accepted => "ACCEPTED",
        Failed => "FAILED",
        Rejected => "REJECTED",
        Canceled => "CANCELED",
        PartialCanceled => "PARTIAL_CANCELED",
        CancelPending => "CANCEL_PENDING",
        Executed => "EXECUTED",
        Partial => "PARTIAL",
        ReplacePending => "REPLACE_PENDING",
        Replaced => "REPLACED",
        Expired => "EXPIRED",
        Queued => "QUEUED",
        Triggered => "TRIGGERED",
        Activated => "ACTIVATED",
        PendingRiskReview => "PENDING_RISK_REVIEW",
        ContingentOrder => "CONTINGENT_ORDER",
    }
}

/// An order as the brokerage reports it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Order {
    #[serde(default)]
    pub brokerage_order_id: Option<BrokerageOrderId>,
    #[serde(default)]
    pub status: Option<OrderStatus>,
    #[serde(default, deserialize_with = "symbol_or_ticker")]
    pub universal_symbol: Option<Symbol>,
    #[serde(default)]
    pub option_symbol: Option<OptionSymbol>,
    #[serde(default)]
    pub action: Option<OrderAction>,
    #[serde(default)]
    pub total_quantity: Option<Decimal>,
    #[serde(default)]
    pub open_quantity: Option<Decimal>,
    #[serde(default)]
    pub canceled_quantity: Option<Decimal>,
    #[serde(default)]
    pub filled_quantity: Option<Decimal>,
    #[serde(default)]
    pub execution_price: Option<Decimal>,
    #[serde(default)]
    pub limit_price: Option<Decimal>,
    #[serde(default)]
    pub stop_price: Option<Decimal>,
    #[serde(default)]
    pub order_type: Option<OrderType>,
    #[serde(default)]
    pub time_in_force: Option<TimeInForce>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub time_placed: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub time_updated: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub time_executed: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub expiry_date: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `recentOrders` response.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecentOrders {
    #[serde(default)]
    pub orders: Vec<Order>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A symbol quote.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Quote {
    #[serde(default, deserialize_with = "symbol_or_ticker")]
    pub symbol: Option<Symbol>,
    #[serde(default)]
    pub bid_price: Option<Decimal>,
    #[serde(default)]
    pub ask_price: Option<Decimal>,
    #[serde(default)]
    pub last_trade_price: Option<Decimal>,
    #[serde(default)]
    pub bid_size: Option<Decimal>,
    #[serde(default)]
    pub ask_size: Option<Decimal>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// The checked trade inside an [`OrderImpact`]; its `id` feeds
/// `Client::place_checked_order`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TradePreview {
    #[serde(default)]
    pub id: Option<TradeId>,
    #[serde(default, deserialize_with = "symbol_or_ticker")]
    pub universal_symbol: Option<Symbol>,
    #[serde(default)]
    pub action: Option<OrderAction>,
    #[serde(default)]
    pub order_type: Option<OrderType>,
    #[serde(default)]
    pub time_in_force: Option<TimeInForce>,
    #[serde(default)]
    pub price: Option<Decimal>,
    #[serde(default)]
    pub units: Option<Decimal>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Estimated effect of a trade on an account.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TradeImpact {
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub remaining_cash: Option<Decimal>,
    #[serde(default)]
    pub estimated_commission: Option<Decimal>,
    #[serde(default)]
    pub forex_fees: Option<Decimal>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Response of an order-impact check.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OrderImpact {
    #[serde(default)]
    pub trade: Option<TradePreview>,
    #[serde(default)]
    pub trade_impacts: Vec<TradeImpact>,
    #[serde(flatten)]
    pub extra: Extra,
}
