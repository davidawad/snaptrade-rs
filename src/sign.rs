//! Request signing.
//!
//! SnapTrade authenticates `clientId`/`consumerKey` requests with a `Signature`
//! header: `base64(HMAC-SHA256(consumerKey, payload))`, where `payload` is the
//! canonical (key-sorted at every level, no whitespace) JSON object
//! `{"content":<body|null>,"path":<path incl. /api/v1>,"query":<raw query>}`.
//!
//! Canonicalisation is implemented here by hand rather than relying on
//! `serde_json`'s map ordering, because the `preserve_order` feature can be
//! switched on by any other crate in a dependency graph.

use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

/// Serialise `v` as compact JSON with object keys sorted at every level.
pub fn canonical_json(v: &Value) -> String {
    let mut out = String::new();
    write_canonical(v, &mut out);
    out
}

fn write_canonical(v: &Value, out: &mut String) {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(k.clone()).to_string());
                out.push(':');
                write_canonical(&m[k], out);
            }
            out.push('}');
        }
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(x, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// The exact string that gets signed. An absent or empty-object body signs as
/// `null`, as the SnapTrade docs require.
pub fn signing_payload(path: &str, query: &str, body: Option<&Value>) -> String {
    let content = match body {
        None => Value::Null,
        Some(Value::Object(m)) if m.is_empty() => Value::Null,
        Some(b) => b.clone(),
    };
    let payload = serde_json::json!({ "content": content, "path": path, "query": query });
    canonical_json(&payload)
}

/// Value of the `Signature` header.
pub fn signature(consumer_key: &str, path: &str, query: &str, body: Option<&Value>) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(consumer_key.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(signing_payload(path, query, body).as_bytes());
    STANDARD.encode(mac.finalize().into_bytes())
}

/// Percent-encode a query component (RFC 3986 unreserved set kept).
pub fn encode_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Build the query string: params plus `clientId` and `timestamp`, sorted by
/// key. The same string is signed and sent, byte for byte.
pub fn build_query(client_id: &str, timestamp: i64, params: &[(&str, String)]) -> String {
    let mut all: Vec<(String, String)> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    all.push(("clientId".into(), client_id.into()));
    all.push(("timestamp".into(), timestamp.to_string()));
    all.sort();
    all.iter()
        .map(|(k, v)| format!("{}={}", encode_component(k), encode_component(v)))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Vector computed independently with:
    //   printf '%s' '<payload>' | openssl dgst -sha256 -hmac testkey -binary | base64
    #[test]
    fn fixed_vector_get() {
        let q = "clientId=TEST-CLIENT&timestamp=1700000000";
        assert_eq!(
            signing_payload("/api/v1/accounts", q, None),
            r#"{"content":null,"path":"/api/v1/accounts","query":"clientId=TEST-CLIENT&timestamp=1700000000"}"#
        );
        assert_eq!(
            signature("testkey", "/api/v1/accounts", q, None),
            "RWaHLI8QQG8V+MUytKKbBY3Y8bGr+/CvrCdSDtdkqBc="
        );
    }

    #[test]
    fn body_payload_matches_docs_example() {
        let q = "clientId=YOUR_CLIENT_ID&timestamp=1715123456";
        assert_eq!(
            signing_payload("/api/v1/symbols", q, Some(&json!({"substring": "AAPL"}))),
            r#"{"content":{"substring":"AAPL"},"path":"/api/v1/symbols","query":"clientId=YOUR_CLIENT_ID&timestamp=1715123456"}"#
        );
    }

    #[test]
    fn empty_object_body_signs_as_null() {
        assert_eq!(
            signing_payload("/p", "q", Some(&json!({}))),
            signing_payload("/p", "q", None)
        );
    }

    #[test]
    fn nested_keys_are_sorted() {
        let v = json!({"b": {"z": 1, "a": [ {"y": 1, "x": 2} ]}, "a": "t\"q"});
        assert_eq!(
            canonical_json(&v),
            r#"{"a":"t\"q","b":{"a":[{"x":2,"y":1}],"z":1}}"#
        );
    }

    #[test]
    fn query_is_sorted_and_encoded() {
        let q = build_query(
            "cid",
            5,
            &[("startDate", "2024-01-01".into()), ("limit", "1 0".into())],
        );
        assert_eq!(
            q,
            "clientId=cid&limit=1%200&startDate=2024-01-01&timestamp=5"
        );
    }
}
