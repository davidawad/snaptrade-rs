//! Reference data: partner info, symbol search, brokerage instruments.

use crate::client::{seg, Client};
use crate::error::Result;
use crate::transport::{Method, Transport};
use serde_json::{json, Value};

impl<T: Transport> Client<T> {
    /// `GET /snapTrade/partners`: client (partner) configuration and limits.
    pub fn partner_info(&self) -> Result<Value> {
        self.get("/snapTrade/partners", &[])
    }

    /// `POST /symbols`: search symbols by text.
    pub fn search_symbols(&self, substring: &str) -> Result<Value> {
        self.request(
            Method::Post,
            "/symbols",
            &[],
            Some(&json!({ "substring": substring })),
        )
    }

    /// `GET /symbols/{query}`: symbol detail by ticker or universal symbol id.
    pub fn symbol_by_ticker(&self, query: &str) -> Result<Value> {
        self.get(&format!("/symbols/{}", seg(query)), &[])
    }

    /// `GET /brokerages/{slug}/instruments`: instruments a brokerage supports.
    pub fn brokerage_instruments(&self, slug: &str) -> Result<Value> {
        self.get(&format!("/brokerages/{}/instruments", seg(slug)), &[])
    }

    /// `POST /accounts/{accountId}/symbols`: search symbols tradable in an account.
    pub fn account_symbol_search(&self, account_id: &str, substring: &str) -> Result<Value> {
        self.request(
            Method::Post,
            &format!("/accounts/{}/symbols", seg(account_id)),
            &self.scoped(&[]),
            Some(&json!({ "substring": substring })),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::client::testutil::*;

    #[test]
    fn reference_calls() {
        let (c, seen, _) = client(vec![
            resp(200, "{}"),
            resp(200, "[]"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "[]"),
        ]);
        c.partner_info().unwrap();
        c.search_symbols("AAPL").unwrap();
        c.symbol_by_ticker("AAPL").unwrap();
        c.brokerage_instruments("alpaca").unwrap();
        c.account_symbol_search("acct", "TS").unwrap();
        let s = seen.borrow();
        assert_eq!(s[1].body.as_deref(), Some(r#"{"substring":"AAPL"}"#));
        assert!(s[3].url.contains("/brokerages/alpaca/instruments"));
        assert!(s[4].url.contains("/accounts/acct/symbols"));
    }
}
