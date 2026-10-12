//! List accounts and cash balances.
//!
//!   export SNAPTRADE_CLIENT_ID=...
//!   export SNAPTRADE_CONSUMER_KEY=...
//!   cargo run --example list_accounts

use snaptrade::credentials::EnvProvider;
use snaptrade::mask::mask_number;
use snaptrade::{Client, Error};

fn main() -> Result<(), Error> {
    let client = Client::from_provider(&EnvProvider::new())?;
    for a in client.accounts()? {
        let id = a["id"].as_str().unwrap_or_default();
        println!(
            "{} {} {}",
            id,
            a["institution_name"].as_str().unwrap_or(""),
            mask_number(a["number"].as_str().unwrap_or(""))
        );
        for b in client.balances(id)? {
            println!("  {} cash={}", b["currency"]["code"], b["cash"]);
        }
    }
    Ok(())
}
