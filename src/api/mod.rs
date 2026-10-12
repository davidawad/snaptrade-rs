//! Endpoint methods, one module per SnapTrade API section. Everything here is
//! an `impl Client` block; see `endpoints()` for the machine-readable list.
//!
//! Responses are returned as [`serde_json::Value`]: SnapTrade adds fields
//! often and brokerages differ, so the client does not pin a response schema.
//! Account-scoped calls automatically append `userId`/`userSecret` when the
//! client has a user attached (standard keys); Personal keys send neither.

mod accounts;
mod auth;
mod connections;
mod reference;
#[cfg(feature = "trading")]
mod trading;

pub use accounts::ActivityQuery;
pub use auth::LoginOptions;
#[cfg(feature = "trading")]
pub use trading::{Action, EquityOrder, OrderType, TimeInForce};

use crate::transport::Method;

/// One documented endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Endpoint {
    /// SnapTrade operation id, e.g. `AccountInformation_getUserAccountBalance`.
    pub operation: &'static str,
    pub method: Method,
    /// Path relative to the API prefix (`/api/v1`).
    pub path: &'static str,
    /// Name of the `Client` method that calls it.
    pub rust_fn: &'static str,
    /// Behind the `trading` cargo feature.
    pub trading: bool,
}

const fn ep(
    operation: &'static str,
    method: Method,
    path: &'static str,
    rust_fn: &'static str,
    trading: bool,
) -> Endpoint {
    Endpoint {
        operation,
        method,
        path,
        rust_fn,
        trading,
    }
}

use Method::{Delete, Get, Post};

/// Endpoints compiled into every build.
pub const BASE_ENDPOINTS: &[Endpoint] = &[
    ep("ApiStatus_check", Get, "/", "api_status", false),
    ep(
        "Authentication_registerSnapTradeUser",
        Post,
        "/snapTrade/registerUser",
        "register_user",
        false,
    ),
    ep(
        "Authentication_loginSnapTradeUser",
        Post,
        "/snapTrade/login",
        "login",
        false,
    ),
    ep(
        "Authentication_listSnapTradeUsers",
        Get,
        "/snapTrade/listUsers",
        "list_users",
        false,
    ),
    ep(
        "Authentication_deleteSnapTradeUser",
        Delete,
        "/snapTrade/deleteUser",
        "delete_user",
        false,
    ),
    ep(
        "Authentication_resetSnapTradeUserSecret",
        Post,
        "/snapTrade/resetUserSecret",
        "reset_user_secret",
        false,
    ),
    ep(
        "Connections_listBrokerageAuthorizations",
        Get,
        "/authorizations",
        "connections",
        false,
    ),
    ep(
        "Connections_detailBrokerageAuthorization",
        Get,
        "/authorizations/{authorizationId}",
        "connection",
        false,
    ),
    ep(
        "Connections_deleteConnection",
        Delete,
        "/connection/{connectionId}",
        "delete_connection",
        false,
    ),
    ep(
        "Connections_refreshBrokerageAuthorization",
        Post,
        "/authorizations/{authorizationId}/refresh",
        "refresh_connection",
        false,
    ),
    ep(
        "Connections_syncBrokerageAuthorizationTransactions",
        Post,
        "/authorizations/{authorizationId}/transactions/sync",
        "sync_connection_transactions",
        false,
    ),
    ep(
        "Connections_disableBrokerageAuthorization",
        Post,
        "/authorizations/{authorizationId}/disable",
        "disable_connection",
        false,
    ),
    ep(
        "Connections_listConnectionAccounts",
        Get,
        "/connections/{connectionId}/accounts",
        "connection_accounts",
        false,
    ),
    ep(
        "AccountInformation_listUserAccounts",
        Get,
        "/accounts",
        "accounts",
        false,
    ),
    ep(
        "AccountInformation_listAllUserAccounts",
        Get,
        "/accounts/all",
        "all_accounts",
        false,
    ),
    ep(
        "AccountInformation_getUserAccountDetails",
        Get,
        "/accounts/{accountId}",
        "account",
        false,
    ),
    ep(
        "AccountInformation_getAccountDetails",
        Get,
        "/accounts/{accountId}/details",
        "account_details",
        false,
    ),
    ep(
        "AccountInformation_getUserAccountBalance",
        Get,
        "/accounts/{accountId}/balances",
        "balances",
        false,
    ),
    ep(
        "AccountInformation_getAccountBalanceHistory",
        Get,
        "/accounts/{accountId}/balanceHistory",
        "balance_history",
        false,
    ),
    ep(
        "AccountInformation_getUserAccountPositions",
        Get,
        "/accounts/{accountId}/positions",
        "positions",
        false,
    ),
    ep(
        "AccountInformation_getAllAccountPositions",
        Get,
        "/accounts/{accountId}/positions/all",
        "all_positions",
        false,
    ),
    ep(
        "Options_listOptionHoldings",
        Get,
        "/accounts/{accountId}/options",
        "option_positions",
        false,
    ),
    ep(
        "AccountInformation_getAccountActivities",
        Get,
        "/accounts/{accountId}/activities",
        "activities",
        false,
    ),
    ep(
        "ReferenceData_getPartnerInfo",
        Get,
        "/snapTrade/partners",
        "partner_info",
        false,
    ),
    ep(
        "ReferenceData_getSymbols",
        Post,
        "/symbols",
        "search_symbols",
        false,
    ),
    ep(
        "ReferenceData_getSymbolsByTicker",
        Get,
        "/symbols/{query}",
        "symbol_by_ticker",
        false,
    ),
    ep(
        "ReferenceData_listAllBrokerageInstruments",
        Get,
        "/brokerages/{slug}/instruments",
        "brokerage_instruments",
        false,
    ),
    ep(
        "ReferenceData_symbolSearchUserAccount",
        Post,
        "/accounts/{accountId}/symbols",
        "account_symbol_search",
        false,
    ),
];

/// Order and trading endpoints; only compiled with `--features trading`.
#[cfg(feature = "trading")]
pub const TRADING_ENDPOINTS: &[Endpoint] = &[
    ep(
        "AccountInformation_getUserAccountOrders",
        Get,
        "/accounts/{accountId}/orders",
        "orders",
        true,
    ),
    ep(
        "AccountInformation_getUserAccountRecentOrders",
        Get,
        "/accounts/{accountId}/recentOrders",
        "recent_orders",
        true,
    ),
    ep(
        "AccountInformation_getUserAccountOrderDetail",
        Post,
        "/accounts/{accountId}/orders/details",
        "order_detail",
        true,
    ),
    ep(
        "Trading_getUserAccountQuotes",
        Get,
        "/accounts/{accountId}/quotes",
        "quotes",
        true,
    ),
    ep(
        "Trading_getOrderImpact",
        Post,
        "/trade/impact",
        "order_impact",
        true,
    ),
    ep(
        "Trading_getOptionImpact",
        Post,
        "/accounts/{accountId}/trading/options/impact",
        "option_impact",
        true,
    ),
    ep(
        "Trading_placeOrder",
        Post,
        "/trade/{tradeId}",
        "place_checked_order",
        true,
    ),
    ep(
        "Trading_placeForceOrder",
        Post,
        "/trade/place",
        "place_order",
        true,
    ),
    ep(
        "Trading_placeMlegOrder",
        Post,
        "/accounts/{accountId}/trading/options",
        "place_option_order",
        true,
    ),
    ep(
        "Trading_searchCryptocurrencyPairInstruments",
        Get,
        "/accounts/{accountId}/trading/instruments/cryptocurrencyPairs",
        "crypto_pairs",
        true,
    ),
    ep(
        "Trading_getCryptocurrencyPairQuote",
        Get,
        "/accounts/{accountId}/trading/instruments/cryptocurrencyPairs/{instrumentSymbol}/quote",
        "crypto_pair_quote",
        true,
    ),
    ep(
        "Trading_previewCryptoOrder",
        Post,
        "/accounts/{accountId}/trading/crypto/preview",
        "preview_crypto_order",
        true,
    ),
    ep(
        "Trading_placeCryptoOrder",
        Post,
        "/accounts/{accountId}/trading/crypto",
        "place_crypto_order",
        true,
    ),
    ep(
        "Trading_cancelOrder",
        Post,
        "/accounts/{accountId}/trading/cancel",
        "cancel_order",
        true,
    ),
    ep(
        "Trading_replaceOrder",
        Post,
        "/accounts/{accountId}/trading/replace",
        "replace_order",
        true,
    ),
    ep(
        "Trading_placeComplexOrder",
        Post,
        "/accounts/{accountId}/trading/complex",
        "place_complex_order",
        true,
    ),
];

/// Every endpoint available in this build.
pub fn endpoints() -> Vec<Endpoint> {
    #[allow(unused_mut)]
    let mut v = BASE_ENDPOINTS.to_vec();
    #[cfg(feature = "trading")]
    v.extend_from_slice(TRADING_ENDPOINTS);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_build_has_no_trading_or_order_endpoints() {
        #[cfg(not(feature = "trading"))]
        for e in endpoints() {
            let l = format!("{} {}", e.operation, e.path).to_lowercase();
            for bad in ["trading_", "/trade", "/trading", "order", "quote", "crypto"] {
                assert!(!l.contains(bad), "{l}");
            }
            assert!(!e.trading);
        }
    }

    #[cfg(feature = "trading")]
    #[test]
    fn trading_feature_adds_exactly_the_trading_endpoints() {
        assert_eq!(
            endpoints().len(),
            BASE_ENDPOINTS.len() + TRADING_ENDPOINTS.len()
        );
        assert!(TRADING_ENDPOINTS.iter().all(|e| e.trading));
    }

    #[test]
    fn endpoint_table_has_no_duplicates() {
        let v = endpoints();
        for (i, a) in v.iter().enumerate() {
            for b in &v[i + 1..] {
                assert_ne!(a.operation, b.operation);
                assert_ne!(a.rust_fn, b.rust_fn);
            }
        }
    }
}
