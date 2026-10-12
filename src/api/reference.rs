//! Reference data: partner info, symbol search, brokerage instruments.

use crate::client::{from_value, list_from_value, seg, Client};
use crate::error::Result;
use crate::models::{AccountId, Symbol};
use crate::transport::{Method, Transport};
use serde_json::{json, Value};

impl<T: Transport> Client<T> {
    /// `GET /snapTrade/partners`: client (partner) configuration and limits.
    /// Returns the raw JSON.
    pub fn partner_info(&self) -> Result<Value> {
        self.get("/snapTrade/partners", &[])
    }

    /// `POST /symbols`: search symbols by text.
    pub fn search_symbols(&self, substring: &str) -> Result<Vec<Symbol>> {
        list_from_value(
            "/symbols",
            self.request(
                Method::Post,
                "/symbols",
                &[],
                Some(&json!({ "substring": substring })),
            )?,
        )
    }

    /// `GET /symbols/{query}`: symbol detail by ticker or universal symbol id.
    pub fn symbol_by_ticker(&self, query: &str) -> Result<Symbol> {
        let p = format!("/symbols/{}", seg(query));
        from_value(&p, self.get(&p, &[])?)
    }

    /// `GET /brokerages/{slug}/instruments`: instruments a brokerage supports.
    /// Returns the raw JSON.
    pub fn brokerage_instruments(&self, slug: &str) -> Result<Value> {
        self.get(&format!("/brokerages/{}/instruments", seg(slug)), &[])
    }

    /// `POST /accounts/{accountId}/symbols`: search symbols tradable in an account.
    pub fn account_symbol_search(
        &self,
        account_id: impl Into<AccountId>,
        substring: &str,
    ) -> Result<Vec<Symbol>> {
        let p = format!("/accounts/{}/symbols", seg(account_id.into().as_str()));
        list_from_value(
            &p,
            self.request(
                Method::Post,
                &p,
                &self.scoped(&[]),
                Some(&json!({ "substring": substring })),
            )?,
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
            resp(200, r#"{"symbol":"AAPL"}"#),
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
