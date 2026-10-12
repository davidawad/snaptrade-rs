//! List accounts and cash balances.
//!
//!   export SNAPTRADE_CLIENT_ID=...
//!   export SNAPTRADE_CONSUMER_KEY=...
//!   cargo run --example list_accounts

use snaptrade_rs::credentials::EnvProvider;
use snaptrade_rs::mask::mask_number;
use snaptrade_rs::{Client, Error};

fn main() -> Result<(), Error> {
    let client = Client::from_provider(&EnvProvider::new())?;
    for a in client.accounts()? {
        println!(
            "{} {} {}",
            a.id,
            a.institution_name.as_deref().unwrap_or(""),
            mask_number(a.number.as_deref().unwrap_or(""))
        );
        for b in client.balances(&a.id)? {
            let code = b.currency.and_then(|c| c.code).unwrap_or_default();
            let cash = b.cash.map(|c| c.to_string()).unwrap_or_default();
            println!("  {code} cash={cash}");
        }
    }
    Ok(())
}
