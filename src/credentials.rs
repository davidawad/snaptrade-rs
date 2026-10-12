//! Credentials and pluggable credential providers.
//!
//! The library never assumes where secrets live. Implement [`CredentialProvider`]
//! or use one of the bundled providers: [`EnvProvider`], [`CommandProvider`],
//! [`FileProvider`], [`StaticProvider`].

use crate::error::{Error, Result};
use std::fmt;
use std::path::PathBuf;
use std::process::Command;

/// API-key credentials from the SnapTrade dashboard.
#[derive(Clone)]
pub struct Credentials {
    pub client_id: String,
    pub consumer_key: String,
}

impl Credentials {
    pub fn new(client_id: impl Into<String>, consumer_key: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            consumer_key: consumer_key.into(),
        }
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credentials(<redacted>)")
    }
}

/// A registered SnapTrade user (`userId` + `userSecret`). Personal API keys do
/// not have one; leave it unset.
#[derive(Clone)]
pub struct UserAuth {
    pub user_id: String,
    pub user_secret: String,
}

impl UserAuth {
    pub fn new(user_id: impl Into<String>, user_secret: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            user_secret: user_secret.into(),
        }
    }
}

impl fmt::Debug for UserAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UserAuth(<redacted>)")
    }
}

/// Source of credentials.
pub trait CredentialProvider {
    /// Load the API key pair.
    fn credentials(&self) -> Result<Credentials>;

    /// Load the end-user pair, if this provider has one. Default: none (the
    /// Personal-key case).
    fn user(&self) -> Result<Option<UserAuth>> {
        Ok(None)
    }
}

fn nonempty(label: &str, v: String) -> Result<String> {
    let t = v.trim().to_string();
    if t.is_empty() {
        Err(Error::Credentials(format!("{label} is empty")))
    } else {
        Ok(t)
    }
}

/// Fixed in-memory values (tests, or secrets you already hold).
#[derive(Clone, Debug)]
pub struct StaticProvider {
    creds: Credentials,
    user: Option<UserAuth>,
}

impl StaticProvider {
    pub fn new(creds: Credentials) -> Self {
        Self { creds, user: None }
    }
    pub fn with_user(mut self, user: UserAuth) -> Self {
        self.user = Some(user);
        self
    }
}

impl CredentialProvider for StaticProvider {
    fn credentials(&self) -> Result<Credentials> {
        Ok(self.creds.clone())
    }
    fn user(&self) -> Result<Option<UserAuth>> {
        Ok(self.user.clone())
    }
}

/// Reads `SNAPTRADE_CLIENT_ID`, `SNAPTRADE_CONSUMER_KEY` and, optionally,
/// `SNAPTRADE_USER_ID` + `SNAPTRADE_USER_SECRET`. A different prefix can be set
/// with [`EnvProvider::with_prefix`].
#[derive(Clone, Debug)]
pub struct EnvProvider {
    prefix: String,
}

impl Default for EnvProvider {
    fn default() -> Self {
        Self {
            prefix: "SNAPTRADE_".into(),
        }
    }
}

impl EnvProvider {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(format!("{}{}", self.prefix, name)).ok()
    }
}

impl CredentialProvider for EnvProvider {
    fn credentials(&self) -> Result<Credentials> {
        let need = |n: &str| {
            self.var(n)
                .ok_or_else(|| Error::Credentials(format!("{}{} is not set", self.prefix, n)))
                .and_then(|v| nonempty(&format!("{}{}", self.prefix, n), v))
        };
        Ok(Credentials::new(need("CLIENT_ID")?, need("CONSUMER_KEY")?))
    }
    fn user(&self) -> Result<Option<UserAuth>> {
        match (self.var("USER_ID"), self.var("USER_SECRET")) {
            (Some(i), Some(s)) if !i.trim().is_empty() && !s.trim().is_empty() => {
                Ok(Some(UserAuth::new(i.trim(), s.trim())))
            }
            _ => Ok(None),
        }
    }
}

/// Runs an external program once per field and uses its trimmed stdout. Any
/// argument equal to `{field}` is replaced with the field name
/// (`client_id`, `consumer_key`, `user_id`, `user_secret`), so a password
/// manager or secret store CLI can be plugged in without code:
///
/// ```no_run
/// use snaptrade_rs::credentials::CommandProvider;
/// let p = CommandProvider::new("my-secret-tool", ["get", "snaptrade", "--field", "{field}"]);
/// ```
///
/// The program is run directly (no shell). Its stderr is discarded and never
/// included in error messages.
#[derive(Clone, Debug)]
pub struct CommandProvider {
    program: String,
    args: Vec<String>,
    with_user: bool,
    field_names: std::collections::HashMap<String, String>,
}

impl CommandProvider {
    pub fn new<I, S>(program: impl Into<String>, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            with_user: false,
            field_names: std::collections::HashMap::new(),
        }
    }

    /// Rename a field as the external tool knows it, e.g.
    /// `.rename_field("client_id", "username")`.
    pub fn rename_field(mut self, canonical: &str, tool_name: &str) -> Self {
        self.field_names.insert(canonical.into(), tool_name.into());
        self
    }

    /// Also fetch `user_id` / `user_secret` (off by default: Personal keys have none).
    pub fn with_user(mut self) -> Self {
        self.with_user = true;
        self
    }

    fn fetch(&self, field: &str) -> Result<String> {
        let shown = self.field_names.get(field).map_or(field, String::as_str);
        let args: Vec<String> = self
            .args
            .iter()
            .map(|a| {
                if a == "{field}" {
                    shown.to_string()
                } else {
                    a.clone()
                }
            })
            .collect();
        let out = Command::new(&self.program)
            .args(&args)
            .stderr(std::process::Stdio::null())
            .output()
            .map_err(|e| Error::Credentials(format!("run {}: {e}", self.program)))?;
        if !out.status.success() {
            return Err(Error::Credentials(format!(
                "{} failed for field {field} (exit {:?})",
                self.program,
                out.status.code()
            )));
        }
        nonempty(field, String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

impl CredentialProvider for CommandProvider {
    fn credentials(&self) -> Result<Credentials> {
        Ok(Credentials::new(
            self.fetch("client_id")?,
            self.fetch("consumer_key")?,
        ))
    }
    fn user(&self) -> Result<Option<UserAuth>> {
        if !self.with_user {
            return Ok(None);
        }
        Ok(Some(UserAuth::new(
            self.fetch("user_id")?,
            self.fetch("user_secret")?,
        )))
    }
}

/// Reads a file of `KEY=VALUE` lines (`#` comments and blank lines ignored;
/// optional surrounding quotes stripped) with keys `SNAPTRADE_CLIENT_ID`,
/// `SNAPTRADE_CONSUMER_KEY` and optionally `SNAPTRADE_USER_ID`,
/// `SNAPTRADE_USER_SECRET`. Or a JSON object with keys `clientId`,
/// `consumerKey`, `userId`, `userSecret` when the file starts with `{`.
#[derive(Clone, Debug)]
pub struct FileProvider {
    path: PathBuf,
}

impl FileProvider {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn read(&self) -> Result<std::collections::HashMap<String, String>> {
        let text = std::fs::read_to_string(&self.path).map_err(|e| {
            Error::Credentials(format!("read {}: {}", self.path.display(), e.kind()))
        })?;
        parse_credential_text(&text)
    }
}

/// Parse the two file formats [`FileProvider`] accepts into a flat map keyed by
/// canonical names `client_id`, `consumer_key`, `user_id`, `user_secret`.
pub fn parse_credential_text(text: &str) -> Result<std::collections::HashMap<String, String>> {
    let mut map = std::collections::HashMap::new();
    if text.trim_start().starts_with('{') {
        let v: serde_json::Value = serde_json::from_str(text)
            .map_err(|_| Error::Credentials("credential file is not valid JSON".into()))?;
        for (json_key, name) in [
            ("clientId", "client_id"),
            ("consumerKey", "consumer_key"),
            ("userId", "user_id"),
            ("userSecret", "user_secret"),
        ] {
            if let Some(s) = v[json_key].as_str() {
                map.insert(name.to_string(), s.to_string());
            }
        }
    } else {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let line = line.strip_prefix("export ").unwrap_or(line);
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
                let name = match k.trim() {
                    "SNAPTRADE_CLIENT_ID" => "client_id",
                    "SNAPTRADE_CONSUMER_KEY" => "consumer_key",
                    "SNAPTRADE_USER_ID" => "user_id",
                    "SNAPTRADE_USER_SECRET" => "user_secret",
                    _ => continue,
                };
                map.insert(name.to_string(), v.to_string());
            }
        }
    }
    Ok(map)
}

impl CredentialProvider for FileProvider {
    fn credentials(&self) -> Result<Credentials> {
        let m = self.read()?;
        let get = |k: &str| {
            m.get(k)
                .cloned()
                .ok_or_else(|| Error::Credentials(format!("{k} missing in credential file")))
                .and_then(|v| nonempty(k, v))
        };
        Ok(Credentials::new(get("client_id")?, get("consumer_key")?))
    }
    fn user(&self) -> Result<Option<UserAuth>> {
        let m = self.read()?;
        match (m.get("user_id"), m.get("user_secret")) {
            (Some(i), Some(s)) if !i.trim().is_empty() && !s.trim().is_empty() => {
                Ok(Some(UserAuth::new(i.trim(), s.trim())))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let c = Credentials::new("cid-value", "key-value");
        assert!(!format!("{c:?}").contains("value"));
        let u = UserAuth::new("u-value", "s-value");
        assert!(!format!("{u:?}").contains("value"));
    }

    #[test]
    fn env_provider_reads_prefixed_vars() {
        std::env::set_var("STTEST_A_CLIENT_ID", " cid ");
        std::env::set_var("STTEST_A_CONSUMER_KEY", "key");
        let p = EnvProvider::with_prefix("STTEST_A_");
        let c = p.credentials().unwrap();
        assert_eq!(
            (c.client_id.as_str(), c.consumer_key.as_str()),
            ("cid", "key")
        );
        assert!(p.user().unwrap().is_none());
        std::env::set_var("STTEST_A_USER_ID", "u");
        std::env::set_var("STTEST_A_USER_SECRET", "s");
        assert!(p.user().unwrap().is_some());
    }

    #[test]
    fn env_provider_missing_is_error_naming_the_variable() {
        let e = EnvProvider::with_prefix("STTEST_NONE_")
            .credentials()
            .unwrap_err();
        assert!(e.to_string().contains("STTEST_NONE_CLIENT_ID"));
    }

    #[test]
    fn command_provider_substitutes_field() {
        let p = CommandProvider::new("echo", ["val-{field}"]);
        // "val-{field}" is not exactly "{field}", so it is passed through.
        assert_eq!(p.fetch("client_id").unwrap(), "val-{field}");
        let p = CommandProvider::new("echo", ["{field}"]);
        let renamed = p.clone().rename_field("client_id", "username");
        assert_eq!(renamed.credentials().unwrap().client_id, "username");
        let c = p.credentials().unwrap();
        assert_eq!(c.client_id, "client_id");
        assert_eq!(c.consumer_key, "consumer_key");
        assert!(p.user().unwrap().is_none());
        let u = p.with_user().user().unwrap().unwrap();
        assert_eq!(u.user_id, "user_id");
    }

    #[test]
    fn command_provider_failure_is_typed_and_quiet() {
        let e = CommandProvider::new("false", Vec::<String>::new())
            .credentials()
            .unwrap_err();
        assert!(matches!(e, Error::Credentials(_)));
    }

    #[test]
    fn file_provider_env_and_json_formats() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("c.env");
        std::fs::write(
            &f,
            "# c\nexport SNAPTRADE_CLIENT_ID=\"abc\"\nSNAPTRADE_CONSUMER_KEY='k'\n",
        )
        .unwrap();
        let c = FileProvider::new(&f).credentials().unwrap();
        assert_eq!(
            (c.client_id.as_str(), c.consumer_key.as_str()),
            ("abc", "k")
        );
        let j = d.path().join("c.json");
        std::fs::write(
            &j,
            r#"{"clientId":"a","consumerKey":"b","userId":"u","userSecret":"s"}"#,
        )
        .unwrap();
        let p = FileProvider::new(&j);
        assert_eq!(p.credentials().unwrap().consumer_key, "b");
        assert_eq!(p.user().unwrap().unwrap().user_secret, "s");
        assert!(FileProvider::new(d.path().join("missing"))
            .credentials()
            .is_err());
    }
}
