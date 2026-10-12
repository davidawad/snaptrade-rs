//! HTTP transport abstraction, so the client can be tested without a network.

use std::time::Duration;

/// HTTP methods SnapTrade uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Delete,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Delete => "DELETE",
        }
    }

    /// Whether a failed request may be repeated after a 5xx or network error
    /// without risking a duplicate side effect (e.g. a double-placed order).
    pub fn is_idempotent(self) -> bool {
        matches!(self, Method::Get | Method::Delete)
    }
}

/// A fully built, signed request.
#[derive(Clone, Debug)]
pub struct Request {
    pub method: Method,
    pub url: String,
    /// Value of the `Signature` header.
    pub signature: String,
    /// JSON body, already serialised.
    pub body: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    /// Parsed `Retry-After` header, in seconds.
    pub retry_after: Option<u64>,
    pub body: String,
}

/// Sends a [`Request`]. Implement this to use another HTTP stack or a mock.
pub trait Transport {
    fn send(&self, req: &Request) -> Result<HttpResponse, String>;
}

/// Default transport: blocking `reqwest` with rustls.
pub struct ReqwestTransport(reqwest::blocking::Client);

impl ReqwestTransport {
    pub fn new() -> Result<Self, String> {
        Self::with_timeout(Duration::from_secs(60))
    }

    pub fn with_timeout(timeout: Duration) -> Result<Self, String> {
        reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map(Self)
            .map_err(|e| format!("http client: {e}"))
    }
}

impl Transport for ReqwestTransport {
    fn send(&self, req: &Request) -> Result<HttpResponse, String> {
        let mut b = match req.method {
            Method::Get => self.0.get(&req.url),
            Method::Post => self.0.post(&req.url),
            Method::Delete => self.0.delete(&req.url),
        }
        .header("Signature", &req.signature)
        .header("Accept", "application/json");
        if let Some(body) = &req.body {
            b = b
                .header("Content-Type", "application/json")
                .body(body.clone());
        }
        // `without_url` keeps clientId (in the query string) out of error text.
        let r = b.send().map_err(|e| format!("{}", e.without_url()))?;
        let status = r.status().as_u16();
        let retry_after = r
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok());
        let body = r
            .text()
            .map_err(|e| format!("read body: {}", e.without_url()))?;
        Ok(HttpResponse {
            status,
            retry_after,
            body,
        })
    }
}
