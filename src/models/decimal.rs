//! A string-backed decimal for money, prices and quantities.
//!
//! SnapTrade sends these as JSON numbers (and sometimes strings). The text is
//! kept exactly as received rather than squeezed through `f64`, so `10.50`
//! stays `10.50`. Equality compares the text: `1.5` and `1.50` are different
//! values; compare [`Decimal::to_f64`] if you need numeric equality. When
//! serialised it is emitted as a JSON number (what the API expects).

use std::fmt;
use std::str::FromStr;

/// Decimal number kept as text. Construct with [`FromStr`], `From<i64>` or
/// `TryFrom<f64>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Decimal(String);

/// Not a decimal number.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not a decimal number")]
pub struct ParseDecimalError;

fn valid(s: &str) -> bool {
    let t = s.strip_prefix(['-', '+']).unwrap_or(s);
    let (mant, exp) = match t.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (t, None),
    };
    let (int, frac) = mant.split_once('.').unwrap_or((mant, ""));
    let digits = |x: &str| x.bytes().all(|b| b.is_ascii_digit());
    let exp_ok = match exp {
        None => true,
        Some(e) => {
            let e = e.strip_prefix(['-', '+']).unwrap_or(e);
            !e.is_empty() && digits(e)
        }
    };
    digits(int) && digits(frac) && !(int.is_empty() && frac.is_empty()) && exp_ok
}

impl Decimal {
    /// The number as received.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Lossy conversion for arithmetic or comparison.
    pub fn to_f64(&self) -> Option<f64> {
        self.0.parse().ok()
    }
    pub fn is_negative(&self) -> bool {
        self.0.starts_with('-')
    }
}

impl FromStr for Decimal {
    type Err = ParseDecimalError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if valid(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(ParseDecimalError)
        }
    }
}

impl TryFrom<f64> for Decimal {
    type Error = ParseDecimalError;
    fn try_from(f: f64) -> Result<Self, Self::Error> {
        if f.is_finite() {
            Ok(Self(f.to_string()))
        } else {
            Err(ParseDecimalError)
        }
    }
}

impl From<i64> for Decimal {
    fn from(i: i64) -> Self {
        Self(i.to_string())
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl serde::Serialize for Decimal {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.to_f64() {
            Some(f) => s.serialize_f64(f),
            None => Err(serde::ser::Error::custom("decimal out of range")),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Decimal {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl serde::de::Visitor<'_> for V {
            type Value = Decimal;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a number or a numeric string")
            }
            fn visit_i64<E>(self, v: i64) -> Result<Decimal, E> {
                Ok(Decimal(v.to_string()))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Decimal, E> {
                Ok(Decimal(v.to_string()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Decimal, E> {
                Decimal::try_from(v).map_err(|_| E::custom("non-finite number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Decimal, E> {
                v.parse().map_err(|_| E::custom("not a decimal number"))
            }
        }
        d.deserialize_any(V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbers_and_strings_keeping_text() {
        let d: Decimal = serde_json::from_str("10.5").unwrap();
        assert_eq!(d.as_str(), "10.5");
        let d: Decimal = serde_json::from_str("\"10.50\"").unwrap();
        assert_eq!(d.as_str(), "10.50");
        let d: Decimal = serde_json::from_str("-3").unwrap();
        assert!(d.is_negative());
        assert_eq!(d.to_f64(), Some(-3.0));
    }

    #[test]
    fn rejects_garbage() {
        assert!("abc".parse::<Decimal>().is_err());
        assert!("".parse::<Decimal>().is_err());
        assert!("1.2.3".parse::<Decimal>().is_err());
        assert!("1e".parse::<Decimal>().is_err());
        assert!("1e5".parse::<Decimal>().is_ok());
        assert!(serde_json::from_str::<Decimal>("true").is_err());
        assert!(Decimal::try_from(f64::NAN).is_err());
    }

    #[test]
    fn serialises_as_json_number() {
        let d: Decimal = "10.5".parse().unwrap();
        assert_eq!(serde_json::to_string(&d).unwrap(), "10.5");
    }
}
