//! Deserialisation of the typed models, using response shapes taken from the
//! SnapTrade API reference (https://docs.snaptrade.com/reference). All values
//! are invented placeholders; there is no real account data in this file.

use chrono::{Datelike, TimeZone, Utc};
use serde_json::json;
use snaptrade_rs::credentials::Credentials;
use snaptrade_rs::models::*;
use snaptrade_rs::transport::{HttpResponse, Request, Transport};
use snaptrade_rs::{ActivityQuery, Client};
use std::cell::RefCell;
use std::collections::VecDeque;

struct Script(RefCell<VecDeque<String>>);

impl Transport for Script {
    fn send(&self, _req: &Request) -> Result<HttpResponse, String> {
        let body = self.0.borrow_mut().pop_front().expect("unexpected request");
        Ok(HttpResponse {
            status: 200,
            retry_after: None,
            body,
        })
    }
}

fn client(bodies: Vec<serde_json::Value>) -> Client<Script> {
    Client::new(
        Script(RefCell::new(
            bodies.into_iter().map(|b| b.to_string()).collect(),
        )),
        Credentials::new("TEST-CLIENT", "test-key"),
    )
    .with_clock(Box::new(|_| {}), Box::new(|| 1_700_000_000))
}

fn account_json() -> serde_json::Value {
    json!({
        "id": "11111111-1111-4111-8111-111111111111",
        "brokerage_authorization": "22222222-2222-4222-8222-222222222222",
        "name": "Example Brokerage Account",
        "number": "Q1234567",
        "institution_name": "Example Broker",
        "created_date": "2024-02-01T12:00:00.000Z",
        "sync_status": {
            "transactions": {
                "initial_sync_completed": true,
                "last_successful_sync": "2026-10-01",
                "first_transaction_date": "2022-01-24"
            },
            "holdings": {
                "initial_sync_completed": true,
                "last_successful_sync": "2026-10-01T04:05:06Z"
            }
        },
        "balance": { "total": { "amount": 1234.56, "currency": "USD" } },
        "status": "open",
        "raw_type": "MARGIN",
        "is_paper": false,
        "meta": { "type": "margin" },
        "some_future_field": { "x": 1 }
    })
}

#[test]
fn accounts_decode_with_dates_money_and_extra() {
    let c = client(vec![json!([account_json()])]);
    let a = &c.accounts().unwrap()[0];
    assert_eq!(a.id.as_str(), "11111111-1111-4111-8111-111111111111");
    assert_eq!(
        a.brokerage_authorization.as_ref().unwrap().as_str(),
        "22222222-2222-4222-8222-222222222222"
    );
    assert_eq!(
        a.created_date,
        Some(Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap())
    );
    let tx = a
        .sync_status
        .as_ref()
        .unwrap()
        .transactions
        .as_ref()
        .unwrap();
    assert_eq!(tx.first_transaction_date.unwrap().year(), 2022);
    assert!(tx.last_successful_sync.is_some());
    let total = a.balance.as_ref().unwrap().total.as_ref().unwrap();
    assert_eq!(total.amount.as_ref().unwrap().as_str(), "1234.56");
    assert_eq!(total.currency.as_deref(), Some("USD"));
    assert_eq!(a.meta.as_ref().unwrap()["type"], "margin");
    assert_eq!(a.extra["some_future_field"]["x"], 1);
}

#[test]
fn balances_with_null_buying_power() {
    let c = client(vec![json!([{
        "currency": { "id": "c1", "code": "USD", "name": "US Dollar" },
        "cash": 100.25,
        "buying_power": null
    }])]);
    let b = &c.balances("acct").unwrap()[0];
    assert_eq!(b.currency.as_ref().unwrap().code.as_deref(), Some("USD"));
    assert_eq!(b.cash.as_ref().unwrap().to_f64(), Some(100.25));
    assert!(b.buying_power.is_none());
}

fn symbol_json() -> serde_json::Value {
    json!({
        "id": "33333333-3333-4333-8333-333333333333",
        "symbol": "VAB.TO",
        "raw_symbol": "VAB",
        "description": "Example Bond ETF",
        "currency": { "id": "c2", "code": "CAD", "name": "Canadian Dollar" },
        "exchange": { "id": "e1", "code": "TSX", "mic_code": "XTSE", "name": "Toronto Stock Exchange" },
        "type": { "id": "t1", "code": "et", "description": "ETF", "is_supported": true },
        "figi_code": "BBG000000000"
    })
}

#[test]
fn positions_and_option_positions() {
    let c = client(vec![
        json!([{
            "symbol": { "symbol": symbol_json(), "id": "p-1", "description": "Example Bond ETF" },
            "units": 10,
            "fractional_units": 0.5,
            "price": 25.0,
            "open_pnl": -3.1,
            "average_purchase_price": "25.31",
            "currency": { "code": "CAD" },
            "cash_equivalent": false
        }]),
        json!([{
            "symbol": { "option_symbol": {
                "id": "o-1",
                "ticker": "AAPL  261218C00200000",
                "option_type": "CALL",
                "strike_price": 200,
                "expiration_date": "2026-12-18",
                "is_mini_option": false,
                "underlying_symbol": { "symbol": "AAPL" }
            }},
            "units": 1,
            "price": 4.2,
            "average_purchase_price": 3.9,
            "currency": { "code": "USD" }
        }]),
        json!({ "positions": [{ "units": 1 }], "option_positions": [], "extra_key": true }),
        json!([{ "units": 2 }]),
    ]);
    let p = &c.positions("acct").unwrap()[0];
    let sym = p.symbol.as_ref().unwrap().symbol.as_ref().unwrap();
    assert_eq!(sym.symbol.as_deref(), Some("VAB.TO"));
    assert_eq!(
        sym.exchange.as_ref().unwrap().mic_code.as_deref(),
        Some("XTSE")
    );
    assert_eq!(
        sym.security_type.as_ref().unwrap().code.as_deref(),
        Some("et")
    );
    assert_eq!(sym.extra["figi_code"], "BBG000000000");
    assert_eq!(p.units.as_ref().unwrap().as_str(), "10");
    assert_eq!(p.average_purchase_price.as_ref().unwrap().as_str(), "25.31");
    assert!(p.open_pnl.as_ref().unwrap().is_negative());

    let o = &c.option_positions("acct").unwrap()[0];
    let os = o.symbol.as_ref().unwrap().option_symbol.as_ref().unwrap();
    assert_eq!(os.option_type, Some(OptionType::Call));
    assert_eq!(os.strike_price.as_ref().unwrap().as_str(), "200");
    assert_eq!(os.expiration_date.unwrap().to_string(), "2026-12-18");

    let all = c.all_positions("acct").unwrap();
    assert_eq!(all.positions.len(), 1);
    assert_eq!(all.extra["extra_key"], true);
    let bare = c.all_positions("acct").unwrap();
    assert_eq!(bare.positions.len(), 1);
}

#[test]
fn activities_decode_paginated_with_unknown_type_and_bare_symbol() {
    let page = json!({
        "data": [
            {
                "id": "act-1",
                "symbol": symbol_json(),
                "price": 25.5,
                "units": 2,
                "amount": -51.0,
                "currency": { "code": "CAD" },
                "type": "BUY",
                "description": "BUY 2 VAB",
                "trade_date": "2026-09-30T00:00:00Z",
                "settlement_date": "2026-10-02T00:00:00Z",
                "fee": 0.0,
                "fx_rate": null,
                "institution": "Example Broker",
                "external_reference_id": "ext-1",
                "option_type": ""
            },
            {
                "id": "act-2",
                "symbol": "XYZ",
                "type": "SOMETHING_NEW",
                "amount": 1.25,
                "trade_date": "2026-10-01"
            }
        ],
        "pagination": { "offset": 0, "limit": 1000, "total": 2 }
    });
    let c = client(vec![page]);
    let acts = c
        .activities("acct", &ActivityQuery::new().from("2026-01-01"))
        .unwrap();
    assert_eq!(acts.len(), 2);
    assert_eq!(acts[0].activity_type, Some(ActivityType::Buy));
    assert_eq!(acts[0].amount.as_ref().unwrap().as_str(), "-51");
    assert_eq!(acts[0].settlement_date.unwrap().day(), 2);
    assert!(acts[0].fx_rate.is_none());
    assert_eq!(
        acts[1].symbol.as_ref().unwrap().symbol.as_deref(),
        Some("XYZ")
    );
    assert_eq!(
        acts[1].activity_type,
        Some(ActivityType::Unknown("SOMETHING_NEW".into()))
    );
    assert!(acts[1].trade_date.is_some());
}

#[test]
fn connections_decode() {
    let c = client(vec![
        json!([{
            "id": "22222222-2222-4222-8222-222222222222",
            "created_date": "2024-02-01T12:00:00Z",
            "updated_date": "2026-01-01T00:00:00Z",
            "brokerage": {
                "id": "b1", "slug": "ALPACA", "name": "Alpaca", "display_name": "Alpaca",
                "description": "Example", "allows_trading": true
            },
            "name": "Example connection",
            "disabled": false,
            "disabled_date": null,
            "type": "trade",
            "meta": {}
        }]),
        json!({ "id": "c-9", "type": "read-plus" }),
    ]);
    let conns = c.connections().unwrap();
    assert_eq!(conns[0].connection_type, Some(ConnectionType::Trade));
    assert_eq!(
        conns[0].brokerage.as_ref().unwrap().slug.as_deref(),
        Some("ALPACA")
    );
    assert_eq!(conns[0].disabled, Some(false));
    let one = c.connection("c-9").unwrap();
    assert_eq!(
        one.connection_type,
        Some(ConnectionType::Unknown("read-plus".into()))
    );
}

#[test]
fn users_and_status() {
    let c = client(vec![
        json!({ "version": 151, "timestamp": "2026-10-01T00:00:00Z", "online": true }),
        json!({ "userId": "u1", "userSecret": "do-not-log" }),
        json!({ "redirectURI": "https://example.test/portal", "sessionId": "s1" }),
    ]);
    assert_eq!(c.api_status().unwrap().online, Some(true));
    let r = c.register_user("u1").unwrap();
    assert_eq!(r.user_id.as_str(), "u1");
    assert!(!format!("{r:?}").contains("do-not-log"));
    let l = c.login(&Default::default()).unwrap();
    assert_eq!(
        l.redirect_uri.as_deref(),
        Some("https://example.test/portal")
    );
}

#[test]
fn decode_errors_do_not_echo_values() {
    let c = client(vec![
        json!([{ "id": "acct-1", "created_date": "SECRET-VALUE-123" }]),
    ]);
    let msg = c.accounts().unwrap_err().to_string();
    assert!(msg.contains("item 0"), "{msg}");
    assert!(!msg.contains("SECRET-VALUE-123"), "{msg}");
}

#[test]
fn missing_primary_id_is_a_decode_error() {
    let c = client(vec![json!([{ "name": "no id" }])]);
    assert!(matches!(
        c.accounts().unwrap_err(),
        snaptrade_rs::Error::Decode { .. }
    ));
}

#[test]
fn raw_escape_hatch_and_custom_type() {
    let c = client(vec![
        json!([account_json()]),
        json!([{ "id": "x", "n": 1 }]),
    ]);
    let raw = c.get("/accounts", &c.scoped(&[])).unwrap();
    assert_eq!(raw[0]["institution_name"], "Example Broker");
    #[derive(serde::Deserialize)]
    struct Mine {
        n: u32,
    }
    let mine: Vec<Mine> = c.get_as("/accounts", &[]).unwrap();
    assert_eq!(mine[0].n, 1);
}

#[cfg(feature = "trading")]
mod trading {
    use super::*;
    use snaptrade_rs::EquityOrder;

    #[test]
    fn orders_quotes_and_impact() {
        let c = client(vec![
            json!([{
                "brokerage_order_id": "bo-1",
                "status": "EXECUTED",
                "universal_symbol": symbol_json(),
                "action": "BUY",
                "total_quantity": "10",
                "open_quantity": "0",
                "filled_quantity": "10",
                "execution_price": 25.1,
                "limit_price": 25.2,
                "stop_price": null,
                "order_type": "Limit",
                "time_in_force": "Day",
                "time_placed": "2026-10-01T14:00:00Z",
                "time_updated": "2026-10-01T14:00:01Z",
                "expiry_date": null
            }]),
            json!({ "orders": [{ "brokerage_order_id": "bo-2", "status": "NEW_STATE", "action": "BUY_COVER" }] }),
            json!([{ "symbol": symbol_json(), "bid_price": 25.0, "ask_price": 25.1, "last_trade_price": 25.05, "bid_size": 100, "ask_size": 200 }]),
            json!({
                "trade": { "id": "t-1", "universal_symbol": symbol_json(), "action": "BUY",
                           "order_type": "Market", "time_in_force": "Day", "price": null, "units": 5,
                           "account": { "id": "a" } },
                "trade_impacts": [{ "account": "a", "currency": "CAD", "remaining_cash": 950.5,
                                    "estimated_commission": 1.0, "forex_fees": 0 }],
                "combined_remaining_balance": { "cash": 1 }
            }),
        ]);
        let o = &c.orders("acct", Some("all"), None).unwrap()[0];
        assert_eq!(o.status, Some(OrderStatus::Executed));
        assert_eq!(o.action, Some(OrderAction::Buy));
        assert_eq!(o.order_type, Some(OrderType::Limit));
        assert_eq!(o.time_in_force, Some(TimeInForce::Day));
        assert_eq!(o.total_quantity.as_ref().unwrap().as_str(), "10");
        assert_eq!(o.execution_price.as_ref().unwrap().as_str(), "25.1");

        let r = c.recent_orders("acct", false).unwrap();
        assert_eq!(
            r.orders[0].status,
            Some(OrderStatus::Unknown("NEW_STATE".into()))
        );
        assert_eq!(r.orders[0].action, Some(OrderAction::BuyToCover));

        let q = &c.quotes("acct", "VAB.TO", true).unwrap()[0];
        assert_eq!(q.last_trade_price.as_ref().unwrap().as_str(), "25.05");

        let order = EquityOrder {
            account_id: "acct".into(),
            action: OrderAction::Buy,
            universal_symbol_id: Some("sym".into()),
            order_type: OrderType::Market,
            time_in_force: TimeInForce::Day,
            price: None,
            stop: None,
            units: Some(5.into()),
            notional_value: None,
            client_order_id: None,
        };
        let imp = c.order_impact(&order).unwrap();
        let trade = imp.trade.unwrap();
        assert_eq!(trade.id.unwrap().as_str(), "t-1");
        assert_eq!(
            imp.trade_impacts[0]
                .remaining_cash
                .as_ref()
                .unwrap()
                .as_str(),
            "950.5"
        );
    }
}
