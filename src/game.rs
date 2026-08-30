use std::fmt;

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use thiserror::Error;

pub const STELLIA_PER_GAME: usize = 100;
pub const DEFAULT_MINIMUM_STELLIUM_DISTANCE: u8 = 3;
pub const MINIMUM_COORDINATE: i8 = -15;
pub const MAXIMUM_COORDINATE: i8 = 15;
const STELLIUM_STREAM: u64 = 0x5354_454c_4c49_554d;
const COORDINATE_STREAM: u64 = 0x434f_4f52_4449_4e41;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GameCode(String);

impl GameCode {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidGameCode> {
        let value = value.into();
        let valid_length = (1..=16).contains(&value.len());
        let mut characters = value.chars();
        let valid_first = characters.next().is_some_and(|c| c.is_ascii_uppercase());
        let valid_rest =
            characters.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-');

        if valid_length && valid_first && valid_rest {
            Ok(Self(value))
        } else {
            Err(InvalidGameCode(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GameCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error(
    "invalid game code `{0}`; use 1-16 characters beginning with A-Z and containing only A-Z, 0-9, or -"
)]
pub struct InvalidGameCode(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameStatus {
    Setup,
}

impl GameStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Setup => "setup",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "setup" => Some(Self::Setup),
            _ => None,
        }
    }
}

impl fmt::Display for GameStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StelliumId(u32);

impl StelliumId {
    pub fn new(number: u32) -> Self {
        Self(number)
    }

    pub fn number(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stellium {
    pub id: StelliumId,
    pub coordinates: Coordinates,
    pub stars: Vec<Star>,
}

impl Stellium {
    pub fn star_count(&self) -> usize {
        self.stars.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Coordinates {
    pub x: i8,
    pub y: i8,
    pub z: i8,
}

impl Coordinates {
    pub fn squared_distance(self, other: Self) -> u32 {
        let dx = i32::from(self.x) - i32::from(other.x);
        let dy = i32::from(self.y) - i32::from(other.y);
        let dz = i32::from(self.z) - i32::from(other.z);
        (dx * dx + dy * dy + dz * dz) as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StarId(u8);

impl StarId {
    pub fn new(number: u8) -> Self {
        Self(number)
    }

    pub fn number(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Star {
    pub id: StarId,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Player {
    pub id: PlayerId,
    pub email: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerId(u64);

impl PlayerId {
    pub fn new(number: u64) -> Self {
        Self(number)
    }

    pub fn number(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AgentId(u64);

impl AgentId {
    pub fn new(number: u64) -> Self {
        Self(number)
    }

    pub fn number(self) -> u64 {
        self.0
    }
}

pub const UNCONTROLLED_AGENT_ID: AgentId = AgentId(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentKind {
    Uncontrolled,
}

impl AgentKind {
    pub fn identifier(self) -> &'static str {
        self.as_str()
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Uncontrolled => "Uncontrolled",
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Uncontrolled => "uncontrolled",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "uncontrolled" => Some(Self::Uncontrolled),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Agent {
    pub id: AgentId,
    pub kind: AgentKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FactionId(u64);

impl FactionId {
    pub fn new(number: u64) -> Self {
        Self(number)
    }

    pub fn number(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactionController {
    Player(PlayerId),
    Agent(AgentId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Faction {
    pub id: FactionId,
    pub controller: FactionController,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShipId(u64);

impl ShipId {
    pub fn new(number: u64) -> Self {
        Self(number)
    }

    pub fn number(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship {
    pub id: ShipId,
    pub faction: FactionId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Game {
    pub code: GameCode,
    pub seed: u64,
    pub minimum_stellium_distance: u8,
    pub status: GameStatus,
    pub stellia: Vec<Stellium>,
    pub players: Vec<Player>,
    pub agents: Vec<Agent>,
    pub factions: Vec<Faction>,
    pub ships: Vec<Ship>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerateGameOptions {
    pub seed: Option<u64>,
    pub minimum_stellium_distance: u8,
}

impl Default for GenerateGameOptions {
    fn default() -> Self {
        Self {
            seed: None,
            minimum_stellium_distance: DEFAULT_MINIMUM_STELLIUM_DISTANCE,
        }
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error(
    "could not place {STELLIA_PER_GAME} stellia at least {minimum_distance} units apart within [{MINIMUM_COORDINATE}..{MAXIMUM_COORDINATE}]"
)]
pub struct StelliumPlacementError {
    minimum_distance: u8,
}

pub fn generate_game(
    code: GameCode,
    options: GenerateGameOptions,
) -> Result<Game, StelliumPlacementError> {
    let GenerateGameOptions {
        seed,
        minimum_stellium_distance,
    } = options;
    let seed = seed.unwrap_or_else(rand::random);
    let mut stellium_rng = ChaCha8Rng::seed_from_u64(stream_seed(seed, STELLIUM_STREAM));
    let coordinates = generate_coordinates(seed, minimum_stellium_distance)?;
    let stellia = coordinates
        .into_iter()
        .enumerate()
        .map(|(index, coordinates)| {
            let star_count = stellium_rng.random_range(1..=5);
            Stellium {
                id: StelliumId::new(index as u32 + 1),
                coordinates,
                stars: (1..=star_count).map(|id| Star { id: StarId(id) }).collect(),
            }
        })
        .collect();

    Ok(Game {
        code,
        seed,
        minimum_stellium_distance,
        status: GameStatus::Setup,
        stellia,
        players: Vec::new(),
        agents: vec![crate::agents::uncontrolled_agent()],
        factions: Vec::new(),
        ships: Vec::new(),
    })
}

fn generate_coordinates(
    seed: u64,
    minimum_distance: u8,
) -> Result<Vec<Coordinates>, StelliumPlacementError> {
    let mut candidates = (MINIMUM_COORDINATE..=MAXIMUM_COORDINATE)
        .flat_map(|x| {
            (MINIMUM_COORDINATE..=MAXIMUM_COORDINATE).flat_map(move |y| {
                (MINIMUM_COORDINATE..=MAXIMUM_COORDINATE).map(move |z| Coordinates { x, y, z })
            })
        })
        .filter(|coordinates| *coordinates != Coordinates { x: 0, y: 0, z: 0 })
        .collect::<Vec<_>>();
    let mut rng = ChaCha8Rng::seed_from_u64(stream_seed(seed, COORDINATE_STREAM));
    candidates.shuffle(&mut rng);

    let minimum_squared_distance = u32::from(minimum_distance).pow(2);
    let mut selected = Vec::with_capacity(STELLIA_PER_GAME);
    for candidate in candidates {
        if selected
            .iter()
            .all(|existing| candidate.squared_distance(*existing) >= minimum_squared_distance)
        {
            selected.push(candidate);
            if selected.len() == STELLIA_PER_GAME {
                return Ok(selected);
            }
        }
    }

    Err(StelliumPlacementError { minimum_distance })
}

// SplitMix64 gives each named stream a stable seed without sharing mutable PRNG state.
fn stream_seed(base_seed: u64, stream: u64) -> u64 {
    let mut value = base_seed ^ stream;
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_human_readable_uppercase_codes() {
        assert_eq!(GameCode::new("ECRA-01").unwrap().as_str(), "ECRA-01");
        assert!(GameCode::new("ecra").is_err());
        assert!(GameCode::new("1-ECRA").is_err());
        assert!(GameCode::new("ECRA_GAME").is_err());
        assert!(GameCode::new("ABCDEFGHIJKLMNOPQ").is_err());
    }

    #[test]
    fn generation_is_deterministic_for_a_seed() {
        let options = GenerateGameOptions {
            seed: Some(42),
            ..GenerateGameOptions::default()
        };
        let first = generate_game(GameCode::new("FIRST").unwrap(), options).unwrap();
        let second = generate_game(GameCode::new("SECOND").unwrap(), options).unwrap();

        assert_eq!(first.seed, 42);
        assert_eq!(
            first.minimum_stellium_distance,
            DEFAULT_MINIMUM_STELLIUM_DISTANCE
        );
        assert_eq!(first.status, GameStatus::Setup);
        assert_eq!(
            first.agents,
            vec![Agent {
                id: UNCONTROLLED_AGENT_ID,
                kind: AgentKind::Uncontrolled,
            }]
        );
        assert!(first.factions.is_empty());
        assert_eq!(first.stellia, second.stellia);
        assert_eq!(first.stellia.len(), STELLIA_PER_GAME);
        assert!(
            first
                .stellia
                .iter()
                .all(|s| (1..=5).contains(&s.star_count()))
        );
        assert_valid_coordinates(&first, DEFAULT_MINIMUM_STELLIUM_DISTANCE);
    }

    #[test]
    fn different_seeds_generate_different_clusters() {
        let first = generate_game(
            GameCode::new("FIRST").unwrap(),
            GenerateGameOptions {
                seed: Some(1),
                ..GenerateGameOptions::default()
            },
        )
        .unwrap();
        let second = generate_game(
            GameCode::new("SECOND").unwrap(),
            GenerateGameOptions {
                seed: Some(2),
                ..GenerateGameOptions::default()
            },
        )
        .unwrap();

        assert_ne!(first.stellia, second.stellia);
    }

    #[test]
    fn honors_a_custom_minimum_distance() {
        let game = generate_game(
            GameCode::new("SPARSE").unwrap(),
            GenerateGameOptions {
                seed: Some(42),
                minimum_stellium_distance: 5,
            },
        )
        .unwrap();

        assert_valid_coordinates(&game, 5);
    }

    #[test]
    fn rejects_a_minimum_distance_that_cannot_fit_the_cluster() {
        let result = generate_game(
            GameCode::new("IMPOSSIBLE").unwrap(),
            GenerateGameOptions {
                seed: Some(42),
                minimum_stellium_distance: u8::MAX,
            },
        );

        assert!(result.is_err());
    }

    fn assert_valid_coordinates(game: &Game, minimum_distance: u8) {
        for (index, stellium) in game.stellia.iter().enumerate() {
            let coordinates = stellium.coordinates;
            assert!((MINIMUM_COORDINATE..=MAXIMUM_COORDINATE).contains(&coordinates.x));
            assert!((MINIMUM_COORDINATE..=MAXIMUM_COORDINATE).contains(&coordinates.y));
            assert!((MINIMUM_COORDINATE..=MAXIMUM_COORDINATE).contains(&coordinates.z));
            assert_ne!(coordinates, Coordinates { x: 0, y: 0, z: 0 });

            for other in &game.stellia[..index] {
                assert!(
                    coordinates.squared_distance(other.coordinates)
                        >= u32::from(minimum_distance).pow(2)
                );
            }
        }
    }
}
