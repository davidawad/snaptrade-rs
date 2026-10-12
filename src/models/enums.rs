//! String enumerations with an `Unknown` fallback.

/// Defines a `#[non_exhaustive]` string enum with `Unknown(String)`.
macro_rules! string_enum {
    ($(#[$m:meta])* $vis:vis enum $name:ident { $($(#[$vm:meta])* $var:ident => $s:literal),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        $vis enum $name {
            $($(#[$vm])* $var,)*
            /// A value this crate does not know yet, kept verbatim.
            Unknown(String),
        }

        impl $name {
            /// The wire value.
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$var => $s,)*
                    Self::Unknown(s) => s,
                }
            }
            /// True for [`Self::Unknown`].
            pub fn is_unknown(&self) -> bool {
                matches!(self, Self::Unknown(_))
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                match s {
                    $($s => Self::$var,)*
                    other => Self::Unknown(other.to_string()),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = <std::borrow::Cow<'de, str> as serde::Deserialize>::deserialize(d)?;
                Ok(Self::from(s.as_ref()))
            }
        }
    };
}

#[cfg(feature = "trading")]
pub(crate) use string_enum;

string_enum! {
    /// Kind of an account activity (transaction).
    pub enum ActivityType {
        Buy => "BUY",
        Sell => "SELL",
        Dividend => "DIVIDEND",
        StockDividend => "STOCK_DIVIDEND",
        Reinvestment => "REI",
        Interest => "INTEREST",
        Fee => "FEE",
        Tax => "TAX",
        Contribution => "CONTRIBUTION",
        Withdrawal => "WITHDRAWAL",
        Transfer => "TRANSFER",
        Split => "SPLIT",
        Adjustment => "ADJUSTMENT",
        OptionExpiration => "OPTION_EXPIRATION",
        OptionAssignment => "OPTION_ASSIGNMENT",
        OptionExercise => "OPTION_EXERCISE",
    }
}

string_enum! {
    /// Call or put.
    pub enum OptionType {
        Call => "CALL",
        Put => "PUT",
    }
}

string_enum! {
    /// Access level of a brokerage connection.
    pub enum ConnectionType {
        Read => "read",
        Trade => "trade",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_and_unknown_round_trip() {
        let t: ActivityType = serde_json::from_str("\"DIVIDEND\"").unwrap();
        assert_eq!(t, ActivityType::Dividend);
        let u: ActivityType = serde_json::from_str("\"BRAND_NEW\"").unwrap();
        assert_eq!(u, ActivityType::Unknown("BRAND_NEW".into()));
        assert!(u.is_unknown());
        assert_eq!(serde_json::to_string(&u).unwrap(), "\"BRAND_NEW\"");
        assert_eq!(serde_json::to_string(&t).unwrap(), "\"DIVIDEND\"");
    }
}
