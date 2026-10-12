//! Connections (brokerage authorizations).

use crate::client::{from_value, list_from_value, seg, Client};
use crate::error::Result;
use crate::models::{Account, Connection, ConnectionId};
use crate::transport::{Method, Transport};
use serde_json::Value;

impl<T: Transport> Client<T> {
    /// `GET /authorizations`: all brokerage connections.
    pub fn connections(&self) -> Result<Vec<Connection>> {
        list_from_value(
            "/authorizations",
            self.get("/authorizations", &self.scoped(&[]))?,
        )
    }

    /// `GET /authorizations/{authorizationId}`.
    pub fn connection(&self, authorization_id: impl Into<ConnectionId>) -> Result<Connection> {
        let p = format!("/authorizations/{}", seg(authorization_id.into().as_str()));
        from_value(&p, self.get(&p, &self.scoped(&[]))?)
    }

    /// `DELETE /connection/{connectionId}`: removes the connection and its
    /// accounts from SnapTrade. Irreversible. Returns the raw JSON.
    pub fn delete_connection(&self, connection_id: impl Into<ConnectionId>) -> Result<Value> {
        self.request(
            Method::Delete,
            &format!("/connection/{}", seg(connection_id.into().as_str())),
            &self.scoped(&[]),
            None,
        )
    }

    /// `POST /authorizations/{authorizationId}/refresh`: queue a holdings
    /// refresh. Billable on some plans; mind the rate limits. Returns the raw JSON.
    pub fn refresh_connection(&self, authorization_id: impl Into<ConnectionId>) -> Result<Value> {
        self.request(
            Method::Post,
            &format!(
                "/authorizations/{}/refresh",
                seg(authorization_id.into().as_str())
            ),
            &self.scoped(&[]),
            None,
        )
    }

    /// `POST /authorizations/{authorizationId}/transactions/sync` (BETA).
    /// Returns the raw JSON.
    pub fn sync_connection_transactions(
        &self,
        authorization_id: impl Into<ConnectionId>,
    ) -> Result<Value> {
        self.request(
            Method::Post,
            &format!(
                "/authorizations/{}/transactions/sync",
                seg(authorization_id.into().as_str())
            ),
            &self.scoped(&[]),
            None,
        )
    }

    /// `POST /authorizations/{authorizationId}/disable`: force-disable a
    /// connection. Returns the raw JSON.
    pub fn disable_connection(&self, authorization_id: impl Into<ConnectionId>) -> Result<Value> {
        self.request(
            Method::Post,
            &format!(
                "/authorizations/{}/disable",
                seg(authorization_id.into().as_str())
            ),
            &self.scoped(&[]),
            None,
        )
    }

    /// `GET /connections/{connectionId}/accounts` (BETA).
    pub fn connection_accounts(
        &self,
        connection_id: impl Into<ConnectionId>,
    ) -> Result<Vec<Account>> {
        let p = format!(
            "/connections/{}/accounts",
            seg(connection_id.into().as_str())
        );
        list_from_value(&p, self.get(&p, &self.scoped(&[]))?)
    }
}

#[cfg(test)]
mod tests {
    use crate::client::testutil::*;

    #[test]
    fn connection_calls_use_expected_methods_and_paths() {
        let (c, seen, _) = client(vec![
            resp(200, "[]"),
            resp(200, r#"{"id":"a/b"}"#),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "{}"),
            resp(200, "[]"),
        ]);
        c.connections().unwrap();
        c.connection("a/b").unwrap();
        c.delete_connection("c1").unwrap();
        c.refresh_connection("c1").unwrap();
        c.sync_connection_transactions("c1").unwrap();
        c.disable_connection("c1").unwrap();
        c.connection_accounts("c1").unwrap();
        let got: Vec<(String, String)> = seen
            .borrow()
            .iter()
            .map(|r| {
                let path = r
                    .url
                    .split('?')
                    .next()
                    .unwrap()
                    .replace("https://example.test/api/v1", "");
                (r.method.as_str().to_string(), path)
            })
            .collect();
        let want = [
            ("GET", "/authorizations"),
            ("GET", "/authorizations/a%2Fb"),
            ("DELETE", "/connection/c1"),
            ("POST", "/authorizations/c1/refresh"),
            ("POST", "/authorizations/c1/transactions/sync"),
            ("POST", "/authorizations/c1/disable"),
            ("GET", "/connections/c1/accounts"),
        ];
        for (g, w) in got.iter().zip(want) {
            assert_eq!((g.0.as_str(), g.1.as_str()), w);
        }
    }
}
