use std::error::Error;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ecra::storage::GameStore;

#[derive(Debug, Parser)]
#[command(about = "Manage deterministic turn-based ECRA games", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new game store
    New {
        /// Path of the store to create
        store: PathBuf,
    },
    /// Open and inspect an existing game store
    Manage {
        /// Path of the store to manage
        store: PathBuf,
    },
    /// Seed an existing store with accounts for testing
    SeedAccounts {
        /// Path of the store to seed
        store: PathBuf,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Command::New { store } => {
            let store = GameStore::create(store)?;
            println!("Created ECRA store at {}", store.path().display());
        }
        Command::Manage { store } => {
            let store = GameStore::open(store)?;
            let info = store.info()?;
            println!("Store: {}", store.path().display());
            println!("Format version: {}", info.format_version);
            println!("Current turn: {}", info.current_turn);
        }
        Command::SeedAccounts { store } => {
            let store = GameStore::open(store)?;
            let created = store.seed_test_accounts()?;
            println!(
                "Created {created} test accounts in {}",
                store.path().display()
            );
        }
    }
    Ok(())
}
