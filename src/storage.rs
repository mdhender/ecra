use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};

use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use thiserror::Error;

use crate::accounts::{Account, AccountRole};
use crate::game::{
    Coordinates, Game, GameCode, GameStatus, MAXIMUM_COORDINATE, MINIMUM_COORDINATE,
    STELLIA_PER_GAME, Star, StarId, Stellium, StelliumId,
};

const METADATA: TableDefinition<&str, &str> = TableDefinition::new("ecra_metadata");
const ACCOUNT_EMAILS: TableDefinition<u32, &str> = TableDefinition::new("account_emails");
const ACCOUNT_TOKENS: TableDefinition<u32, &str> = TableDefinition::new("account_tokens");
const ACCOUNT_ROLES: TableDefinition<u32, &str> = TableDefinition::new("account_roles");
const GAME_SEEDS: TableDefinition<&str, u64> = TableDefinition::new("game_seeds");
const GAME_STATUSES: TableDefinition<&str, &str> = TableDefinition::new("game_statuses");
const GAME_MINIMUM_STELLIUM_DISTANCES: TableDefinition<&str, u8> =
    TableDefinition::new("game_minimum_stellium_distances");
const STELLIUM_STAR_COUNTS: TableDefinition<&str, u8> =
    TableDefinition::new("stellium_star_counts");
const STELLIUM_X_COORDINATES: TableDefinition<&str, i8> =
    TableDefinition::new("stellium_x_coordinates");
const STELLIUM_Y_COORDINATES: TableDefinition<&str, i8> =
    TableDefinition::new("stellium_y_coordinates");
const STELLIUM_Z_COORDINATES: TableDefinition<&str, i8> =
    TableDefinition::new("stellium_z_coordinates");
const ORDER_IMPORT_FILENAMES: TableDefinition<u64, &str> =
    TableDefinition::new("order_import_filenames");
const ORDER_IMPORT_SOURCES: TableDefinition<u64, &str> =
    TableDefinition::new("order_import_sources");
const ORDER_IMPORT_PARSE_STATUS: TableDefinition<u64, &str> =
    TableDefinition::new("order_import_parse_status");
const ORDER_IMPORT_DIAGNOSTICS: TableDefinition<u64, &str> =
    TableDefinition::new("order_import_diagnostics");
const APPLICATION_KEY: &str = "application";
const APPLICATION_VALUE: &str = "ecra";
const FORMAT_VERSION_KEY: &str = "format_version";
const FORMAT_VERSION_VALUE: &str = "2";
const SUPPORTED_FORMAT_VERSION: u32 = 2;
const CURRENT_TURN_KEY: &str = "current_turn";
const INITIAL_TURN: &str = "1";
const NEXT_ORDER_IMPORT_KEY: &str = "next_order_import";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OrderImportId(u64);

impl OrderImportId {
    pub fn number(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoredParseOutcome {
    Success,
    Failure(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedOrderFile {
    pub id: OrderImportId,
    pub filename: String,
    pub source: String,
    pub parse_outcome: Option<StoredParseOutcome>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreInfo {
    pub format_version: u32,
    pub current_turn: u32,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("store `{}` already exists", .0.display())]
    AlreadyExists(PathBuf),
    #[error("could not create store `{}`: {source}", path.display())]
    Create {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid store path `{}`: {reason}", path.display())]
    InvalidPath { path: PathBuf, reason: String },
    #[error("could not access store `{}`: {source}", path.display())]
    Database {
        path: PathBuf,
        #[source]
        source: redb::Error,
    },
    #[error("`{}` is not an ECRA store: {reason}", path.display())]
    InvalidStore { path: PathBuf, reason: String },
    #[error("test account {number:04} conflicts with an existing account")]
    AccountConflict { number: u32 },
    #[error("game `{0}` already exists")]
    GameAlreadyExists(String),
    #[error("game `{0}` does not exist")]
    GameNotFound(String),
    #[error("order import {0} does not exist")]
    OrderImportNotFound(u64),
    #[error("order import {0} already has a different parse result")]
    ParseResultConflict(u64),
}

pub struct GameStore {
    database: Database,
    path: PathBuf,
}

impl GameStore {
    /// Creates and initializes a store without replacing an existing file.
    pub fn create(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_owned();
        validate_new_store_path(&path)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| {
                if source.kind() == io::ErrorKind::AlreadyExists {
                    StoreError::AlreadyExists(path.clone())
                } else {
                    StoreError::Create {
                        path: path.clone(),
                        source,
                    }
                }
            })?;

        let initialized = (|| {
            let database = Database::builder()
                .create_file(file)
                .map_err(|source| database_error(&path, source))?;
            let write = database
                .begin_write()
                .map_err(|source| database_error(&path, source))?;
            {
                let mut metadata = write
                    .open_table(METADATA)
                    .map_err(|source| database_error(&path, source))?;
                metadata
                    .insert(APPLICATION_KEY, APPLICATION_VALUE)
                    .map_err(|source| database_error(&path, source))?;
                metadata
                    .insert(FORMAT_VERSION_KEY, FORMAT_VERSION_VALUE)
                    .map_err(|source| database_error(&path, source))?;
                metadata
                    .insert(CURRENT_TURN_KEY, INITIAL_TURN)
                    .map_err(|source| database_error(&path, source))?;
                metadata
                    .insert(NEXT_ORDER_IMPORT_KEY, "1")
                    .map_err(|source| database_error(&path, source))?;
            }
            database_tables(&write, &path)?;
            write
                .commit()
                .map_err(|source| database_error(&path, source))?;
            Ok(Self {
                database,
                path: path.clone(),
            })
        })();

        if initialized.is_err() {
            let _ = std::fs::remove_file(&path);
        }
        initialized
    }

    /// Opens an existing ECRA store and validates its metadata.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_owned();
        let database = Database::open(&path).map_err(|source| database_error(&path, source))?;
        let store = Self { database, path };
        store.info()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn info(&self) -> Result<StoreInfo, StoreError> {
        let read = self
            .database
            .begin_read()
            .map_err(|source| database_error(&self.path, source))?;
        let metadata = read
            .open_table(METADATA)
            .map_err(|_| StoreError::InvalidStore {
                path: self.path.clone(),
                reason: "required metadata is missing".to_owned(),
            })?;

        let application = metadata_value(&metadata, APPLICATION_KEY, &self.path)?;
        if application != APPLICATION_VALUE {
            return Err(StoreError::InvalidStore {
                path: self.path.clone(),
                reason: "application identifier is invalid".to_owned(),
            });
        }

        let format_version = parse_metadata_u32(&metadata, FORMAT_VERSION_KEY, &self.path)?;
        if format_version != SUPPORTED_FORMAT_VERSION {
            return Err(StoreError::InvalidStore {
                path: self.path.clone(),
                reason: format!("unsupported format version {format_version}"),
            });
        }
        let current_turn = parse_metadata_u32(&metadata, CURRENT_TURN_KEY, &self.path)?;

        Ok(StoreInfo {
            format_version,
            current_turn,
        })
    }

    /// Atomically adds a generated game. Game codes are unique within the store.
    pub fn create_game(&self, game: &Game) -> Result<(), StoreError> {
        let write = self
            .database
            .begin_write()
            .map_err(|source| database_error(&self.path, source))?;
        let code = game.code.as_str();

        {
            let mut seeds = write
                .open_table(GAME_SEEDS)
                .map_err(|source| database_error(&self.path, source))?;
            if seeds
                .get(code)
                .map_err(|source| database_error(&self.path, source))?
                .is_some()
            {
                return Err(StoreError::GameAlreadyExists(code.to_owned()));
            }
            seeds
                .insert(code, game.seed)
                .map_err(|source| database_error(&self.path, source))?;
        }
        write
            .open_table(GAME_STATUSES)
            .map_err(|source| database_error(&self.path, source))?
            .insert(code, game.status.as_str())
            .map_err(|source| database_error(&self.path, source))?;
        write
            .open_table(GAME_MINIMUM_STELLIUM_DISTANCES)
            .map_err(|source| database_error(&self.path, source))?
            .insert(code, game.minimum_stellium_distance)
            .map_err(|source| database_error(&self.path, source))?;
        {
            let mut star_counts = write
                .open_table(STELLIUM_STAR_COUNTS)
                .map_err(|source| database_error(&self.path, source))?;
            let mut x_coordinates = write
                .open_table(STELLIUM_X_COORDINATES)
                .map_err(|source| database_error(&self.path, source))?;
            let mut y_coordinates = write
                .open_table(STELLIUM_Y_COORDINATES)
                .map_err(|source| database_error(&self.path, source))?;
            let mut z_coordinates = write
                .open_table(STELLIUM_Z_COORDINATES)
                .map_err(|source| database_error(&self.path, source))?;
            for stellium in &game.stellia {
                let key = stellium_key(code, stellium.id);
                star_counts
                    .insert(key.as_str(), stellium.star_count() as u8)
                    .map_err(|source| database_error(&self.path, source))?;
                x_coordinates
                    .insert(key.as_str(), stellium.coordinates.x)
                    .map_err(|source| database_error(&self.path, source))?;
                y_coordinates
                    .insert(key.as_str(), stellium.coordinates.y)
                    .map_err(|source| database_error(&self.path, source))?;
                z_coordinates
                    .insert(key.as_str(), stellium.coordinates.z)
                    .map_err(|source| database_error(&self.path, source))?;
            }
        }

        write
            .commit()
            .map_err(|source| database_error(&self.path, source))
    }

    pub fn load_game(&self, code: &GameCode) -> Result<Game, StoreError> {
        let read = self
            .database
            .begin_read()
            .map_err(|source| database_error(&self.path, source))?;
        let seeds = read
            .open_table(GAME_SEEDS)
            .map_err(|source| database_error(&self.path, source))?;
        let statuses = read
            .open_table(GAME_STATUSES)
            .map_err(|source| database_error(&self.path, source))?;
        let minimum_distances = read
            .open_table(GAME_MINIMUM_STELLIUM_DISTANCES)
            .map_err(|source| database_error(&self.path, source))?;
        let star_counts = read
            .open_table(STELLIUM_STAR_COUNTS)
            .map_err(|source| database_error(&self.path, source))?;
        let x_coordinates = read
            .open_table(STELLIUM_X_COORDINATES)
            .map_err(|source| database_error(&self.path, source))?;
        let y_coordinates = read
            .open_table(STELLIUM_Y_COORDINATES)
            .map_err(|source| database_error(&self.path, source))?;
        let z_coordinates = read
            .open_table(STELLIUM_Z_COORDINATES)
            .map_err(|source| database_error(&self.path, source))?;
        let missing = || StoreError::GameNotFound(code.to_string());

        let seed = seeds
            .get(code.as_str())
            .map_err(|source| database_error(&self.path, source))?
            .map(|value| value.value())
            .ok_or_else(missing)?;
        let status_text = statuses
            .get(code.as_str())
            .map_err(|source| database_error(&self.path, source))?
            .map(|value| value.value().to_owned())
            .ok_or_else(|| invalid_game(&self.path, code, "status is missing"))?;
        let status = GameStatus::from_str(&status_text)
            .ok_or_else(|| invalid_game(&self.path, code, "status is invalid"))?;
        let minimum_stellium_distance = minimum_distances
            .get(code.as_str())
            .map_err(|source| database_error(&self.path, source))?
            .map(|value| value.value())
            .ok_or_else(|| {
                invalid_game(&self.path, code, "minimum stellium distance is missing")
            })?;
        let mut stellia = Vec::with_capacity(STELLIA_PER_GAME);
        for number in 1..=STELLIA_PER_GAME as u32 {
            let id = StelliumId::new(number);
            let key = stellium_key(code.as_str(), id);
            let star_count = star_counts
                .get(key.as_str())
                .map_err(|source| database_error(&self.path, source))?
                .map(|value| value.value())
                .ok_or_else(|| invalid_game(&self.path, code, "stellium data is incomplete"))?;
            if !(1..=5).contains(&star_count) {
                return Err(invalid_game(
                    &self.path,
                    code,
                    "stellium star count is invalid",
                ));
            }
            let stars = (1..=star_count)
                .map(|number| Star {
                    id: StarId::new(number),
                })
                .collect();
            let coordinates = Coordinates {
                x: x_coordinates
                    .get(key.as_str())
                    .map_err(|source| database_error(&self.path, source))?
                    .map(|value| value.value())
                    .ok_or_else(|| {
                        invalid_game(&self.path, code, "stellium coordinates are incomplete")
                    })?,
                y: y_coordinates
                    .get(key.as_str())
                    .map_err(|source| database_error(&self.path, source))?
                    .map(|value| value.value())
                    .ok_or_else(|| {
                        invalid_game(&self.path, code, "stellium coordinates are incomplete")
                    })?,
                z: z_coordinates
                    .get(key.as_str())
                    .map_err(|source| database_error(&self.path, source))?
                    .map(|value| value.value())
                    .ok_or_else(|| {
                        invalid_game(&self.path, code, "stellium coordinates are incomplete")
                    })?,
            };
            if !(MINIMUM_COORDINATE..=MAXIMUM_COORDINATE).contains(&coordinates.x)
                || !(MINIMUM_COORDINATE..=MAXIMUM_COORDINATE).contains(&coordinates.y)
                || !(MINIMUM_COORDINATE..=MAXIMUM_COORDINATE).contains(&coordinates.z)
                || coordinates == (Coordinates { x: 0, y: 0, z: 0 })
            {
                return Err(invalid_game(
                    &self.path,
                    code,
                    "stellium coordinates are invalid",
                ));
            }
            stellia.push(Stellium {
                id,
                coordinates,
                stars,
            });
        }

        Ok(Game {
            code: code.clone(),
            seed,
            minimum_stellium_distance,
            status,
            stellia,
        })
    }

    pub fn game_count(&self) -> Result<u64, StoreError> {
        let read = self
            .database
            .begin_read()
            .map_err(|source| database_error(&self.path, source))?;
        read.open_table(GAME_SEEDS)
            .map_err(|source| database_error(&self.path, source))?
            .len()
            .map_err(|source| database_error(&self.path, source))
    }

    /// Adds the fixed testing accounts, leaving an already-matching seed unchanged.
    pub fn seed_test_accounts(&self) -> Result<usize, StoreError> {
        let write = self
            .database
            .begin_write()
            .map_err(|source| database_error(&self.path, source))?;
        let mut created = 0;
        {
            let mut emails = write
                .open_table(ACCOUNT_EMAILS)
                .map_err(|source| database_error(&self.path, source))?;
            let mut tokens = write
                .open_table(ACCOUNT_TOKENS)
                .map_err(|source| database_error(&self.path, source))?;
            let mut roles = write
                .open_table(ACCOUNT_ROLES)
                .map_err(|source| database_error(&self.path, source))?;

            for number in 1..=13 {
                let role = if number == 1 {
                    AccountRole::Administrator
                } else {
                    AccountRole::User
                };
                let account = Account::test_account(number, role);
                let stored_email = emails
                    .get(number)
                    .map_err(|source| database_error(&self.path, source))?
                    .map(|value| value.value().to_owned());
                let stored_token = tokens
                    .get(number)
                    .map_err(|source| database_error(&self.path, source))?
                    .map(|value| value.value().to_owned());
                let stored_role = roles
                    .get(number)
                    .map_err(|source| database_error(&self.path, source))?
                    .map(|value| value.value().to_owned());

                match (stored_email, stored_token, stored_role) {
                    (None, None, None) => {
                        emails
                            .insert(number, account.email())
                            .map_err(|source| database_error(&self.path, source))?;
                        tokens
                            .insert(number, account.token())
                            .map_err(|source| database_error(&self.path, source))?;
                        roles
                            .insert(number, account.role().as_str())
                            .map_err(|source| database_error(&self.path, source))?;
                        created += 1;
                    }
                    (Some(email), Some(token), Some(stored_role))
                        if email == account.email()
                            && token == account.token()
                            && stored_role == account.role().as_str() => {}
                    _ => return Err(StoreError::AccountConflict { number }),
                }
            }
        }
        write
            .commit()
            .map_err(|source| database_error(&self.path, source))?;
        Ok(created)
    }

    /// Atomically stores a raw order-file submission without interpreting it.
    pub(crate) fn import_order_file(
        &self,
        filename: &str,
        source: &str,
    ) -> Result<OrderImportId, StoreError> {
        let write = self
            .database
            .begin_write()
            .map_err(|source| database_error(&self.path, source))?;

        let id = {
            let mut metadata = write
                .open_table(METADATA)
                .map_err(|source| database_error(&self.path, source))?;
            let next = metadata
                .get(NEXT_ORDER_IMPORT_KEY)
                .map_err(|source| database_error(&self.path, source))?
                .map(|value| value.value().parse::<u64>())
                .transpose()
                .map_err(|_| StoreError::InvalidStore {
                    path: self.path.clone(),
                    reason: format!("metadata field `{NEXT_ORDER_IMPORT_KEY}` is invalid"),
                })?
                .unwrap_or(1);
            let following = next
                .checked_add(1)
                .ok_or_else(|| StoreError::InvalidStore {
                    path: self.path.clone(),
                    reason: "order import IDs are exhausted".to_owned(),
                })?;
            let following = following.to_string();
            metadata
                .insert(NEXT_ORDER_IMPORT_KEY, following.as_str())
                .map_err(|source| database_error(&self.path, source))?;
            OrderImportId(next)
        };

        write
            .open_table(ORDER_IMPORT_FILENAMES)
            .map_err(|source| database_error(&self.path, source))?
            .insert(id.0, filename)
            .map_err(|source| database_error(&self.path, source))?;
        write
            .open_table(ORDER_IMPORT_SOURCES)
            .map_err(|source| database_error(&self.path, source))?
            .insert(id.0, source)
            .map_err(|source| database_error(&self.path, source))?;
        write
            .open_table(ORDER_IMPORT_PARSE_STATUS)
            .map_err(|source| database_error(&self.path, source))?;
        write
            .open_table(ORDER_IMPORT_DIAGNOSTICS)
            .map_err(|source| database_error(&self.path, source))?;

        write
            .commit()
            .map_err(|source| database_error(&self.path, source))?;
        Ok(id)
    }

    pub(crate) fn record_order_parse_result(
        &self,
        id: OrderImportId,
        outcome: &StoredParseOutcome,
    ) -> Result<(), StoreError> {
        let write = self
            .database
            .begin_write()
            .map_err(|source| database_error(&self.path, source))?;
        let status = match outcome {
            StoredParseOutcome::Success => "success",
            StoredParseOutcome::Failure(_) => "failure",
        };
        let diagnostics = match outcome {
            StoredParseOutcome::Success => "",
            StoredParseOutcome::Failure(diagnostics) => diagnostics,
        };

        {
            let sources = write
                .open_table(ORDER_IMPORT_SOURCES)
                .map_err(|source| database_error(&self.path, source))?;
            if sources
                .get(id.0)
                .map_err(|source| database_error(&self.path, source))?
                .is_none()
            {
                return Err(StoreError::OrderImportNotFound(id.0));
            }
        }
        {
            let mut statuses = write
                .open_table(ORDER_IMPORT_PARSE_STATUS)
                .map_err(|source| database_error(&self.path, source))?;
            let mut stored_diagnostics = write
                .open_table(ORDER_IMPORT_DIAGNOSTICS)
                .map_err(|source| database_error(&self.path, source))?;
            let existing_status = statuses
                .get(id.0)
                .map_err(|source| database_error(&self.path, source))?
                .map(|value| value.value().to_owned());
            let existing_diagnostics = stored_diagnostics
                .get(id.0)
                .map_err(|source| database_error(&self.path, source))?
                .map(|value| value.value().to_owned());
            if let Some(existing_status) = existing_status {
                if existing_status != status || existing_diagnostics.as_deref() != Some(diagnostics)
                {
                    return Err(StoreError::ParseResultConflict(id.0));
                }
            } else {
                statuses
                    .insert(id.0, status)
                    .map_err(|source| database_error(&self.path, source))?;
                stored_diagnostics
                    .insert(id.0, diagnostics)
                    .map_err(|source| database_error(&self.path, source))?;
            }
        }
        write
            .commit()
            .map_err(|source| database_error(&self.path, source))
    }

    pub fn load_order_import(&self, id: OrderImportId) -> Result<ImportedOrderFile, StoreError> {
        let read = self
            .database
            .begin_read()
            .map_err(|source| database_error(&self.path, source))?;
        let filenames = read
            .open_table(ORDER_IMPORT_FILENAMES)
            .map_err(|source| database_error(&self.path, source))?;
        let sources = read
            .open_table(ORDER_IMPORT_SOURCES)
            .map_err(|source| database_error(&self.path, source))?;
        let statuses = read
            .open_table(ORDER_IMPORT_PARSE_STATUS)
            .map_err(|source| database_error(&self.path, source))?;
        let diagnostics = read
            .open_table(ORDER_IMPORT_DIAGNOSTICS)
            .map_err(|source| database_error(&self.path, source))?;

        let required = |value: Option<String>| value.ok_or(StoreError::OrderImportNotFound(id.0));
        let filename = required(
            filenames
                .get(id.0)
                .map_err(|source| database_error(&self.path, source))?
                .map(|value| value.value().to_owned()),
        )?;
        let source = required(
            sources
                .get(id.0)
                .map_err(|source| database_error(&self.path, source))?
                .map(|value| value.value().to_owned()),
        )?;
        let parse_outcome = match statuses
            .get(id.0)
            .map_err(|source| database_error(&self.path, source))?
            .map(|value| value.value().to_owned())
            .as_deref()
        {
            None => None,
            Some("success") => Some(StoredParseOutcome::Success),
            Some("failure") => Some(StoredParseOutcome::Failure(
                diagnostics
                    .get(id.0)
                    .map_err(|source| database_error(&self.path, source))?
                    .map(|value| value.value().to_owned())
                    .unwrap_or_default(),
            )),
            Some(other) => {
                return Err(StoreError::InvalidStore {
                    path: self.path.clone(),
                    reason: format!("order import {0} has invalid parse status `{other}`", id.0),
                });
            }
        };

        Ok(ImportedOrderFile {
            id,
            filename,
            source,
            parse_outcome,
        })
    }
}

fn database_tables(write: &redb::WriteTransaction, path: &Path) -> Result<(), StoreError> {
    write
        .open_table(GAME_SEEDS)
        .map_err(|source| database_error(path, source))?;
    write
        .open_table(GAME_STATUSES)
        .map_err(|source| database_error(path, source))?;
    write
        .open_table(GAME_MINIMUM_STELLIUM_DISTANCES)
        .map_err(|source| database_error(path, source))?;
    write
        .open_table(STELLIUM_STAR_COUNTS)
        .map_err(|source| database_error(path, source))?;
    write
        .open_table(STELLIUM_X_COORDINATES)
        .map_err(|source| database_error(path, source))?;
    write
        .open_table(STELLIUM_Y_COORDINATES)
        .map_err(|source| database_error(path, source))?;
    write
        .open_table(STELLIUM_Z_COORDINATES)
        .map_err(|source| database_error(path, source))?;
    Ok(())
}

fn validate_new_store_path(path: &Path) -> Result<(), StoreError> {
    if path.file_name().is_none() {
        return Err(StoreError::InvalidPath {
            path: path.to_owned(),
            reason: "path must include a store filename".to_owned(),
        });
    }

    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let metadata = std::fs::metadata(parent).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            StoreError::InvalidPath {
                path: path.to_owned(),
                reason: format!("parent directory `{}` does not exist", parent.display()),
            }
        } else {
            StoreError::Create {
                path: path.to_owned(),
                source,
            }
        }
    })?;
    if !metadata.is_dir() {
        return Err(StoreError::InvalidPath {
            path: path.to_owned(),
            reason: format!("parent path `{}` is not a directory", parent.display()),
        });
    }
    Ok(())
}

fn metadata_value(
    table: &impl ReadableTable<&'static str, &'static str>,
    key: &str,
    path: &Path,
) -> Result<String, StoreError> {
    table
        .get(key)
        .map_err(|source| database_error(path, source))?
        .map(|value| value.value().to_owned())
        .ok_or_else(|| StoreError::InvalidStore {
            path: path.to_owned(),
            reason: format!("metadata field `{key}` is missing"),
        })
}

fn parse_metadata_u32(
    table: &impl ReadableTable<&'static str, &'static str>,
    key: &str,
    path: &Path,
) -> Result<u32, StoreError> {
    metadata_value(table, key, path)?
        .parse()
        .map_err(|_| StoreError::InvalidStore {
            path: path.to_owned(),
            reason: format!("metadata field `{key}` is invalid"),
        })
}

fn database_error(path: &Path, source: impl Into<redb::Error>) -> StoreError {
    StoreError::Database {
        path: path.to_owned(),
        source: source.into(),
    }
}

fn stellium_key(code: &str, id: StelliumId) -> String {
    format!("{code}:{:03}", id.number())
}

fn invalid_game(path: &Path, code: &GameCode, reason: &str) -> StoreError {
    StoreError::InvalidStore {
        path: path.to_owned(),
        reason: format!("game `{code}` {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::ReadableTableMetadata;

    #[test]
    fn creates_and_reopens_a_store() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("game.redb");

        let store = GameStore::create(&path).unwrap();
        assert_eq!(
            store.info().unwrap(),
            StoreInfo {
                format_version: 2,
                current_turn: 1
            }
        );
        drop(store);

        let reopened = GameStore::open(&path).unwrap();
        assert_eq!(
            reopened.info().unwrap(),
            StoreInfo {
                format_version: 2,
                current_turn: 1
            }
        );
    }

    #[test]
    fn does_not_replace_an_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("game.redb");
        std::fs::write(&path, "keep me").unwrap();

        assert!(
            matches!(GameStore::create(&path), Err(StoreError::AlreadyExists(found)) if found == path)
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), "keep me");
    }

    #[test]
    fn does_not_create_a_missing_parent_directory() {
        let directory = tempfile::tempdir().unwrap();
        let missing_parent = directory.path().join("missing");
        let path = missing_parent.join("game.redb");

        assert!(matches!(
            GameStore::create(&path),
            Err(StoreError::InvalidPath { .. })
        ));
        assert!(!missing_parent.exists());
    }

    #[test]
    fn seeds_test_accounts_idempotently() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("game.redb");
        let store = GameStore::create(&path).unwrap();

        assert_eq!(store.seed_test_accounts().unwrap(), 13);
        assert_eq!(store.seed_test_accounts().unwrap(), 0);

        let read = store.database.begin_read().unwrap();
        let emails = read.open_table(ACCOUNT_EMAILS).unwrap();
        let tokens = read.open_table(ACCOUNT_TOKENS).unwrap();
        let roles = read.open_table(ACCOUNT_ROLES).unwrap();
        assert_eq!(emails.len().unwrap(), 13);
        assert_eq!(tokens.len().unwrap(), 13);
        assert_eq!(roles.len().unwrap(), 13);

        for number in 1..=13 {
            assert_eq!(
                emails.get(number).unwrap().unwrap().value(),
                format!("account.{number:04}@example.com")
            );
            assert_eq!(
                tokens.get(number).unwrap().unwrap().value(),
                format!("amp.rocks.{number:04}")
            );
            let expected_role = if number == 1 { "administrator" } else { "user" };
            assert_eq!(roles.get(number).unwrap().unwrap().value(), expected_role);
        }
    }

    #[test]
    fn stores_multiple_games_and_reconstructs_their_clusters() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("game.redb");
        let store = GameStore::create(&path).unwrap();
        let first = crate::game::generate_game(
            GameCode::new("FIRST").unwrap(),
            crate::game::GenerateGameOptions {
                seed: Some(10),
                ..Default::default()
            },
        )
        .unwrap();
        let second = crate::game::generate_game(
            GameCode::new("SECOND").unwrap(),
            crate::game::GenerateGameOptions {
                seed: Some(20),
                minimum_stellium_distance: 4,
            },
        )
        .unwrap();

        store.create_game(&first).unwrap();
        store.create_game(&second).unwrap();
        assert_eq!(store.game_count().unwrap(), 2);
        drop(store);

        let reopened = GameStore::open(&path).unwrap();
        assert_eq!(reopened.load_game(&first.code).unwrap(), first);
        assert_eq!(reopened.load_game(&second.code).unwrap(), second);
    }

    #[test]
    fn rejects_a_duplicate_game_code_without_changing_the_game() {
        let directory = tempfile::tempdir().unwrap();
        let store = GameStore::create(directory.path().join("game.redb")).unwrap();
        let first = crate::game::generate_game(
            GameCode::new("SAME").unwrap(),
            crate::game::GenerateGameOptions {
                seed: Some(10),
                ..Default::default()
            },
        )
        .unwrap();
        let duplicate = crate::game::generate_game(
            GameCode::new("SAME").unwrap(),
            crate::game::GenerateGameOptions {
                seed: Some(20),
                ..Default::default()
            },
        )
        .unwrap();

        store.create_game(&first).unwrap();
        assert!(matches!(
            store.create_game(&duplicate),
            Err(StoreError::GameAlreadyExists(code)) if code == "SAME"
        ));
        assert_eq!(store.load_game(&first.code).unwrap(), first);
    }
}
