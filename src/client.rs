//! The signed client core: request building, signing, retry/backoff,
//! pagination. Endpoint methods live in [`crate::api`].

use crate::credentials::{CredentialProvider, Credentials, UserAuth};
use crate::error::{from_status, Error, Result};
use crate::sign::{build_query, encode_component, signature};
use crate::transport::{HttpResponse, Method, Request, ReqwestTransport, Transport};
use serde_json::Value;
use std::time::Duration;

pub const DEFAULT_BASE_URL: &str = "https://api.snaptrade.com";
/// Path prefix that is part of every signed path.
pub const DEFAULT_API_PREFIX: &str = "/api/v1";
pub const DEFAULT_MAX_RETRIES: u32 = 6;
pub const DEFAULT_PAGE_LIMIT: u32 = 1000;

/// Delay before retry number `attempt` (0-based): `Retry-After` if given, else
/// exponential `2^attempt` seconds, both capped at 60s.
pub fn backoff_delay(attempt: u32, retry_after: Option<u64>) -> Duration {
    let secs = retry_after.unwrap_or_else(|| 1u64 << attempt.min(6));
    Duration::from_secs(secs.min(60))
}

/// Percent-encode a value placed in a URL path segment.
pub(crate) fn seg(s: &str) -> String {
    encode_component(s)
}

/// A SnapTrade API client over any [`Transport`].
pub struct Client<T: Transport = ReqwestTransport> {
    transport: T,
    creds: Credentials,
    user: Option<UserAuth>,
    base_url: String,
    api_prefix: String,
    max_retries: u32,
    page_limit: u32,
    sleep: Box<dyn Fn(Duration)>,
    now: Box<dyn Fn() -> i64>,
}

impl Client<ReqwestTransport> {
    /// Build a client with the default HTTP transport from a credential provider.
    /// If the provider has a user (`userId`/`userSecret`) it is attached.
    pub fn from_provider(provider: &dyn CredentialProvider) -> Result<Self> {
        let transport = ReqwestTransport::new().map_err(Error::Credentials)?;
        let mut c = Client::new(transport, provider.credentials()?);
        c.user = provider.user()?;
        Ok(c)
    }
}

impl<T: Transport> Client<T> {
    pub fn new(transport: T, creds: Credentials) -> Self {
        Self {
            transport,
            creds,
            user: None,
            base_url: DEFAULT_BASE_URL.into(),
            api_prefix: DEFAULT_API_PREFIX.into(),
            max_retries: DEFAULT_MAX_RETRIES,
            page_limit: DEFAULT_PAGE_LIMIT,
            sleep: Box::new(std::thread::sleep),
            now: Box::new(|| chrono::Utc::now().timestamp()),
        }
    }

    pub fn base_url(mut self, url: &str) -> Self {
        self.base_url = url.trim_end_matches('/').to_string();
        self
    }

    /// Override the signed path prefix (default `/api/v1`).
    pub fn api_prefix(mut self, prefix: &str) -> Self {
        self.api_prefix = format!("/{}", prefix.trim_matches('/'));
        if self.api_prefix == "/" {
            self.api_prefix.clear();
        }
        self
    }

    /// Attach a registered user. Standard (non-Personal) keys need this on
    /// every account-scoped call; Personal keys must leave it unset.
    pub fn user(mut self, user: UserAuth) -> Self {
        self.user = Some(user);
        self
    }

    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    pub fn page_limit(mut self, n: u32) -> Self {
        self.page_limit = n.max(1);
        self
    }

    /// Replace the sleep and clock functions (tests).
    pub fn with_clock(mut self, sleep: Box<dyn Fn(Duration)>, now: Box<dyn Fn() -> i64>) -> Self {
        self.sleep = sleep;
        self.now = now;
        self
    }

    /// Query pairs for an account-scoped call: caller params plus
    /// `userId`/`userSecret` when a user is attached.
    pub fn scoped<'a>(&self, params: &[(&'a str, String)]) -> Vec<(&'a str, String)> {
        let mut v: Vec<(&'a str, String)> = params.to_vec();
        if let Some(u) = &self.user {
            v.push(("userId", u.user_id.clone()));
            v.push(("userSecret", u.user_secret.clone()));
        }
        v
    }

    /// Signed GET returning the raw JSON; `path` excludes the API prefix
    /// (e.g. `/accounts`). This is the escape hatch from the typed models:
    /// pair it with [`Client::scoped`] to attach the user, e.g.
    /// `client.get("/accounts", &client.scoped(&[]))`.
    pub fn get(&self, path: &str, params: &[(&str, String)]) -> Result<Value> {
        self.request(Method::Get, path, params, None)
    }

    /// Signed GET decoded into any type of yours. Use it for a response shape
    /// the bundled models do not cover.
    pub fn get_as<R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<R> {
        from_value(path, self.get(path, params)?)
    }

    /// Signed request with retry. A fresh timestamp and signature are made for
    /// every attempt. 429 is retried for every method (the request was
    /// rejected before processing); 5xx and network errors only for
    /// idempotent methods, so an order is never submitted twice.
    pub fn request(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<Value> {
        let signed_path = format!("{}{}", self.api_prefix, path);
        let body_text = body.map(|b| b.to_string());
        let mut attempt = 0;
        loop {
            let query = build_query(&self.creds.client_id, (self.now)(), params);
            let sig = signature(&self.creds.consumer_key, &signed_path, &query, body);
            let req = Request {
                method,
                url: format!("{}{}?{}", self.base_url, signed_path, query),
                signature: sig,
                body: body_text.clone(),
            };
            let can_retry = attempt < self.max_retries;
            match self.transport.send(&req) {
                Ok(r) if (200..300).contains(&r.status) => return decode(path, &r),
                Ok(r) if can_retry && r.status == 429 => {
                    (self.sleep)(backoff_delay(attempt, r.retry_after));
                }
                Ok(r) if can_retry && (500..600).contains(&r.status) && method.is_idempotent() => {
                    (self.sleep)(backoff_delay(attempt, r.retry_after));
                }
                Ok(r) => return Err(from_status(path, r.status, r.retry_after, &r.body)),
                Err(_) if can_retry && method.is_idempotent() => {
                    (self.sleep)(backoff_delay(attempt, None));
                }
                Err(message) => {
                    return Err(Error::Network {
                        path: path.into(),
                        message,
                    })
                }
            }
            attempt += 1;
        }
    }

    /// Follow `offset`/`limit` pagination for endpoints that answer
    /// `{"data":[...],"pagination":{"total":N}}`, until `total` is reached or a
    /// page comes back empty. Endpoints that answer a bare array are returned
    /// as-is after one call.
    pub(crate) fn paginate(&self, path: &str, params: &[(&str, String)]) -> Result<Vec<Value>> {
        let mut out: Vec<Value> = Vec::new();
        loop {
            let mut p: Vec<(&str, String)> = params.to_vec();
            p.push(("limit", self.page_limit.to_string()));
            p.push(("offset", out.len().to_string()));
            let page = self.get(path, &p)?;
            if let Value::Array(a) = page {
                return Ok(a);
            }
            let data = page["data"].as_array().cloned().unwrap_or_default();
            let total = page["pagination"]["total"].as_u64();
            let got = data.len();
            out.extend(data);
            if got == 0 || total.is_none_or(|t| out.len() as u64 >= t) {
                return Ok(out);
            }
        }
    }
}

fn decode(path: &str, r: &HttpResponse) -> Result<Value> {
    if r.body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&r.body).map_err(|e| Error::Decode {
        path: path.into(),
        message: format!("invalid JSON: {e}"),
    })
}

/// Decode a JSON value into a model. The error message never echoes response
/// values (serde messages can quote them), only the kind of mismatch.
pub(crate) fn from_value<R: serde::de::DeserializeOwned>(path: &str, v: Value) -> Result<R> {
    serde_json::from_value(v).map_err(|e| Error::Decode {
        path: path.into(),
        message: scrub(&e.to_string()),
    })
}

/// Decode a JSON array into a list of models, naming the failing index.
pub(crate) fn list_from_value<R: serde::de::DeserializeOwned>(
    path: &str,
    v: Value,
) -> Result<Vec<R>> {
    as_array(path, v)?
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            serde_json::from_value(item).map_err(|e| Error::Decode {
                path: path.into(),
                message: format!("item {i}: {}", scrub(&e.to_string())),
            })
        })
        .collect()
}

/// Drop quoted text from a serde error message (it may hold account data).
fn scrub(msg: &str) -> String {
    let mut out = String::new();
    let mut in_quote = false;
    for c in msg.chars() {
        match (c, in_quote) {
            ('"', false) => {
                in_quote = true;
                out.push_str("\"...\"");
            }
            ('"', true) => in_quote = false,
            (_, true) => {}
            (c, false) => out.push(c),
        }
    }
    out
}

/// Require a JSON array, for list endpoints.
pub(crate) fn as_array(path: &str, v: Value) -> Result<Vec<Value>> {
    match v {
        Value::Array(a) => Ok(a),
        other => Err(Error::Decode {
            path: path.into(),
            message: format!("expected a JSON array, got {}", type_name(&other)),
        }),
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
pub(crate) mod testutil {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    pub type Seen = Rc<RefCell<Vec<Request>>>;
    pub type Slept = Rc<RefCell<Vec<Duration>>>;

    pub struct Mock {
        pub replies: RefCell<VecDeque<std::result::Result<HttpResponse, String>>>,
        pub seen: Seen,
    }

    impl Transport for Mock {
        fn send(&self, req: &Request) -> std::result::Result<HttpResponse, String> {
            self.seen.borrow_mut().push(req.clone());
            self.replies
                .borrow_mut()
                .pop_front()
                .expect("unexpected request")
        }
    }

    pub fn resp(status: u16, body: &str) -> std::result::Result<HttpResponse, String> {
        Ok(HttpResponse {
            status,
            retry_after: None,
            body: body.into(),
        })
    }

    pub fn client(
        replies: Vec<std::result::Result<HttpResponse, String>>,
    ) -> (Client<Mock>, Seen, Slept) {
        client_with(replies, |c| c)
    }

    pub fn client_with(
        replies: Vec<std::result::Result<HttpResponse, String>>,
        f: impl FnOnce(Client<Mock>) -> Client<Mock>,
    ) -> (Client<Mock>, Seen, Slept) {
        let seen: Seen = Rc::default();
        let slept: Slept = Rc::default();
        let s2 = slept.clone();
        let c = Client::new(
            Mock {
                replies: RefCell::new(replies.into()),
                seen: seen.clone(),
            },
            Credentials::new("CID", "KEY"),
        )
        .base_url("https://example.test/")
        .max_retries(3)
        .page_limit(2)
        .with_clock(
            Box::new(move |d| s2.borrow_mut().push(d)),
            Box::new(|| 1700000000),
        );
        (f(c), seen, slept)
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::*;
    use super::*;
    use serde_json::json;

    #[test]
    fn signs_every_request_including_prefix() {
        let (c, seen, _) = client(vec![resp(200, "[]")]);
        c.get("/accounts", &[]).unwrap();
        let r = seen.borrow()[0].clone();
        assert_eq!(
            r.url,
            "https://example.test/api/v1/accounts?clientId=CID&timestamp=1700000000"
        );
        assert_eq!(
            r.signature,
            signature(
                "KEY",
                "/api/v1/accounts",
                "clientId=CID&timestamp=1700000000",
                None
            )
        );
    }

    #[test]
    fn post_body_is_signed_and_sent() {
        let (c, seen, _) = client(vec![resp(200, "{}")]);
        let body = json!({"substring": "AAPL"});
        c.request(Method::Post, "/symbols", &[], Some(&body))
            .unwrap();
        let r = seen.borrow()[0].clone();
        assert_eq!(r.body.as_deref(), Some(r#"{"substring":"AAPL"}"#));
        assert_eq!(
            r.signature,
            signature(
                "KEY",
                "/api/v1/symbols",
                "clientId=CID&timestamp=1700000000",
                Some(&body)
            )
        );
    }

    #[test]
    fn custom_prefix_is_signed() {
        let (c, seen, _) = client_with(vec![resp(200, "[]")], |c| c.api_prefix(""));
        c.get("/accounts", &[]).unwrap();
        assert!(seen.borrow()[0]
            .url
            .starts_with("https://example.test/accounts?"));
    }

    #[test]
    fn retries_429_and_5xx_with_backoff_on_get() {
        let (c, seen, slept) = client(vec![resp(429, "slow"), resp(503, "x"), resp(200, "[]")]);
        c.get("/authorizations", &[]).unwrap();
        assert_eq!(seen.borrow().len(), 3);
        assert_eq!(
            *slept.borrow(),
            vec![Duration::from_secs(1), Duration::from_secs(2)]
        );
    }

    #[test]
    fn fresh_timestamp_per_attempt() {
        use std::cell::Cell;
        use std::rc::Rc;
        let n = Rc::new(Cell::new(100));
        let n2 = n.clone();
        let (c, seen, _) = client(vec![resp(429, ""), resp(200, "[]")]);
        let c = c.with_clock(
            Box::new(|_| {}),
            Box::new(move || {
                n2.set(n2.get() + 1);
                n2.get()
            }),
        );
        c.get("/accounts", &[]).unwrap();
        let s = seen.borrow();
        assert!(s[0].url.contains("timestamp=101") && s[1].url.contains("timestamp=102"));
        assert_ne!(s[0].signature, s[1].signature);
    }

    #[test]
    fn honors_retry_after_and_caps() {
        assert_eq!(backoff_delay(0, Some(7)), Duration::from_secs(7));
        assert_eq!(backoff_delay(0, Some(900)), Duration::from_secs(60));
        assert_eq!(backoff_delay(20, None), Duration::from_secs(60));
    }

    #[test]
    fn rate_limit_error_after_retries_exhausted() {
        let (c, seen, _) = client(vec![
            resp(429, "a"),
            resp(429, "b"),
            resp(429, "c"),
            resp(429, "d"),
        ]);
        let e = c.get("/accounts", &[]).unwrap_err();
        assert!(e.is_rate_limited());
        assert_eq!(e.status(), Some(429));
        assert_eq!(seen.borrow().len(), 4);
    }

    #[test]
    fn http_410_is_typed_and_not_retried() {
        let (c, seen, _) = client(vec![resp(410, "gone")]);
        let e = c.get("/accounts/x/positions", &[]).unwrap_err();
        assert!(e.is_not_available());
        assert_eq!(e.status(), Some(410));
        assert!(e.to_string().contains("not available for your account"));
        assert_eq!(seen.borrow().len(), 1);
    }

    #[test]
    fn auth_and_not_found_are_typed() {
        let (c, _, _) = client(vec![
            resp(401, "denied"),
            resp(404, "nope"),
            resp(400, "bad"),
        ]);
        assert!(matches!(
            c.get("/a", &[]).unwrap_err(),
            Error::Auth { status: 401, .. }
        ));
        assert!(matches!(
            c.get("/a", &[]).unwrap_err(),
            Error::NotFound { .. }
        ));
        assert!(matches!(
            c.get("/a", &[]).unwrap_err(),
            Error::Http { status: 400, .. }
        ));
    }

    #[test]
    fn post_is_not_retried_on_5xx_or_network_error() {
        let (c, seen, _) = client(vec![resp(503, "x")]);
        let e = c
            .request(Method::Post, "/x", &[], Some(&json!({"a":1})))
            .unwrap_err();
        assert_eq!(e.status(), Some(503));
        assert_eq!(seen.borrow().len(), 1);
        let (c, seen, _) = client(vec![Err("boom".into())]);
        let e = c
            .request(Method::Post, "/x", &[], Some(&json!({"a":1})))
            .unwrap_err();
        assert!(matches!(e, Error::Network { .. }));
        assert_eq!(seen.borrow().len(), 1);
    }

    #[test]
    fn post_is_retried_on_429() {
        let (c, seen, _) = client(vec![resp(429, ""), resp(200, "{}")]);
        c.request(Method::Post, "/x", &[], Some(&json!({"a":1})))
            .unwrap();
        assert_eq!(seen.borrow().len(), 2);
    }

    #[test]
    fn network_error_retried_for_get_then_surfaced() {
        let (c, seen, _) = client(vec![
            Err("a".into()),
            Err("b".into()),
            Err("c".into()),
            Err("secret-url-free".into()),
        ]);
        let e = c.get("/accounts", &[]).unwrap_err();
        assert!(matches!(e, Error::Network { .. }));
        assert_eq!(seen.borrow().len(), 4);
    }

    #[test]
    fn errors_never_leak_credentials_or_url() {
        let (c, _, _) = client(vec![resp(401, "denied")]);
        let e = c.get("/accounts", &[]).unwrap_err().to_string();
        assert!(!e.contains("KEY") && !e.contains("CID") && !e.contains("example.test"));
    }

    #[test]
    fn empty_body_is_null_and_bad_json_is_decode_error() {
        let (c, _, _) = client(vec![resp(204, ""), resp(200, "<html>")]);
        assert_eq!(c.get("/a", &[]).unwrap(), Value::Null);
        assert!(matches!(
            c.get("/a", &[]).unwrap_err(),
            Error::Decode { .. }
        ));
    }

    #[test]
    fn paginate_follows_total_and_stops_on_empty() {
        let p1 = r#"{"data":[{"id":"1"},{"id":"2"}],"pagination":{"total":3}}"#;
        let p2 = r#"{"data":[{"id":"3"}],"pagination":{"total":3}}"#;
        let (c, seen, _) = client(vec![resp(200, p1), resp(200, p2)]);
        let all = c
            .paginate(
                "/accounts/a/activities",
                &[("startDate", "2024-01-01".into())],
            )
            .unwrap();
        assert_eq!(all.len(), 3);
        let s = seen.borrow();
        assert!(s[0].url.contains("limit=2&offset=0&startDate=2024-01-01"));
        assert!(s[1].url.contains("offset=2"));
        let (c, _, _) = client(vec![resp(200, r#"{"data":[],"pagination":{"total":99}}"#)]);
        assert!(c.paginate("/x", &[]).unwrap().is_empty());
    }
}
