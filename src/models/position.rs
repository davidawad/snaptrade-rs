//! Positions (holdings) and option positions.

use super::{Currency, Decimal, Extra, OptionSymbol, Symbol};
use serde::{Deserialize, Serialize};

/// The instrument held: a [`Symbol`] plus the brokerage's own id/description.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PositionSymbol {
    #[serde(default)]
    pub symbol: Option<Symbol>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A stock, ETF, mutual fund or crypto position.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Position {
    #[serde(default)]
    pub symbol: Option<PositionSymbol>,
    #[serde(default)]
    pub units: Option<Decimal>,
    #[serde(default)]
    pub fractional_units: Option<Decimal>,
    #[serde(default)]
    pub price: Option<Decimal>,
    #[serde(default)]
    pub open_pnl: Option<Decimal>,
    #[serde(default)]
    pub average_purchase_price: Option<Decimal>,
    #[serde(default)]
    pub currency: Option<Currency>,
    #[serde(default)]
    pub cash_equivalent: Option<bool>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OptionPositionSymbol {
    #[serde(default)]
    pub option_symbol: Option<OptionSymbol>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// An option holding.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OptionPosition {
    #[serde(default)]
    pub symbol: Option<OptionPositionSymbol>,
    #[serde(default)]
    pub units: Option<Decimal>,
    #[serde(default)]
    pub price: Option<Decimal>,
    #[serde(default)]
    pub average_purchase_price: Option<Decimal>,
    #[serde(default)]
    pub currency: Option<Currency>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Response of the all-instrument positions endpoint. A bare array is read as
/// `positions`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AllPositions {
    pub positions: Vec<Position>,
    pub option_positions: Vec<OptionPosition>,
    #[serde(flatten)]
    pub extra: Extra,
}

impl<'de> Deserialize<'de> for AllPositions {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        #[derive(Deserialize)]
        struct Obj {
            #[serde(default)]
            positions: Vec<Position>,
            #[serde(default)]
            option_positions: Vec<OptionPosition>,
            #[serde(flatten)]
            extra: Extra,
        }
        match serde_json::Value::deserialize(d)? {
            v @ serde_json::Value::Array(_) => Ok(Self {
                positions: serde_json::from_value(v).map_err(D::Error::custom)?,
                ..Self::default()
            }),
            v => {
                let o: Obj = serde_json::from_value(v).map_err(D::Error::custom)?;
                Ok(Self {
                    positions: o.positions,
                    option_positions: o.option_positions,
                    extra: o.extra,
                })
            }
        }
    }
}
