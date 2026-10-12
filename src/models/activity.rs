//! Account activities (transaction history).

use super::datetime::opt_datetime;
use super::symbol::symbol_or_ticker;
use super::{ActivityId, ActivityType, Currency, Decimal, Extra, OptionSymbol, Symbol};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One activity: a trade, dividend, fee, transfer, ...
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Activity {
    #[serde(default)]
    pub id: Option<ActivityId>,
    #[serde(default, deserialize_with = "symbol_or_ticker")]
    pub symbol: Option<Symbol>,
    #[serde(default)]
    pub option_symbol: Option<OptionSymbol>,
    #[serde(default)]
    pub price: Option<Decimal>,
    #[serde(default)]
    pub units: Option<Decimal>,
    /// Net cash effect; negative for money out.
    #[serde(default)]
    pub amount: Option<Decimal>,
    #[serde(default)]
    pub fee: Option<Decimal>,
    #[serde(default)]
    pub fx_rate: Option<Decimal>,
    #[serde(default)]
    pub currency: Option<Currency>,
    #[serde(default, rename = "type")]
    pub activity_type: Option<ActivityType>,
    /// Option-specific action (e.g. `BUY_TO_OPEN`), free text.
    #[serde(default)]
    pub option_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub trade_date: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub settlement_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub institution: Option<String>,
    #[serde(default)]
    pub external_reference_id: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}
