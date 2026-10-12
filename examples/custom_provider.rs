//! Plug in your own secret store by implementing `CredentialProvider`, or use
//! `CommandProvider` to call any CLI that prints a secret.
//!
//!   cargo run --example custom_provider

use snaptrade_rs::credentials::{CommandProvider, CredentialProvider, Credentials};
use snaptrade_rs::Result;

struct Mine;

impl CredentialProvider for Mine {
    fn credentials(&self) -> Result<Credentials> {
        // Fetch from your vault here.
        Ok(Credentials::new(
            "client-id-from-vault",
            "consumer-key-from-vault",
        ))
    }
}

fn main() -> Result<()> {
    let creds = Mine.credentials()?;
    println!("loaded: {creds:?}"); // Debug is redacted

    // Any CLI works; `{field}` becomes client_id / consumer_key.
    let cmd = CommandProvider::new("my-vault", ["get", "snaptrade", "--field", "{field}"])
        .rename_field("client_id", "username")
        .rename_field("consumer_key", "password");
    println!("{cmd:?}");
    Ok(())
}
