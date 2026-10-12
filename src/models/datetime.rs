//! Tolerant date and timestamp parsing. SnapTrade mixes `2024-01-31`,
//! `2024-01-31T00:00:00Z` and offset-less forms depending on brokerage.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer};

/// RFC 3339, `YYYY-MM-DDTHH:MM:SS[.f]` (taken as UTC) or `YYYY-MM-DD`
/// (midnight UTC).
pub(crate) fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    if let Ok(n) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(Utc.from_utc_datetime(&n));
    }
    let d = NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?;
    Some(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?))
}

/// `YYYY-MM-DD`, or the date part of a timestamp.
pub(crate) fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .or_else(|| parse_datetime(s).map(|d| d.date_naive()))
}

pub(crate) fn opt_datetime<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<DateTime<Utc>>, D::Error> {
    match Option::<String>::deserialize(d)? {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => parse_datetime(&s)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("unrecognised timestamp format")),
    }
}

pub(crate) fn opt_date<'de, D: Deserializer<'de>>(d: D) -> Result<Option<NaiveDate>, D::Error> {
    match Option::<String>::deserialize(d)? {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => parse_date(&s)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("unrecognised date format")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_common_shapes() {
        assert!(parse_datetime("2024-01-31T10:00:00Z").is_some());
        assert!(parse_datetime("2024-01-31T10:00:00.123+02:00").is_some());
        assert!(parse_datetime("2024-01-31T10:00:00").is_some());
        assert!(parse_datetime("2024-01-31").is_some());
        assert!(parse_datetime("31/01/2024").is_none());
        assert_eq!(
            parse_date("2024-01-31T23:00:00Z"),
            NaiveDate::from_ymd_opt(2024, 1, 31)
        );
    }
}
