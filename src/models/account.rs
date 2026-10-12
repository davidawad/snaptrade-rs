//! Accounts and balances.

use super::datetime::{opt_date, opt_datetime};
use super::{AccountId, ConnectionId, Currency, Decimal, Extra};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Sync state of one data type (transactions or holdings).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SyncDetail {
    #[serde(default)]
    pub initial_sync_completed: Option<bool>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub last_successful_sync: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "opt_date")]
    pub first_transaction_date: Option<NaiveDate>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SyncStatus {
    #[serde(default)]
    pub transactions: Option<SyncDetail>,
    #[serde(default)]
    pub holdings: Option<SyncDetail>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// An amount in a currency given by code (`{"amount":1.0,"currency":"USD"}`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TotalValue {
    #[serde(default)]
    pub amount: Option<Decimal>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AccountBalanceSummary {
    #[serde(default)]
    pub total: Option<TotalValue>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A brokerage account. `number` is sensitive: use [`crate::mask::mask_number`]
/// before showing it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Account {
    pub id: AccountId,
    #[serde(default)]
    pub brokerage_authorization: Option<ConnectionId>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub number: Option<String>,
    #[serde(default)]
    pub institution_name: Option<String>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub created_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub sync_status: Option<SyncStatus>,
    #[serde(default)]
    pub balance: Option<AccountBalanceSummary>,
    /// Brokerage-reported status (free text, e.g. `ACTIVE`).
    #[serde(default)]
    pub status: Option<String>,
    /// Brokerage-reported account type (free text).
    #[serde(default)]
    pub raw_type: Option<String>,
    #[serde(default)]
    pub is_paper: Option<bool>,
    #[serde(default)]
    pub meta: Option<Extra>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Cash balance in one currency.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Balance {
    #[serde(default)]
    pub currency: Option<Currency>,
    #[serde(default)]
    pub cash: Option<Decimal>,
    #[serde(default)]
    pub buying_power: Option<Decimal>,
    #[serde(flatten)]
    pub extra: Extra,
}
