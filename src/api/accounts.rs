//! Account information: accounts, balances, positions, option positions,
//! activities (transaction history and reporting).

use crate::client::{from_value, list_from_value, seg, Client};
use crate::error::{Error, Result};
use crate::models::{
    Account, AccountId, Activity, AllPositions, Balance, OptionPosition, Position,
};
use crate::transport::Transport;
use serde_json::Value;

/// Filters for [`Client::activities`]. Dates are inclusive `YYYY-MM-DD`.
#[derive(Clone, Debug, Default)]
pub struct ActivityQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    /// Comma-separated activity types, e.g. `BUY,SELL,DIVIDEND`.
    pub activity_type: Option<String>,
}

impl ActivityQuery {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from(mut self, d: &str) -> Self {
        self.start_date = Some(d.into());
        self
    }
    pub fn to(mut self, d: &str) -> Self {
        self.end_date = Some(d.into());
        self
    }
    pub fn activity_type(mut self, t: &str) -> Self {
        self.activity_type = Some(t.into());
        self
    }

    fn params(&self) -> Result<Vec<(&'static str, String)>> {
        let mut p = Vec::new();
        for (key, label, v) in [
            ("startDate", "start date", &self.start_date),
            ("endDate", "end date", &self.end_date),
        ] {
            if let Some(d) = v {
                if chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err() {
                    return Err(Error::InvalidArgument(format!(
                        "{label} must be YYYY-MM-DD, got '{d}'"
                    )));
                }
                p.push((key, d.clone()));
            }
        }
        if let Some(t) = &self.activity_type {
            p.push(("type", t.clone()));
        }
        Ok(p)
    }
}

impl<T: Transport> Client<T> {
    /// `GET /accounts`.
    pub fn accounts(&self) -> Result<Vec<Account>> {
        list_from_value("/accounts", self.get("/accounts", &self.scoped(&[]))?)
    }

    /// `GET /accounts/all`: accounts including those on disabled connections.
    pub fn all_accounts(&self) -> Result<Vec<Account>> {
        list_from_value(
            "/accounts/all",
            self.get("/accounts/all", &self.scoped(&[]))?,
        )
    }

    /// `GET /accounts/{accountId}`.
    pub fn account(&self, account_id: impl Into<AccountId>) -> Result<Account> {
        let p = format!("/accounts/{}", seg(account_id.into().as_str()));
        from_value(&p, self.get(&p, &self.scoped(&[]))?)
    }

    /// `GET /accounts/{accountId}/details`.
    pub fn account_details(&self, account_id: impl Into<AccountId>) -> Result<Account> {
        let p = format!("/accounts/{}/details", seg(account_id.into().as_str()));
        from_value(&p, self.get(&p, &self.scoped(&[]))?)
    }

    /// `GET /accounts/{accountId}/balances`: one row per currency.
    pub fn balances(&self, account_id: impl Into<AccountId>) -> Result<Vec<Balance>> {
        let p = format!("/accounts/{}/balances", seg(account_id.into().as_str()));
        list_from_value(&p, self.get(&p, &self.scoped(&[]))?)
    }

    /// `GET /accounts/{accountId}/balanceHistory` (BETA): historical total
    /// value. Returned as raw JSON: the shape is not stable enough to pin.
    pub fn balance_history(&self, account_id: impl Into<AccountId>) -> Result<Value> {
        self.get(
            &format!(
                "/accounts/{}/balanceHistory",
                seg(account_id.into().as_str())
            ),
            &self.scoped(&[]),
        )
    }

    /// `GET /accounts/{accountId}/positions`: stock, ETF, crypto and mutual
    /// fund positions.
    ///
    /// Personal API keys receive HTTP 410 here ([`Error::NotAvailable`]);
    /// use [`Client::all_positions`] or check the error. This path is taken
    /// from SnapTrade's endpoint index and official SDKs, because the docs'
    /// reference page for it was not retrievable when this crate was written.
    pub fn positions(&self, account_id: impl Into<AccountId>) -> Result<Vec<Position>> {
        let p = format!("/accounts/{}/positions", seg(account_id.into().as_str()));
        list_from_value(&p, self.get(&p, &self.scoped(&[]))?)
    }

    /// `GET /accounts/{accountId}/positions/all`: positions across all
    /// instrument types (stocks, ETFs, mutual funds, crypto, futures, options).
    pub fn all_positions(&self, account_id: impl Into<AccountId>) -> Result<AllPositions> {
        let p = format!(
            "/accounts/{}/positions/all",
            seg(account_id.into().as_str())
        );
        from_value(&p, self.get(&p, &self.scoped(&[]))?)
    }

    /// `GET /accounts/{accountId}/options`: option positions (holdings).
    ///
    /// Same sourcing note as [`Client::positions`].
    pub fn option_positions(
        &self,
        account_id: impl Into<AccountId>,
    ) -> Result<Vec<OptionPosition>> {
        let p = format!("/accounts/{}/options", seg(account_id.into().as_str()));
        list_from_value(&p, self.get(&p, &self.scoped(&[]))?)
    }

    /// `GET /accounts/{accountId}/activities`: the full history for the filter,
    /// following `offset`/`limit` pagination (page size from
    /// [`Client::page_limit`]; SnapTrade caps it at 1000) until `total` is
    /// reached or a page is empty.
    pub fn activities(
        &self,
        account_id: impl Into<AccountId>,
        q: &ActivityQuery,
    ) -> Result<Vec<Activity>> {
        let path = format!("/accounts/{}/activities", seg(account_id.into().as_str()));
        let mut params = q.params()?;
        for (k, v) in self.scoped(&[]) {
            params.push((k, v));
        }
        list_from_value(&path, Value::Array(self.paginate(&path, &params)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::testutil::*;
    use crate::credentials::UserAuth;

    #[test]
    fn activities_paginate_with_filters() {
        let p1 =
            r#"{"data":[{"id":"1"},{"id":"2"}],"pagination":{"offset":0,"limit":2,"total":3}}"#;
        let p2 = r#"{"data":[{"id":"3"}],"pagination":{"offset":2,"limit":2,"total":3}}"#;
        let (c, seen, _) = client(vec![resp(200, p1), resp(200, p2)]);
        let all = c
            .activities(
                "acct",
                &ActivityQuery::new().from("2024-01-01").activity_type("BUY"),
            )
            .unwrap();
        assert_eq!(all.len(), 3);
        let s = seen.borrow();
        assert!(s[0]
            .url
            .contains("limit=2&offset=0&startDate=2024-01-01&timestamp=1700000000&type=BUY"));
        assert!(!s[0].url.contains("endDate"));
        assert!(s[1].url.contains("offset=2"));
    }

    #[test]
    fn bad_date_is_rejected_before_any_request() {
        let (c, seen, _) = client(vec![]);
        let e = c
            .activities("a", &ActivityQuery::new().from("01/02/2024"))
            .unwrap_err();
        assert!(matches!(e, Error::InvalidArgument(_)));
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn personal_key_sends_no_user_params_standard_key_does() {
        let (c, seen, _) = client(vec![resp(200, "[]")]);
        c.accounts().unwrap();
        assert!(!seen.borrow()[0].url.contains("user"));
        let (c, seen, _) = client_with(vec![resp(200, "[]")], |c| c.user(UserAuth::new("u", "s")));
        c.accounts().unwrap();
        assert!(seen.borrow()[0].url.ends_with("userId=u&userSecret=s"));
    }

    #[test]
    fn positions_410_is_not_available_for_personal_keys() {
        let (c, seen, _) = client(vec![resp(410, "gone")]);
        let e = c.positions("x").unwrap_err();
        assert!(e.is_not_available());
        assert_eq!(seen.borrow().len(), 1);
    }

    #[test]
    fn read_endpoints_paths() {
        let (c, seen, _) = client(vec![
            resp(200, "[]"),
            resp(200, "[]"),
            resp(200, r#"{"id":"a"}"#),
            resp(200, r#"{"id":"a"}"#),
            resp(200, "[]"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "[]"),
            resp(200, "[]"),
        ]);
        c.accounts().unwrap();
        c.all_accounts().unwrap();
        c.account("a").unwrap();
        c.account_details("a").unwrap();
        c.balances("a").unwrap();
        c.balance_history("a").unwrap();
        c.all_positions("a").unwrap();
        c.positions("a").unwrap();
        c.option_positions("a").unwrap();
        let paths: Vec<String> = seen
            .borrow()
            .iter()
            .map(|r| {
                r.url
                    .split('?')
                    .next()
                    .unwrap()
                    .replace("https://example.test/api/v1", "")
            })
            .collect();
        assert_eq!(
            paths,
            [
                "/accounts",
                "/accounts/all",
                "/accounts/a",
                "/accounts/a/details",
                "/accounts/a/balances",
                "/accounts/a/balanceHistory",
                "/accounts/a/positions/all",
                "/accounts/a/positions",
                "/accounts/a/options"
            ]
        );
    }

    #[test]
    fn list_endpoint_returning_object_is_a_decode_error() {
        let (c, _, _) = client(vec![resp(200, "{}")]);
        assert!(matches!(c.accounts().unwrap_err(), Error::Decode { .. }));
    }
}
