//! Typed errors.

use std::time::Duration;

/// Errors returned by the client. Messages never contain credentials or the
/// request URL (which carries `clientId`).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP 410: the endpoint is not available for this account/plan. SnapTrade
    /// Personal keys get this for some endpoints (notably positions).
    #[error("{path}: HTTP 410: endpoint not available for your account or plan (Personal keys cannot use every endpoint): {body}")]
    NotAvailable { path: String, body: String },

    /// HTTP 429 that survived all retries.
    #[error("{path}: HTTP 429: rate limited (250 req/min per customer, 10 req/min per account){}", retry_hint(.retry_after))]
    RateLimited {
        path: String,
        retry_after: Option<Duration>,
    },

    /// HTTP 401 or 403: bad `clientId`/`consumerKey`, bad signature, or clock skew.
    #[error("{path}: HTTP {status}: authentication failed (check clientId/consumerKey, user secret and system clock): {body}")]
    Auth {
        path: String,
        status: u16,
        body: String,
    },

    /// HTTP 404.
    #[error("{path}: HTTP 404: not found: {body}")]
    NotFound { path: String, body: String },

    /// Any other non-2xx status.
    #[error("{path}: HTTP {status}: {body}")]
    Http {
        path: String,
        status: u16,
        body: String,
    },

    /// Transport-level failure (DNS, TLS, timeout, ...), after retries.
    #[error("{path}: network error: {message}")]
    Network { path: String, message: String },

    /// 2xx response that is not the JSON shape expected.
    #[error("{path}: invalid response: {message}")]
    Decode { path: String, message: String },

    /// Credentials could not be loaded from a provider.
    #[error("credentials: {0}")]
    Credentials(String),

    /// Caller error caught before sending (missing user auth, bad date, ...).
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

fn retry_hint(d: &Option<Duration>) -> String {
    match d {
        Some(d) => format!(" (Retry-After {}s)", d.as_secs()),
        None => String::new(),
    }
}

impl Error {
    /// HTTP status, when the error came from a response.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::NotAvailable { .. } => Some(410),
            Error::RateLimited { .. } => Some(429),
            Error::Auth { status, .. } | Error::Http { status, .. } => Some(*status),
            Error::NotFound { .. } => Some(404),
            _ => None,
        }
    }

    /// True for HTTP 410 (not available for the account/plan).
    pub fn is_not_available(&self) -> bool {
        matches!(self, Error::NotAvailable { .. })
    }

    /// True for HTTP 429.
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Error::RateLimited { .. })
    }
}

/// Map a non-2xx response to a typed error. `body` is truncated to 200 chars.
pub(crate) fn from_status(path: &str, status: u16, retry_after: Option<u64>, body: &str) -> Error {
    let body: String = body.chars().take(200).collect();
    let path = path.to_string();
    match status {
        410 => Error::NotAvailable { path, body },
        429 => Error::RateLimited {
            path,
            retry_after: retry_after.map(Duration::from_secs),
        },
        401 | 403 => Error::Auth { path, status, body },
        404 => Error::NotFound { path, body },
        _ => Error::Http { path, status, body },
    }
}

pub type Result<T> = std::result::Result<T, Error>;
