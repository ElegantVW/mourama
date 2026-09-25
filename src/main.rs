mod hex;
mod paths;
mod serve;
mod sim;
mod store;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::sync::Arc;
use store::Store;

#[derive(Parser)]
#[command(name = "mourama", about = "Mourama — Iberian hillfort. Five seats.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Tick the wood and serve the window (:4747)
    Serve {
        #[arg(long, default_value = "0.0.0.0:4747")]
        bind: String,
    },
    /// Print a one-time invite (max 5 seats)
    Invite,
    /// Who holds the five seats
    Seats,
    /// Clock, seats, next breath
    Status,
    /// Stop is just SIGINT on serve; this prints the law
    Smoor,
    /// Open the native client
    Play,
    /// Leave a report on a seated court (host)
    Whisper {
        name: String,
        message: String,
    },
}

fn open_store() -> Result<Store> {
    paths::ensure_data_dir()?;
    Store::open(&paths::world_db())
}

fn print_status(s: &store::Status) {
    println!("╭─ ✦ mourama ✦ ──────────────────────────╮");
    println!(
        "│ seats {:>1}/{:<1}   {}           │",
        s.seats_taken, s.seats_max, s.clock_local
    );
    for seat in &s.seats {
        let who = seat.name.as_deref().unwrap_or("— open —");
        println!(
            "│  seat {}  ({:>2},{:>2})  {:<18} │",
            seat.idx, seat.q, seat.r, who
        );
    }
    println!("╰────────────────────────────────────────╯");
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Status) {
        Command::Serve { bind } => {
            let addr: SocketAddr = bind.parse().map_err(|_| {
                anyhow::anyhow!("mourama: could not parse bind address.\n  next:  mourama serve --bind 0.0.0.0:4747")
            })?;
            let store = Arc::new(open_store()?);
            serve::serve(addr, store).await
        }
        Command::Invite => {
            let store = open_store()?;
            match store.invite() {
                Ok(code) => {
                    println!("mourama: invite {code}");
                    println!("  next:  open the window and claim a seat.");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("  next:  mourama seats");
                    std::process::exit(1);
                }
            }
        }
        Command::Seats | Command::Status => {
            let store = open_store()?;
            match store.status() {
                Ok(s) => {
                    print_status(&s);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Command::Smoor => {
            println!("mourama: smooring is stopping serve (Ctrl-C).");
            println!("  next:  time only moves while the office box runs mourama serve.");
            Ok(())
        }
        Command::Play => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let here = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let mut candidates = Vec::new();
            if let Ok(p) = std::env::var("MOURAMA_PLAY") {
                candidates.push(std::path::PathBuf::from(p));
            }
            candidates.push(std::path::PathBuf::from(&home).join(".local/lib/faeos/mourama-play"));
            candidates.push(here.join("target/release/mourama-play"));
            candidates.push(
                std::path::PathBuf::from(&home).join("mourama/target/release/mourama-play"),
            );
            for c in candidates {
                if c.is_file() {
                    let status = std::process::Command::new(&c).status()?;
                    std::process::exit(status.code().unwrap_or(1));
                }
            }
            anyhow::bail!(
                "mourama: native client is not built yet.\n  next:  cd ~/mourama && ./build.sh install"
            )
        }
        Command::Whisper { name, message } => {
            let store = open_store()?;
            store.whisper(&name, &message)?;
            println!("mourama: left a note for {name}.");
            Ok(())
        }
    }
}
