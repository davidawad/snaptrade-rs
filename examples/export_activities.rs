//! Export one account's activities to CSV + JSON, with a credentials file and
//! handling of the Personal-key HTTP 410 case.
//!
//!   cargo run --example export_activities -- creds.env <account-id> ./out

use snaptrade_rs::credentials::FileProvider;
use snaptrade_rs::export::{self, Format};
use snaptrade_rs::{ActivityQuery, Client, Error};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [creds, account_id, out] = args.as_slice() else {
        eprintln!("usage: export_activities <creds-file> <account-id> <out-dir>");
        std::process::exit(2);
    };
    let client = Client::from_provider(&FileProvider::new(creds))?;
    let accounts = client.accounts()?;
    let account = accounts
        .iter()
        .find(|a| a.id.as_str() == account_id.as_str())
        .ok_or("account id not found")?;
    let acts = match client.activities(account_id.as_str(), &ActivityQuery::new()) {
        Err(e @ Error::NotAvailable { .. }) => {
            eprintln!("not available on this plan: {e}");
            return Ok(());
        }
        other => other?,
    };
    let stem = export::file_stem(account, None, None);
    for p in export::write_exports(Path::new(out), &stem, Format::Both, account, &acts)? {
        println!("wrote {}", p.display());
    }
    Ok(())
}
