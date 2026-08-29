use std::fmt;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use thiserror::Error;

pub const STELLIA_PER_GAME: usize = 100;
const STELLIUM_STREAM: u64 = 0x5354_454c_4c49_554d;

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
    pub stars: Vec<Star>,
}

impl Stellium {
    pub fn star_count(&self) -> usize {
        self.stars.len()
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Game {
    pub code: GameCode,
    pub seed: u64,
    pub status: GameStatus,
    pub stellia: Vec<Stellium>,
}

pub fn generate_game(code: GameCode, seed: Option<u64>) -> Game {
    let seed = seed.unwrap_or_else(rand::random);
    let mut stellium_rng = ChaCha8Rng::seed_from_u64(stream_seed(seed, STELLIUM_STREAM));
    let stellia = (1..=STELLIA_PER_GAME)
        .map(|number| {
            let star_count = stellium_rng.random_range(1..=5);
            Stellium {
                id: StelliumId::new(number as u32),
                stars: (1..=star_count).map(|id| Star { id: StarId(id) }).collect(),
            }
        })
        .collect();

    Game {
        code,
        seed,
        status: GameStatus::Setup,
        stellia,
    }
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
        let first = generate_game(GameCode::new("FIRST").unwrap(), Some(42));
        let second = generate_game(GameCode::new("SECOND").unwrap(), Some(42));

        assert_eq!(first.seed, 42);
        assert_eq!(first.status, GameStatus::Setup);
        assert_eq!(first.stellia, second.stellia);
        assert_eq!(first.stellia.len(), STELLIA_PER_GAME);
        assert!(
            first
                .stellia
                .iter()
                .all(|s| (1..=5).contains(&s.star_count()))
        );
    }

    #[test]
    fn different_seeds_generate_different_clusters() {
        let first = generate_game(GameCode::new("FIRST").unwrap(), Some(1));
        let second = generate_game(GameCode::new("SECOND").unwrap(), Some(2));

        assert_ne!(first.stellia, second.stellia);
    }
}
