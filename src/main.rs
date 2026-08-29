use std::error::Error;
use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ecra::app::import_order_file;
use ecra::game::{GameCode, generate_game};
use ecra::orders::check_order_file_syntax;
use ecra::storage::GameStore;

#[derive(Debug, Parser)]
#[command(about = "Manage deterministic turn-based ECRA games", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the application version
    Version,
    /// Create a new game store
    New {
        /// Path of the store to create
        store: PathBuf,
    },
    /// Generate a game in an existing store
    GenerateGame {
        /// Path of the game store
        store: PathBuf,
        /// Short uppercase game code
        code: String,
        /// Base seed for deterministic generation
        #[arg(long)]
        seed: Option<u64>,
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
    /// Check an order file for syntax errors
    CheckOrders {
        /// Path of the order file to check
        file: PathBuf,
    },
    /// Import and parse a raw order file
    ImportOrders {
        /// Path of the game store
        store: PathBuf,
        /// Path of the order file to import
        file: PathBuf,
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
        Command::Version => {
            println!("ecra {}", env!("CARGO_PKG_VERSION"));
        }
        Command::New { store } => {
            let store = GameStore::create(store)?;
            println!("Created ECRA store at {}", store.path().display());
        }
        Command::GenerateGame { store, code, seed } => {
            let store = GameStore::open(store)?;
            let game = generate_game(GameCode::new(code)?, seed);
            store.create_game(&game)?;
            println!(
                "Generated game {} with seed {} (status: {}, stellia: {})",
                game.code,
                game.seed,
                game.status,
                game.stellia.len()
            );
        }
        Command::Manage { store } => {
            let store = GameStore::open(store)?;
            let info = store.info()?;
            println!("Store: {}", store.path().display());
            println!("Format version: {}", info.format_version);
            println!("Current turn: {}", info.current_turn);
            println!("Games: {}", store.game_count()?);
        }
        Command::SeedAccounts { store } => {
            let store = GameStore::open(store)?;
            let created = store.seed_test_accounts()?;
            println!(
                "Created {created} test accounts in {}",
                store.path().display()
            );
        }
        Command::CheckOrders { file } => {
            let source = fs::read_to_string(&file)?;
            let errors = check_order_file_syntax(file.display().to_string(), &source);
            if errors.is_empty() {
                println!("No syntax errors found in {}", file.display());
            } else {
                for error in &errors {
                    eprintln!("{error}");
                }
                return Err(format!(
                    "found {} syntax error{}",
                    errors.len(),
                    if errors.len() == 1 { "" } else { "s" }
                )
                .into());
            }
        }
        Command::ImportOrders { store, file } => {
            let store = GameStore::open(store)?;
            let source = fs::read_to_string(&file)?;
            let result = import_order_file(&store, &file.display().to_string(), &source)?;
            println!(
                "Imported {} as order import {}",
                file.display(),
                result.imported.id.number()
            );
            match result.parsed {
                Ok(parsed) => {
                    println!("Parsed {} player orders successfully", parsed.orders.len());
                }
                Err(errors) => {
                    for error in &errors {
                        eprintln!("{error}");
                    }
                    return Err(format!(
                        "imported file {} contains {} syntax error{}; no orders are ready for validation",
                        result.imported.id.number(),
                        errors.len(),
                        if errors.len() == 1 { "" } else { "s" }
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}
