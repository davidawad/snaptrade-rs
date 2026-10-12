//! Authentication and user management, plus API status.
//!
//! These calls are for standard (partner) API keys that manage their own
//! SnapTrade users. A Personal key has a single implicit user and cannot
//! register or delete users.

use crate::client::{from_value, list_from_value, Client};
use crate::error::{Error, Result};
use crate::models::{ApiStatus, LoginRedirect, UserId, UserRegistration, UserSecret};
use crate::transport::{Method, Transport};
use serde::Serialize;
use serde_json::{json, Value};

/// Options for [`Client::login`] (Connection Portal URL). All optional.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginOptions {
    /// Brokerage slug to preselect, e.g. `ALPACA`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub immediate_redirect: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_redirect: Option<String>,
    /// Connection id to reconnect (fixes a disabled connection).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconnect: Option<String>,
    /// `read`, `trade` or `trade-if-available`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_close_button: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dark_mode: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

impl<T: Transport> Client<T> {
    /// `GET /` : API status.
    pub fn api_status(&self) -> Result<ApiStatus> {
        from_value("/", self.get("/", &[])?)
    }

    /// `POST /snapTrade/registerUser`: create a SnapTrade user; the response
    /// holds the `userSecret`. Store it securely and never log it.
    pub fn register_user(&self, user_id: impl Into<UserId>) -> Result<UserRegistration> {
        from_value(
            "/snapTrade/registerUser",
            self.request(
                Method::Post,
                "/snapTrade/registerUser",
                &[],
                Some(&json!({ "userId": user_id.into() })),
            )?,
        )
    }

    /// `POST /snapTrade/login`: Connection Portal URL for the attached user.
    pub fn login(&self, opts: &LoginOptions) -> Result<LoginRedirect> {
        let body = serde_json::to_value(opts).map_err(|e| Error::InvalidArgument(e.to_string()))?;
        from_value(
            "/snapTrade/login",
            self.request(
                Method::Post,
                "/snapTrade/login",
                &self.scoped(&[]),
                Some(&body),
            )?,
        )
    }

    /// `GET /snapTrade/listUsers`: ids of all users under this API key.
    pub fn list_users(&self) -> Result<Vec<UserId>> {
        list_from_value(
            "/snapTrade/listUsers",
            self.get("/snapTrade/listUsers", &[])?,
        )
    }

    /// `DELETE /snapTrade/deleteUser`: deletes the user and all their
    /// connections. Irreversible. Returns the raw JSON.
    pub fn delete_user(&self, user_id: impl Into<UserId>) -> Result<Value> {
        self.request(
            Method::Delete,
            "/snapTrade/deleteUser",
            &[("userId", user_id.into().into_inner())],
            None,
        )
    }

    /// `POST /snapTrade/resetUserSecret`: rotate a user's secret. Returns the
    /// raw JSON (it holds the new secret).
    pub fn reset_user_secret(
        &self,
        user_id: impl Into<UserId>,
        user_secret: &UserSecret,
    ) -> Result<Value> {
        self.request(
            Method::Post,
            "/snapTrade/resetUserSecret",
            &[],
            Some(&json!({ "userId": user_id.into(), "userSecret": user_secret })),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::client::testutil::*;
    use crate::credentials::UserAuth;
    use serde_json::json;

    #[test]
    fn register_posts_user_id() {
        let (c, seen, _) = client(vec![resp(200, r#"{"userId":"u","userSecret":"s"}"#)]);
        let v = c.register_user("u").unwrap();
        assert_eq!(v.user_id.as_str(), "u");
        assert_eq!(v.user_secret.expose(), "s");
        let r = seen.borrow()[0].clone();
        assert!(r.url.contains("/api/v1/snapTrade/registerUser?"));
        assert_eq!(r.body.as_deref(), Some(r#"{"userId":"u"}"#));
    }

    #[test]
    fn login_sends_user_in_query_and_options_in_body() {
        let (c, seen, _) =
            client_with(vec![resp(200, "{}")], |c| c.user(UserAuth::new("u1", "s1")));
        c.login(&super::LoginOptions {
            broker: Some("ALPACA".into()),
            dark_mode: Some(true),
            ..Default::default()
        })
        .unwrap();
        let r = seen.borrow()[0].clone();
        assert!(r.url.contains("userId=u1&userSecret=s1"));
        assert_eq!(
            r.body.as_deref(),
            Some(r#"{"broker":"ALPACA","darkMode":true}"#)
        );
    }

    #[test]
    fn list_delete_reset() {
        let (c, seen, _) = client(vec![
            resp(200, r#"["a","b"]"#),
            resp(200, "{}"),
            resp(200, "{}"),
        ]);
        assert_eq!(c.list_users().unwrap().len(), 2);
        c.delete_user("a").unwrap();
        c.reset_user_secret("a", &crate::models::UserSecret::new("old"))
            .unwrap();
        let s = seen.borrow();
        assert!(s[1]
            .url
            .contains("deleteUser?clientId=CID&timestamp=1700000000&userId=a"));
        assert_eq!(
            s[2].body.as_deref(),
            Some(
                json!({"userId":"a","userSecret":"old"})
                    .to_string()
                    .as_str()
            )
        );
    }
}
