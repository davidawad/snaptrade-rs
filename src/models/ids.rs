//! Newtype identifiers. Each wraps the opaque string SnapTrade returns, so an
//! account id cannot be passed where a connection id is expected.

use std::fmt;

macro_rules! id_type {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn into_inner(self) -> String {
                self.0
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
        impl From<&$name> for $name {
            fn from(s: &$name) -> Self {
                s.clone()
            }
        }
        impl std::str::FromStr for $name {
            type Err = std::convert::Infallible;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.to_string()))
            }
        }
    };
}

id_type!(
    /// A brokerage account (`id` on an account).
    AccountId
);
id_type!(
    /// A brokerage connection, also called a brokerage authorization.
    ConnectionId
);
id_type!(
    /// A SnapTrade user (the `userId` you chose at registration).
    UserId
);
id_type!(
    /// A universal symbol id.
    SymbolId
);
id_type!(
    /// An account activity (transaction) id.
    ActivityId
);
id_type!(
    /// A brokerage-assigned order id.
    BrokerageOrderId
);
id_type!(
    /// The id of a checked trade (from an order-impact call), valid for a few minutes.
    TradeId
);

/// A user's secret. Never printed: `Debug` is redacted and there is no
/// `Display`. Read it deliberately with [`UserSecret::expose`].
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct UserSecret(String);

impl UserSecret {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// The raw secret. Do not log it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for UserSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UserSecret(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_transparent_in_json() {
        let a: AccountId = serde_json::from_str("\"abc\"").unwrap();
        assert_eq!(a.as_str(), "abc");
        assert_eq!(serde_json::to_string(&a).unwrap(), "\"abc\"");
        assert_eq!(a.to_string(), "abc");
    }

    #[test]
    fn secret_is_redacted() {
        let s = UserSecret::new("hunter2");
        assert!(!format!("{s:?}").contains("hunter2"));
        assert_eq!(s.expose(), "hunter2");
    }
}
