# Changelog

All notable changes are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[SemVer](https://semver.org/).

## [Unreleased]

### Changed
- Renamed the package to `snaptrade-rs` (library `snaptrade_rs`, binary
  `snaptrade-rs`). Repository: https://github.com/davidawad/snaptrade-rs.
- Responses are now typed (`snaptrade_rs::models`) instead of
  `serde_json::Value`: newtype ids, text-backed `Decimal`, `chrono` dates,
  `#[non_exhaustive]` enums with `Unknown`, and an `extra` map on every model.
  Raw access remains via `Client::get` / `Client::get_as`.
- Methods take `impl Into<AccountId>` / `ConnectionId` / `UserId` /
  `BrokerageOrderId` / `TradeId` instead of `&str`.
- `trading`: `EquityOrder` uses `Decimal` and typed enums; order responses are
  typed (`Order`, `RecentOrders`, `Quote`, `OrderImpact`).
- Decode errors no longer echo response values.
- MSRV is Rust 1.85 (verified with the committed lockfile).

## [0.1.0]

Initial release.

### Added
- Signed-request core: `base64(HMAC-SHA256(consumerKey, canonical JSON of
  {content, path, query}))`, canonical JSON implemented independently of
  `serde_json` map ordering; fresh timestamp and signature per attempt.
- Blocking `Client` over a mockable `Transport` (default: `reqwest` + rustls).
- Retry/backoff: HTTP 429 (all methods) and 5xx/network errors (idempotent
  methods only), `Retry-After` honoured, exponential delay capped at 60s.
- Offset/limit pagination for activities.
- Typed `Error`: `NotAvailable` (HTTP 410), `RateLimited`, `Auth`, `NotFound`,
  `Http`, `Network`, `Decode`, `Credentials`, `InvalidArgument`.
- `CredentialProvider` trait with `EnvProvider`, `CommandProvider`,
  `FileProvider`, `StaticProvider`; redacted `Debug` for secrets.
- Endpoints: API status; user management (register, login/portal URL, list,
  delete, rotate secret); connections (list, detail, delete, refresh, sync,
  disable, accounts); accounts (list, all, detail, details, balances, balance
  history, positions, all positions, option positions, activities); reference
  data (partner info, symbol search, symbol by ticker, brokerage instruments,
  account symbol search).
- `trading` cargo feature (off by default): order history, quotes, order
  impact, equity/option/crypto/complex orders, replace, cancel.
- Helpers: account-number masking, account selector, CSV/JSON activity export.
- Optional `snaptrade` CLI behind the `cli` feature (read-only).
