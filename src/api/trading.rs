//! Orders and trading. **Only compiled with the `trading` cargo feature**, which
//! is off by default.
//!
//! Everything here can move real money (place, replace, cancel) or exposes
//! order history. Read the SnapTrade docs on trading and test against a paper
//! brokerage before pointing this at a live account. The client never retries
//! a POST after a 5xx or network error (so an order is not submitted twice);
//! it does retry after HTTP 429, where the request was rejected unprocessed.
//! Pass a `client_order_id` where the API supports it so you can reconcile.

use crate::client::{as_array, seg, Client};
use crate::error::{Error, Result};
use crate::transport::{Method, Transport};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Action {
    Buy,
    Sell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OrderType {
    Limit,
    Market,
    StopLimit,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TimeInForce {
    Day,
    #[serde(rename = "GTC")]
    Gtc,
    #[serde(rename = "FOK")]
    Fok,
    #[serde(rename = "IOC")]
    Ioc,
}

/// Body for [`Client::order_impact`] and [`Client::place_order`].
#[derive(Clone, Debug, Serialize)]
pub struct EquityOrder {
    pub account_id: String,
    pub action: Action,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub universal_symbol_id: Option<String>,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notional_value: Option<f64>,
    /// Only used by [`Client::place_order`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

fn to_body<B: Serialize>(b: &B) -> Result<Value> {
    serde_json::to_value(b).map_err(|e| Error::InvalidArgument(e.to_string()))
}

impl<T: Transport> Client<T> {
    // ---- order history (read-only, but gated with the rest of the order API) ----

    /// `GET /accounts/{accountId}/orders`. `state`: `all`, `open` or `executed`.
    pub fn orders(
        &self,
        account_id: &str,
        state: Option<&str>,
        days: Option<u32>,
    ) -> Result<Vec<Value>> {
        let mut p = Vec::new();
        if let Some(s) = state {
            p.push(("state", s.to_string()));
        }
        if let Some(d) = days {
            p.push(("days", d.to_string()));
        }
        let path = format!("/accounts/{}/orders", seg(account_id));
        as_array(&path, self.get(&path, &self.scoped(&p))?)
    }

    /// `GET /accounts/{accountId}/recentOrders`: last 24 hours only.
    pub fn recent_orders(&self, account_id: &str, only_executed: bool) -> Result<Value> {
        self.get(
            &format!("/accounts/{}/recentOrders", seg(account_id)),
            &self.scoped(&[("only_executed", only_executed.to_string())]),
        )
    }

    /// `POST /accounts/{accountId}/orders/details`.
    pub fn order_detail(&self, account_id: &str, brokerage_order_id: &str) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/accounts/{}/orders/details", seg(account_id)),
            &self.scoped(&[]),
            Some(&json!({ "brokerage_order_id": brokerage_order_id })),
        )
    }

    // ---- quotes ----

    /// `GET /accounts/{accountId}/quotes`: `symbols` is comma-separated.
    pub fn quotes(&self, account_id: &str, symbols: &str, use_ticker: bool) -> Result<Value> {
        self.get(
            &format!("/accounts/{}/quotes", seg(account_id)),
            &self.scoped(&[
                ("symbols", symbols.to_string()),
                ("use_ticker", use_ticker.to_string()),
            ]),
        )
    }

    // ---- equity ----

    /// `POST /trade/impact`: check the impact of an equity order. The response
    /// holds a `trade.id` for [`Client::place_checked_order`] (valid ~5 minutes).
    pub fn order_impact(&self, order: &EquityOrder) -> Result<Value> {
        self.request(
            Method::Post,
            "/trade/impact",
            &self.scoped(&[]),
            Some(&to_body(order)?),
        )
    }

    /// `POST /trade/{tradeId}`: place a previously checked order. **Moves money.**
    pub fn place_checked_order(&self, trade_id: &str, wait_to_confirm: bool) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/trade/{}", seg(trade_id)),
            &self.scoped(&[]),
            Some(&json!({ "wait_to_confirm": wait_to_confirm })),
        )
    }

    /// `POST /trade/place`: place an equity order without a prior check.
    /// **Moves money.**
    pub fn place_order(&self, order: &EquityOrder) -> Result<Value> {
        self.request(
            Method::Post,
            "/trade/place",
            &self.scoped(&[]),
            Some(&to_body(order)?),
        )
    }

    // ---- options ----

    /// `POST /accounts/{accountId}/trading/options/impact` (BETA). `order` is
    /// the documented body: `order_type`, `time_in_force`, `legs`, ...
    pub fn option_impact(&self, account_id: &str, order: &Value) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/accounts/{}/trading/options/impact", seg(account_id)),
            &self.scoped(&[]),
            Some(order),
        )
    }

    /// `POST /accounts/{accountId}/trading/options`: multi-leg option order.
    /// **Moves money.**
    pub fn place_option_order(&self, account_id: &str, order: &Value) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/accounts/{}/trading/options", seg(account_id)),
            &self.scoped(&[]),
            Some(order),
        )
    }

    // ---- crypto ----

    /// `GET /accounts/{accountId}/trading/instruments/cryptocurrencyPairs`.
    pub fn crypto_pairs(
        &self,
        account_id: &str,
        base: Option<&str>,
        quote: Option<&str>,
    ) -> Result<Value> {
        let mut p = Vec::new();
        if let Some(b) = base {
            p.push(("base", b.to_string()));
        }
        if let Some(q) = quote {
            p.push(("quote", q.to_string()));
        }
        self.get(
            &format!(
                "/accounts/{}/trading/instruments/cryptocurrencyPairs",
                seg(account_id)
            ),
            &self.scoped(&p),
        )
    }

    /// `GET .../cryptocurrencyPairs/{instrumentSymbol}/quote`.
    pub fn crypto_pair_quote(&self, account_id: &str, instrument_symbol: &str) -> Result<Value> {
        self.get(
            &format!(
                "/accounts/{}/trading/instruments/cryptocurrencyPairs/{}/quote",
                seg(account_id),
                seg(instrument_symbol)
            ),
            &self.scoped(&[]),
        )
    }

    /// `POST /accounts/{accountId}/trading/crypto/preview`.
    pub fn preview_crypto_order(&self, account_id: &str, order: &Value) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/accounts/{}/trading/crypto/preview", seg(account_id)),
            &self.scoped(&[]),
            Some(order),
        )
    }

    /// `POST /accounts/{accountId}/trading/crypto`. **Moves money.**
    pub fn place_crypto_order(&self, account_id: &str, order: &Value) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/accounts/{}/trading/crypto", seg(account_id)),
            &self.scoped(&[]),
            Some(order),
        )
    }

    // ---- change / cancel ----

    /// `POST /accounts/{accountId}/trading/cancel`.
    pub fn cancel_order(&self, account_id: &str, brokerage_order_id: &str) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/accounts/{}/trading/cancel", seg(account_id)),
            &self.scoped(&[]),
            Some(&json!({ "brokerage_order_id": brokerage_order_id })),
        )
    }

    /// `POST /accounts/{accountId}/trading/replace`: `order` has
    /// `brokerage_order_id`, `action`, `order_type`, `time_in_force`, ...
    pub fn replace_order(&self, account_id: &str, order: &Value) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/accounts/{}/trading/replace", seg(account_id)),
            &self.scoped(&[]),
            Some(order),
        )
    }

    /// `POST /accounts/{accountId}/trading/complex`: complex (OCO/OTO...) order.
    pub fn place_complex_order(&self, account_id: &str, order: &Value) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/accounts/{}/trading/complex", seg(account_id)),
            &self.scoped(&[]),
            Some(order),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::testutil::*;

    fn order() -> EquityOrder {
        EquityOrder {
            account_id: "acct".into(),
            action: Action::Buy,
            universal_symbol_id: Some("sym".into()),
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Gtc,
            price: Some(1.5),
            stop: None,
            units: Some(2.0),
            notional_value: None,
            client_order_id: None,
        }
    }

    #[test]
    fn equity_order_serialises_to_api_shape() {
        let v = to_body(&order()).unwrap();
        assert_eq!(v["action"], "BUY");
        assert_eq!(v["order_type"], "Limit");
        assert_eq!(v["time_in_force"], "GTC");
        assert!(v.get("stop").is_none());
    }

    #[test]
    fn place_order_is_post_signed_with_body_and_never_retried_on_5xx() {
        let (c, seen, _) = client(vec![resp(502, "bad gateway")]);
        let e = c.place_order(&order()).unwrap_err();
        assert_eq!(e.status(), Some(502));
        assert_eq!(seen.borrow().len(), 1);
        assert!(seen.borrow()[0].url.contains("/api/v1/trade/place?"));
    }

    #[test]
    fn trading_paths() {
        let (c, seen, _) = client(vec![
            resp(200, "[]"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "{}"),
        ]);
        c.orders("a", Some("open"), Some(7)).unwrap();
        c.recent_orders("a", true).unwrap();
        c.order_detail("a", "o1").unwrap();
        c.quotes("a", "AAPL,MSFT", true).unwrap();
        c.place_checked_order("t1", true).unwrap();
        c.cancel_order("a", "o1").unwrap();
        c.crypto_pairs("a", Some("BTC"), None).unwrap();
        c.crypto_pair_quote("a", "BTC-USD").unwrap();
        c.replace_order("a", &json!({"brokerage_order_id":"o1"}))
            .unwrap();
        let s = seen.borrow();
        assert!(s[0]
            .url
            .contains("/accounts/a/orders?clientId=CID&days=7&state=open"));
        assert!(s[3]
            .url
            .contains("symbols=AAPL%2CMSFT&timestamp=1700000000&use_ticker=true"));
        assert!(s[4].url.contains("/trade/t1?"));
        assert_eq!(s[4].body.as_deref(), Some(r#"{"wait_to_confirm":true}"#));
        assert!(s[7].url.contains("cryptocurrencyPairs/BTC-USD/quote"));
        assert!(s[8].url.contains("/trading/replace?"));
    }
}
