//! CSV/JSON export of activities. Account numbers are masked everywhere.

use crate::mask::mask_number;
use crate::models::{Account, Activity, Decimal};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Output format for exports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Csv,
    Json,
    Both,
}

impl Format {
    /// Parse `csv`, `json` or `both` (case-insensitive).
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "csv" => Ok(Self::Csv),
            "json" => Ok(Self::Json),
            "both" => Ok(Self::Both),
            other => Err(format!("unknown format '{other}' (csv|json|both)")),
        }
    }
    pub fn csv(self) -> bool {
        matches!(self, Self::Csv | Self::Both)
    }
    pub fn json(self) -> bool {
        matches!(self, Self::Json | Self::Both)
    }
}

/// Make a string safe as one path component.
pub fn sanitize_component(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let t = cleaned.trim().trim_matches('.');
    if t.is_empty() {
        "Unknown".into()
    } else {
        t.into()
    }
}

pub const CSV_HEADER: [&str; 15] = [
    "account_id",
    "account_number_masked",
    "institution",
    "activity_id",
    "trade_date",
    "settlement_date",
    "type",
    "symbol",
    "description",
    "amount",
    "currency",
    "units",
    "price",
    "fee",
    "external_reference_id",
];

fn dt(d: Option<DateTime<Utc>>) -> String {
    d.map(|d| d.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_default()
}

fn dec(d: &Option<Decimal>) -> String {
    d.as_ref().map(|d| d.to_string()).unwrap_or_default()
}

fn opt(s: &Option<String>) -> String {
    s.clone().unwrap_or_default()
}

fn symbol(a: &Activity) -> String {
    a.symbol
        .as_ref()
        .and_then(|s| s.symbol.clone())
        .or_else(|| a.option_symbol.as_ref().and_then(|o| o.ticker.clone()))
        .unwrap_or_default()
}

fn number(account: &Account) -> &str {
    account.number.as_deref().unwrap_or("")
}

/// One CSV row per activity, in `CSV_HEADER` order.
pub fn row(account: &Account, a: &Activity) -> Vec<String> {
    vec![
        account.id.to_string(),
        mask_number(number(account)),
        opt(&account.institution_name),
        a.id.as_ref().map(|i| i.to_string()).unwrap_or_default(),
        dt(a.trade_date),
        dt(a.settlement_date),
        a.activity_type
            .as_ref()
            .map(|t| t.to_string())
            .unwrap_or_default(),
        symbol(a),
        opt(&a.description),
        dec(&a.amount),
        a.currency
            .as_ref()
            .and_then(|c| c.code.clone())
            .unwrap_or_default(),
        dec(&a.units),
        dec(&a.price),
        dec(&a.fee),
        opt(&a.external_reference_id),
    ]
}

pub fn to_csv(account: &Account, activities: &[Activity]) -> Result<String, String> {
    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record(CSV_HEADER).map_err(|e| e.to_string())?;
    for a in activities {
        w.write_record(row(account, a)).map_err(|e| e.to_string())?;
    }
    let bytes = w.into_inner().map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// Account summary with the number masked; never includes the raw number or
/// the brokerage's opaque institution_account_id.
pub fn masked_account(account: &Account) -> Value {
    let tx = account
        .sync_status
        .as_ref()
        .and_then(|s| s.transactions.as_ref());
    json!({
        "id": account.id,
        "institution": account.institution_name,
        "name": account.name,
        "number_masked": mask_number(number(account)),
        "type": account.raw_type,
        "status": account.status,
        "first_transaction_date": tx.and_then(|t| t.first_transaction_date),
        "last_transaction_sync": tx.and_then(|t| t.last_successful_sync),
    })
}

pub fn to_json(account: &Account, activities: &[Activity]) -> Result<String, String> {
    serde_json::to_string_pretty(&json!({
        "account": masked_account(account),
        "count": activities.len(),
        "activities": activities,
    }))
    .map_err(|e| e.to_string())
}

/// File stem: `snaptrade_<institution>_<last4>_<from|start>_<to|latest>`.
pub fn file_stem(account: &Account, from: Option<&str>, to: Option<&str>) -> String {
    let inst = sanitize_component(account.institution_name.as_deref().unwrap_or("unknown"))
        .replace(' ', "-")
        .to_lowercase();
    let tail = mask_number(number(account)).replace('*', "").to_lowercase();
    format!(
        "snaptrade_{inst}_{}_{}_{}",
        if tail.is_empty() { "xxxx".into() } else { tail },
        from.unwrap_or("start"),
        to.unwrap_or("latest")
    )
}

/// Planned output files (no I/O).
pub fn planned_paths(dir: &Path, stem: &str, format: Format) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if format.csv() {
        v.push(dir.join(format!("{stem}.csv")));
    }
    if format.json() {
        v.push(dir.join(format!("{stem}.json")));
    }
    v
}

pub fn write_exports(
    dir: &Path,
    stem: &str,
    format: Format,
    account: &Account,
    activities: &[Activity],
) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let paths = planned_paths(dir, stem, format);
    for p in &paths {
        let body = if p.extension().is_some_and(|e| e == "csv") {
            to_csv(account, activities)?
        } else {
            to_json(account, activities)?
        };
        std::fs::write(p, body).map_err(|e| format!("write {}: {e}", p.display()))?;
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> Account {
        serde_json::from_value(json!({"id":"acc-1","institution_name":"Charles Schwab","name":"Brokerage","number":"12345678","institution_account_id":"OPAQUE"})).unwrap()
    }

    fn acts() -> Vec<Activity> {
        serde_json::from_value(json!([
            {"id":"x1","trade_date":"2026-10-05T00:00:00Z","type":"BUY","symbol":{"symbol":"AAPL"},"description":"Buy, \"AAPL\"","amount":-10.5,"currency":{"code":"USD"},"units":1.0,"price":10.5,"fee":0.0,"external_reference_id":"ref"},
            {"id":"x2","type":"INTEREST","symbol":null,"amount":0.04,"currency":{"code":"USD"}}
        ])).unwrap()
    }

    #[test]
    fn csv_has_header_masked_number_and_quoting() {
        let s = to_csv(&account(), &acts()).unwrap();
        let mut lines = s.lines();
        assert_eq!(lines.next().unwrap(), CSV_HEADER.join(","));
        let first = lines.next().unwrap();
        assert!(first.contains("****5678") && first.contains("AAPL"));
        assert!(first.contains("\"Buy, \"\"AAPL\"\"\""));
        assert!(!s.contains("12345678") && !s.contains("OPAQUE"));
    }

    #[test]
    fn json_is_masked() {
        let s = to_json(&account(), &acts()).unwrap();
        assert!(s.contains("****5678") && !s.contains("12345678") && !s.contains("OPAQUE"));
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["count"], 2);
    }

    #[test]
    fn stem_and_paths() {
        let st = file_stem(&account(), None, Some("2026-01-31"));
        assert_eq!(st, "snaptrade_charles-schwab_5678_start_2026-01-31");
        let p = planned_paths(Path::new("/d"), &st, Format::Both);
        assert_eq!(p.len(), 2);
        assert_eq!(planned_paths(Path::new("/d"), &st, Format::Json).len(), 1);
    }

    #[test]
    fn writes_files() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("a/b");
        let p = write_exports(&out, "s", Format::Both, &account(), &acts()).unwrap();
        assert_eq!(p.len(), 2);
        assert!(std::fs::read_to_string(&p[0])
            .unwrap()
            .starts_with("account_id,"));
    }
}
