//! Currencies, exchanges and symbols shared by several responses.

use super::datetime::opt_date;
use super::{Decimal, Extra, OptionType, SymbolId};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// A currency (`{"id","code","name"}`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Currency {
    #[serde(default)]
    pub id: Option<String>,
    /// ISO-style code, e.g. `USD`.
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A securities exchange.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Exchange {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub mic_code: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Security type (`cs`, `et`, ...).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SecurityType {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_supported: Option<bool>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A universal symbol.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Symbol {
    #[serde(default)]
    pub id: Option<SymbolId>,
    /// Ticker, e.g. `AAPL`.
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub raw_symbol: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub currency: Option<Currency>,
    #[serde(default)]
    pub exchange: Option<Exchange>,
    #[serde(default, rename = "type")]
    pub security_type: Option<SecurityType>,
    #[serde(flatten)]
    pub extra: Extra,
}

impl Symbol {
    /// A symbol known only by ticker.
    pub fn from_ticker(ticker: impl Into<String>) -> Self {
        Self {
            symbol: Some(ticker.into()),
            ..Self::default()
        }
    }
}

/// Deserialise a symbol that some brokerages send as a bare ticker string.
pub(crate) fn symbol_or_ticker<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Symbol>, D::Error> {
    use serde::de::Error;
    match Option::<serde_json::Value>::deserialize(d)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(Symbol::from_ticker(s))),
        Some(v) => Symbol::deserialize(v).map(Some).map_err(D::Error::custom),
    }
}

/// An option contract.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OptionSymbol {
    #[serde(default)]
    pub id: Option<String>,
    /// OCC-style ticker.
    #[serde(default)]
    pub ticker: Option<String>,
    #[serde(default)]
    pub option_type: Option<OptionType>,
    #[serde(default)]
    pub strike_price: Option<Decimal>,
    #[serde(default, deserialize_with = "opt_date")]
    pub expiration_date: Option<NaiveDate>,
    #[serde(default)]
    pub is_mini_option: Option<bool>,
    #[serde(default)]
    pub underlying_symbol: Option<Symbol>,
    #[serde(flatten)]
    pub extra: Extra,
}
