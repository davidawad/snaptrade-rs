# snaptrade

Unofficial Rust client for the [SnapTrade](https://snaptrade.com) brokerage
aggregation API. Not affiliated with or endorsed by SnapTrade.

- HMAC-SHA256 signed requests (fresh timestamp and signature on every attempt)
- Pagination, `Retry-After`-aware exponential backoff on 429 and 5xx
- Typed errors, including HTTP 410 "not available for your account"
- Pluggable credentials: environment, external command, file, or your own trait impl
- Mockable transport: the whole test suite runs offline
- Order and trading endpoints only behind the **`trading`** cargo feature (off by default)
- Optional read-only `snaptrade` CLI behind the `cli` feature

## Install

```toml
[dependencies]
snaptrade = "0.1"
# order placement / order history (off by default, read the warning below):
# snaptrade = { version = "0.1", features = ["trading"] }
```

CLI:

```sh
cargo install snaptrade --features cli
```

## Quick start

```rust
use snaptrade::{credentials::EnvProvider, Client};

fn main() -> Result<(), snaptrade::Error> {
    // SNAPTRADE_CLIENT_ID and SNAPTRADE_CONSUMER_KEY
    let client = Client::from_provider(&EnvProvider::new())?;
    for account in client.accounts()? {
        println!("{}", account["id"]);
    }
    Ok(())
}
```

See `examples/` for balances, activity export and a custom credential provider.

## Personal API keys

SnapTrade "Personal" keys differ from standard partner keys:

- There is **no `userId` / `userSecret`**. Leave `Client::user` unset and the
  client sends neither. With a standard key, attach the registered user with
  `Client::user(UserAuth::new(id, secret))` or a provider whose `user()` returns one.
- Rate limits are **250 requests per minute per customer** and **10 requests per
  minute per account**. The client retries 429 with backoff (`Retry-After` first,
  otherwise 1, 2, 4... up to 60 seconds); after `max_retries` (default 6) you get
  `Error::RateLimited`.
- Some endpoints answer **HTTP 410** for Personal keys, notably
  **positions**. This is `Error::NotAvailable` and is never retried:

```rust
match client.positions(account_id) {
    Err(e) if e.is_not_available() => eprintln!("positions unavailable on this plan"),
    other => { other?; }
}
```

Check your plan and limits in the SnapTrade dashboard and docs
(<https://docs.snaptrade.com/docs/ratelimiting>).

## Credentials

Implement `CredentialProvider` or use a bundled provider:

| Provider | Source |
| --- | --- |
| `EnvProvider` | `SNAPTRADE_CLIENT_ID`, `SNAPTRADE_CONSUMER_KEY` (optional `SNAPTRADE_USER_ID`, `SNAPTRADE_USER_SECRET`); `with_prefix` to change the prefix |
| `CommandProvider` | runs a program once per field; the argument `{field}` is replaced with `client_id`, `consumer_key` (`user_id`, `user_secret` with `.with_user()`); `.rename_field` maps names; no shell, stderr discarded |
| `FileProvider` | `KEY=VALUE` lines or a JSON object (`clientId`, `consumerKey`, `userId`, `userSecret`) |
| `StaticProvider` | values you already hold |

Secrets have redacted `Debug` output, and error messages never include
credentials or request URLs. No default credential location is built in.

## API coverage

Paths are relative to `/api/v1` (the prefix is part of the signed path;
override with `Client::api_prefix`). Source of truth: the
[SnapTrade API reference](https://docs.snaptrade.com/reference) and
[request signature guide](https://docs.snaptrade.com/docs/request-signatures).
`snaptrade::endpoints()` returns this table at runtime.

| Area | Method | Path | Client fn |
| --- | --- | --- | --- |
| Status | GET | `/` | `api_status` |
| Users | POST | `/snapTrade/registerUser` | `register_user` |
| Users | POST | `/snapTrade/login` | `login` |
| Users | GET | `/snapTrade/listUsers` | `list_users` |
| Users | DELETE | `/snapTrade/deleteUser` | `delete_user` |
| Users | POST | `/snapTrade/resetUserSecret` | `reset_user_secret` |
| Connections | GET | `/authorizations` | `connections` |
| Connections | GET | `/authorizations/{id}` | `connection` |
| Connections | DELETE | `/connection/{id}` | `delete_connection` |
| Connections | POST | `/authorizations/{id}/refresh` | `refresh_connection` |
| Connections | POST | `/authorizations/{id}/transactions/sync` | `sync_connection_transactions` |
| Connections | POST | `/authorizations/{id}/disable` | `disable_connection` |
| Connections | GET | `/connections/{id}/accounts` | `connection_accounts` |
| Accounts | GET | `/accounts`, `/accounts/all` | `accounts`, `all_accounts` |
| Accounts | GET | `/accounts/{id}`, `/accounts/{id}/details` | `account`, `account_details` |
| Balances | GET | `/accounts/{id}/balances` | `balances` |
| Balances | GET | `/accounts/{id}/balanceHistory` | `balance_history` |
| Positions | GET | `/accounts/{id}/positions` | `positions` (410 on Personal) |
| Positions | GET | `/accounts/{id}/positions/all` | `all_positions` |
| Options | GET | `/accounts/{id}/options` | `option_positions` |
| Activities / reporting | GET | `/accounts/{id}/activities` | `activities` (paginated, date and type filters) |
| Reference | GET | `/snapTrade/partners` | `partner_info` |
| Reference | POST | `/symbols`, `/accounts/{id}/symbols` | `search_symbols`, `account_symbol_search` |
| Reference | GET | `/symbols/{query}`, `/brokerages/{slug}/instruments` | `symbol_by_ticker`, `brokerage_instruments` |

Notes on sourcing: the docs index lists the positions and option-holdings
endpoints, but their reference pages did not load when this crate was written,
so `/accounts/{id}/positions` and `/accounts/{id}/options` follow the endpoint
index and the official SDKs. Verify against a live key before relying on them.
SnapTrade's former standalone "reporting" endpoints are not in the current
reference; transaction history and performance data come from `activities` and
`balance_history`.

### Not covered

Webhook signature verification, the deprecated holdings endpoints, and typed
response models (responses are `serde_json::Value`, because SnapTrade and the
brokerages add fields frequently).

## Trading (feature `trading`, off by default)

```toml
snaptrade = { version = "0.1", features = ["trading"] }
```

Adds order history (`orders`, `recent_orders`, `order_detail`), `quotes`,
`order_impact`, `place_checked_order`, `place_order`, option, crypto and complex
orders, `replace_order` and `cancel_order`. **These can move real money.** The
default build contains none of them, and the CLI never exposes them.

Safety behaviour: POSTs are never retried after a 5xx or network error, so an
order is not submitted twice; they are retried only after HTTP 429, where the
request was rejected unprocessed. Prefer `order_impact` then
`place_checked_order`, and pass `client_order_id` so you can reconcile after an
ambiguous failure. Use at your own risk; this is not financial advice.

## CLI

Read-only: `status`, `connections`, `accounts`, `balances`, `positions`,
`all-positions`, `options`, `activities` (CSV/JSON export, account numbers
masked to the last four digits).

```sh
export SNAPTRADE_CLIENT_ID=... SNAPTRADE_CONSUMER_KEY=...
snaptrade accounts
snaptrade activities --brokerage example --last4 1234 --from 2024-01-01 --out-dir ./out
snaptrade --cred-source command --cred-command "my-vault get snaptrade {field}" connections
snaptrade --cred-source file --cred-file ./creds.env accounts
```

## Testing

```sh
cargo test                       # default features
cargo test --features cli,trading
```

All tests use a mocked transport with fixed fake values; there is no network
access and no real account data.

## License

Licensed under either of Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
or MIT ([LICENSE-MIT](LICENSE-MIT)) at your option.
