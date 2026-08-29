use std::error::Error;
use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ecra::app::import_order_file;
use ecra::game::{
    DEFAULT_MINIMUM_STELLIUM_DISTANCE, GameCode, GenerateGameOptions, Player, generate_game,
};
use ecra::orders::check_order_file_syntax;
use ecra::reports::{player_list, player_list_json, stellium_list, stellium_list_json};
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
        /// Minimum Euclidean distance between stellia
        #[arg(long, default_value_t = DEFAULT_MINIMUM_STELLIUM_DISTANCE)]
        minimum_distance: u8,
    },
    /// Add players to an existing game
    AddPlayers {
        /// Path of the game store
        store: PathBuf,
        /// Short uppercase game code
        code: String,
        /// Email addresses to assign to the game
        #[arg(required = true)]
        emails: Vec<String>,
    },
    /// Generate reports from a stored game
    Report {
        #[command(subcommand)]
        report: ReportCommand,
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

#[derive(Debug, Subcommand)]
enum ReportCommand {
    /// List players assigned to a game
    Players {
        /// Path of the game store
        store: PathBuf,
        /// Short uppercase game code
        code: String,
        /// Save the list as JSON
        #[arg(long, value_name = "FILE")]
        json: Option<PathBuf>,
    },
    /// List stellia, their coordinates, and their star counts
    Stellia {
        /// Path of the game store
        store: PathBuf,
        /// Short uppercase game code
        code: String,
        /// Save the list as JSON
        #[arg(long, value_name = "FILE")]
        json: Option<PathBuf>,
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
        Command::GenerateGame {
            store,
            code,
            seed,
            minimum_distance,
        } => {
            let store = GameStore::open(store)?;
            let game = generate_game(
                GameCode::new(code)?,
                GenerateGameOptions {
                    seed,
                    minimum_stellium_distance: minimum_distance,
                },
            )?;
            store.create_game(&game)?;
            println!(
                "Generated game {} with seed {} (status: {}, stellia: {}, minimum distance: {})",
                game.code,
                game.seed,
                game.status,
                game.stellia.len(),
                game.minimum_stellium_distance
            );
        }
        Command::AddPlayers {
            store,
            code,
            emails,
        } => {
            let store = GameStore::open(store)?;
            let code = GameCode::new(code)?;
            let players = emails
                .into_iter()
                .map(|email| Player { email })
                .collect::<Vec<_>>();
            let created = store.add_players(&code, &players)?;
            println!(
                "Added {created} player{} to game {code}",
                if created == 1 { "" } else { "s" }
            );
        }
        Command::Report { report } => match report {
            ReportCommand::Players { store, code, json } => {
                let store = GameStore::open(store)?;
                let game = store.load_game(&GameCode::new(code)?)?;
                let entries = player_list(&game);

                if let Some(path) = json {
                    fs::write(&path, player_list_json(&entries)?)?;
                    println!("Saved players report to {}", path.display());
                } else {
                    println!("EMAIL");
                    for entry in entries {
                        println!("{}", entry.email);
                    }
                }
            }
            ReportCommand::Stellia { store, code, json } => {
                let store = GameStore::open(store)?;
                let game = store.load_game(&GameCode::new(code)?)?;
                let entries = stellium_list(&game);

                if let Some(path) = json {
                    fs::write(&path, stellium_list_json(&entries)?)?;
                    println!("Saved stellia report to {}", path.display());
                } else {
                    println!("STELLIUM   X   Y   Z  STARS");
                    for entry in entries {
                        println!(
                            "{:8} {:3} {:3} {:3} {:6}",
                            entry.id, entry.x, entry.y, entry.z, entry.stars
                        );
                    }
                }
            }
        },
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
