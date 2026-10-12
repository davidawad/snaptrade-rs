//! Brokerage connections (authorizations).

use super::datetime::opt_datetime;
use super::{ConnectionId, ConnectionType, Extra};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A supported brokerage.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Brokerage {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub allows_trading: Option<bool>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A connection between a user and a brokerage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Connection {
    pub id: ConnectionId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub brokerage: Option<Brokerage>,
    /// `true` when the connection needs to be re-authorised.
    #[serde(default)]
    pub disabled: Option<bool>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub disabled_date: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub created_date: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub updated_date: Option<DateTime<Utc>>,
    #[serde(default, rename = "type")]
    pub connection_type: Option<ConnectionType>,
    #[serde(flatten)]
    pub extra: Extra,
}
