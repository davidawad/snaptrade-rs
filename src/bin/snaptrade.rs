//! `snaptrade`: read-only command-line client built on the `snaptrade` crate.
//! Order and trading endpoints are not exposed here regardless of crate features.
//!
//! Credentials: `--cred-source env` (default; SNAPTRADE_CLIENT_ID and
//! SNAPTRADE_CONSUMER_KEY), `--cred-source file --cred-file PATH`, or
//! `--cred-source command --cred-command "tool get --field {field}"`.

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::Value;
use snaptrade::credentials::{CommandProvider, CredentialProvider, EnvProvider, FileProvider};
use snaptrade::export::{self, Format};
use snaptrade::mask::mask_number;
use snaptrade::select::Selector;
use snaptrade::transport::Transport;
use snaptrade::{ActivityQuery, Client};
use std::path::PathBuf;

#[derive(Clone, Copy, ValueEnum)]
enum Source {
    Env,
    File,
    Command,
}

#[derive(Parser)]
#[command(name = "snaptrade", version, about = "Read-only SnapTrade client.")]
struct Cli {
    #[arg(long, value_enum, default_value = "env", global = true)]
    cred_source: Source,
    /// Credential file (KEY=VALUE lines or JSON) for `--cred-source file`.
    #[arg(long, global = true)]
    cred_file: Option<PathBuf>,
    /// Command for `--cred-source command`; `{field}` becomes client_id or
    /// consumer_key. Split on whitespace, run without a shell.
    #[arg(long, global = true)]
    cred_command: Option<String>,
    #[arg(long, default_value = snaptrade::client::DEFAULT_BASE_URL, global = true)]
    base_url: String,
    /// Print what would be requested; send nothing.
    #[arg(long, global = true)]
    dry_run: bool,
    /// Machine-readable JSON output.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Args, Clone)]
struct Sel {
    /// Account id (SnapTrade UUID).
    #[arg(long)]
    account_id: Option<String>,
    /// Brokerage/institution name, case-insensitive substring.
    #[arg(long)]
    brokerage: Option<String>,
    /// Last digits of the account number.
    #[arg(long)]
    last4: Option<String>,
}

#[derive(Subcommand)]
enum Cmd {
    /// API status.
    Status,
    /// List brokerage connections.
    Connections,
    /// List accounts (numbers masked).
    Accounts(Sel),
    /// Cash balances.
    Balances(Sel),
    /// Positions (HTTP 410 on Personal keys).
    Positions(Sel),
    /// Positions across all instrument types.
    AllPositions(Sel),
    /// Option positions.
    Options(Sel),
    /// Export activities to CSV/JSON files.
    Activities {
        #[command(flatten)]
        sel: Sel,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long, default_value = "both")]
        format: String,
        #[arg(long, default_value = "./snaptrade-exports")]
        out_dir: PathBuf,
        #[arg(long, default_value_t = 1000)]
        page_limit: u32,
    },
}

fn provider(cli: &Cli) -> Result<Box<dyn CredentialProvider>, String> {
    Ok(match cli.cred_source {
        Source::Env => Box::new(EnvProvider::new()),
        Source::File => Box::new(FileProvider::new(
            cli.cred_file
                .clone()
                .ok_or("--cred-file is required with --cred-source file")?,
        )),
        Source::Command => {
            let spec = cli
                .cred_command
                .clone()
                .ok_or("--cred-command is required with --cred-source command")?;
            let mut parts = spec.split_whitespace().map(String::from);
            let prog = parts.next().ok_or("--cred-command is empty")?;
            Box::new(CommandProvider::new(prog, parts))
        }
    })
}

fn select<'a>(accounts: &'a [Value], s: &Sel) -> Result<Vec<&'a Value>, String> {
    let m = Selector {
        id: s.account_id.clone(),
        brokerage: s.brokerage.clone(),
        last4: s.last4.clone(),
    }
    .filter(accounts);
    if m.is_empty() {
        Err("no account matches the selector (run `snaptrade accounts`)".into())
    } else {
        Ok(m)
    }
}

fn account_line(a: &Value) -> String {
    format!(
        "{}  {}  {}  {}",
        a["id"].as_str().unwrap_or(""),
        a["institution_name"].as_str().unwrap_or(""),
        a["name"].as_str().unwrap_or(""),
        mask_number(a["number"].as_str().unwrap_or("")),
    )
}

fn pretty(v: &impl serde::Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(v).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn run<T: Transport>(cli: &Cli, cl: &Client<T>) -> Result<(), String> {
    let e = |x: snaptrade::Error| x.to_string();
    match &cli.cmd {
        Cmd::Status => pretty(&cl.api_status().map_err(e)?),
        Cmd::Connections => {
            let rows = cl.connections().map_err(e)?;
            if cli.json {
                return pretty(&rows);
            }
            for r in &rows {
                println!(
                    "{}  {}  disabled={}",
                    r["id"].as_str().unwrap_or(""),
                    r["brokerage"]["name"].as_str().unwrap_or(""),
                    r["disabled"]
                );
            }
            Ok(())
        }
        Cmd::Accounts(s) => {
            let all = cl.accounts().map_err(e)?;
            let shown = Selector {
                id: s.account_id.clone(),
                brokerage: s.brokerage.clone(),
                last4: s.last4.clone(),
            }
            .filter(&all);
            if cli.json {
                let v: Vec<Value> = shown.iter().map(|a| export::masked_account(a)).collect();
                return pretty(&v);
            }
            shown.iter().for_each(|a| println!("{}", account_line(a)));
            Ok(())
        }
        Cmd::Balances(s) => per_account(cl, s, |id| Ok(Value::Array(cl.balances(id).map_err(e)?))),
        Cmd::Positions(s) => {
            per_account(cl, s, |id| Ok(Value::Array(cl.positions(id).map_err(e)?)))
        }
        Cmd::AllPositions(s) => per_account(cl, s, |id| cl.all_positions(id).map_err(e)),
        Cmd::Options(s) => per_account(cl, s, |id| {
            Ok(Value::Array(cl.option_positions(id).map_err(e)?))
        }),
        Cmd::Activities {
            sel,
            from,
            to,
            format,
            out_dir,
            page_limit: _,
        } => {
            let fmt = Format::parse(format)?;
            let all = cl.accounts().map_err(e)?;
            for a in select(&all, sel)? {
                let id = a["id"].as_str().unwrap_or("");
                let stem = export::file_stem(a, from.as_deref(), to.as_deref());
                if cli.dry_run {
                    println!("DRY-RUN {}", account_line(a));
                    export::planned_paths(out_dir, &stem, fmt)
                        .iter()
                        .for_each(|p| println!("  would write {}", p.display()));
                    continue;
                }
                let mut q = ActivityQuery::new();
                q.start_date = from.clone();
                q.end_date = to.clone();
                let acts = cl.activities(id, &q).map_err(e)?;
                let written = export::write_exports(out_dir, &stem, fmt, a, &acts)?;
                println!("{}  activities={}", account_line(a), acts.len());
                written
                    .iter()
                    .for_each(|p| println!("  wrote {}", p.display()));
            }
            Ok(())
        }
    }
}

fn per_account<T: Transport>(
    cl: &Client<T>,
    s: &Sel,
    f: impl Fn(&str) -> Result<Value, String>,
) -> Result<(), String> {
    let all = cl.accounts().map_err(|x| x.to_string())?;
    for a in select(&all, s)? {
        println!("# {}", account_line(a));
        pretty(&f(a["id"].as_str().unwrap_or(""))?)?;
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let result = if cli.dry_run && !matches!(cli.cmd, Cmd::Activities { .. }) {
        println!("DRY-RUN: no request sent");
        Ok(())
    } else {
        provider(&cli)
            .and_then(|p| Client::from_provider(p.as_ref()).map_err(|e| e.to_string()))
            .map(|c| {
                let page = match &cli.cmd {
                    Cmd::Activities { page_limit, .. } => *page_limit,
                    _ => snaptrade::client::DEFAULT_PAGE_LIMIT,
                };
                c.base_url(&cli.base_url).page_limit(page)
            })
            .and_then(|c| run(&cli, &c))
    };
    if let Err(e) = result {
        eprintln!("snaptrade: {e}");
        std::process::exit(1);
    }
}
