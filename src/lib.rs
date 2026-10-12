//! Unofficial Rust client for the [SnapTrade](https://snaptrade.com) API.
//!
//! * HMAC-SHA256 signed requests ([`sign`]), fresh timestamp per attempt.
//! * Pagination, `Retry-After`-aware exponential backoff on 429/5xx.
//! * Typed [`Error`]s, including HTTP 410 ([`Error::NotAvailable`]).
//! * Pluggable secrets via [`credentials::CredentialProvider`]: environment,
//!   external command, file, or your own.
//! * Mockable [`transport::Transport`]; tests need no network.
//!
//! Order and trading endpoints are compiled only with the **`trading`** cargo
//! feature, which is off by default. The default build cannot place, replace,
//! cancel or even list orders.
//!
//! ```no_run
//! use snaptrade::{credentials::EnvProvider, Client};
//!
//! # fn main() -> Result<(), snaptrade::Error> {
//! // SNAPTRADE_CLIENT_ID and SNAPTRADE_CONSUMER_KEY in the environment.
//! let client = Client::from_provider(&EnvProvider::new())?;
//! for account in client.accounts()? {
//!     println!("{}", account["id"]);
//! }
//! # Ok(()) }
//! ```
//!
//! # Personal API keys
//!
//! A Personal key has one implicit user: do not attach a `userId`/`userSecret`
//! (leave [`Client::user`] unset). Limits: 250 requests/minute per customer and
//! 10 requests/minute per account. Some endpoints, notably positions, answer
//! HTTP 410 for Personal keys.

pub mod api;
pub mod client;
pub mod credentials;
pub mod error;
pub mod export;
pub mod mask;
pub mod select;
pub mod sign;
pub mod transport;

pub use api::{endpoints, ActivityQuery, Endpoint, LoginOptions};
pub use client::Client;
pub use credentials::{CredentialProvider, Credentials, UserAuth};
pub use error::{Error, Result};
