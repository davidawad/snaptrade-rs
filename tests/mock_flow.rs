//! End-to-end through the public API with a mocked transport: no network.

use snaptrade_rs::credentials::{Credentials, StaticProvider};
use snaptrade_rs::transport::{HttpResponse, Request, Transport};
use snaptrade_rs::{ActivityQuery, Client, Error};
use std::cell::RefCell;
use std::collections::VecDeque;

struct Script(RefCell<VecDeque<(u16, String)>>);

impl Transport for Script {
    fn send(&self, _req: &Request) -> Result<HttpResponse, String> {
        let (status, body) = self.0.borrow_mut().pop_front().expect("unexpected request");
        Ok(HttpResponse {
            status,
            retry_after: Some(0),
            body,
        })
    }
}

fn client(script: Vec<(u16, &str)>) -> Client<Script> {
    Client::new(
        Script(RefCell::new(
            script
                .into_iter()
                .map(|(s, b)| (s, b.to_string()))
                .collect(),
        )),
        Credentials::new("TEST-CLIENT", "test-key"),
    )
    .with_clock(Box::new(|_| {}), Box::new(|| 1_700_000_000))
}

#[test]
fn personal_key_session_accounts_balances_activities() {
    let c = client(vec![
        (
            200,
            r#"[{"id":"acct-1","institution_name":"Example Broker","number":"00001234"}]"#,
        ),
        (200, r#"[{"currency":{"code":"USD"},"cash":10.0}]"#),
        (429, "slow down"),
        (200, r#"{"data":[{"id":"a1"}],"pagination":{"total":1}}"#),
    ]);
    let accounts = c.accounts().unwrap();
    assert_eq!(accounts[0].id.as_str(), "acct-1");
    assert_eq!(c.balances("acct-1").unwrap().len(), 1);
    let acts = c
        .activities("acct-1", &ActivityQuery::new().from("2024-01-01"))
        .unwrap();
    assert_eq!(acts.len(), 1);
}

#[test]
fn http_410_surfaces_as_not_available() {
    let c = client(vec![(410, r#"{"detail":"not available"}"#)]);
    match c.positions("acct-1") {
        Err(e @ Error::NotAvailable { .. }) => assert_eq!(e.status(), Some(410)),
        other => panic!("expected NotAvailable, got {other:?}"),
    }
}

#[test]
fn static_provider_builds_credentials() {
    use snaptrade_rs::CredentialProvider;
    let p = StaticProvider::new(Credentials::new("a", "b"));
    assert_eq!(p.credentials().unwrap().client_id, "a");
    assert!(p.user().unwrap().is_none());
}

#[cfg(not(feature = "trading"))]
#[test]
fn default_build_exposes_no_trading_endpoint_in_the_table() {
    assert!(snaptrade_rs::endpoints().iter().all(|e| !e.trading));
}

/// `trading` must never be reachable through `default` or another feature.
#[test]
fn trading_feature_is_not_enabled_by_default_or_by_cli() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("default = []"));
    assert!(manifest.contains("cli = [\"dep:clap\"]"));
    assert!(manifest.contains("trading = []"));
}
