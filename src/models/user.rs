//! Authentication and status responses.

use super::datetime::opt_datetime;
use super::{Extra, UserId, UserSecret};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `GET /`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ApiStatus {
    #[serde(default)]
    pub version: Option<i64>,
    #[serde(default, deserialize_with = "opt_datetime")]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub online: Option<bool>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Result of registering a user. Store the secret securely.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UserRegistration {
    #[serde(rename = "userId")]
    pub user_id: UserId,
    #[serde(rename = "userSecret")]
    pub user_secret: UserSecret,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Connection Portal redirect.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LoginRedirect {
    #[serde(default, rename = "redirectURI")]
    pub redirect_uri: Option<String>,
    #[serde(default, rename = "sessionId")]
    pub session_id: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}
