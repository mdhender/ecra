use serde::Serialize;

use crate::agents::available_agents;
use crate::game::{Agent, FactionController, Game};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AgentListEntry {
    pub id: u64,
    pub identifier: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AgentFactionsListEntry {
    pub id: u64,
    pub identifier: String,
    pub name: String,
    pub factions: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlayerListEntry {
    pub email: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct StelliumListEntry {
    pub id: u32,
    pub x: i8,
    pub y: i8,
    pub z: i8,
    pub stars: u8,
}

pub fn available_agent_list() -> Vec<AgentListEntry> {
    agent_list(available_agents())
}

pub fn game_agent_list(game: &Game) -> Vec<AgentListEntry> {
    agent_list(&game.agents)
}

pub fn agent_factions_list(game: &Game) -> Vec<AgentFactionsListEntry> {
    let mut entries = game
        .agents
        .iter()
        .map(|agent| {
            let mut factions = game
                .factions
                .iter()
                .filter_map(|faction| match faction.controller {
                    FactionController::Agent(id) if id == agent.id => Some(faction.id.number()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            factions.sort_unstable();
            AgentFactionsListEntry {
                id: agent.id.number(),
                identifier: agent.kind.identifier().to_owned(),
                name: agent.kind.display_name().to_owned(),
                factions,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.id);
    entries
}

pub fn agent_list_json(entries: &[AgentListEntry]) -> Result<String, serde_json::Error> {
    list_json(entries)
}

pub fn agent_factions_list_json(
    entries: &[AgentFactionsListEntry],
) -> Result<String, serde_json::Error> {
    list_json(entries)
}

fn agent_list(agents: &[Agent]) -> Vec<AgentListEntry> {
    let mut entries = agents
        .iter()
        .map(|agent| AgentListEntry {
            id: agent.id.number(),
            identifier: agent.kind.identifier().to_owned(),
            name: agent.kind.display_name().to_owned(),
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.id);
    entries
}

pub fn stellium_list(game: &Game) -> Vec<StelliumListEntry> {
    let mut entries = game
        .stellia
        .iter()
        .map(|stellium| StelliumListEntry {
            id: stellium.id.number(),
            x: stellium.coordinates.x,
            y: stellium.coordinates.y,
            z: stellium.coordinates.z,
            stars: stellium.star_count() as u8,
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| (entry.x, entry.y, entry.z, entry.id));
    entries
}

pub fn stellium_list_json(entries: &[StelliumListEntry]) -> Result<String, serde_json::Error> {
    list_json(entries)
}

pub fn player_list(game: &Game) -> Vec<PlayerListEntry> {
    let mut entries = game
        .players
        .iter()
        .map(|player| PlayerListEntry {
            email: player.email.clone(),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.email.cmp(&right.email));
    entries
}

pub fn player_list_json(entries: &[PlayerListEntry]) -> Result<String, serde_json::Error> {
    list_json(entries)
}

fn list_json<T: Serialize + ?Sized>(entries: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(entries).map(|mut json| {
        json.push('\n');
        json
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{GameCode, GenerateGameOptions, generate_game};

    #[test]
    fn stellia_are_sorted_by_coordinates_with_id_as_a_tie_breaker() {
        let game = generate_game(
            GameCode::new("REPORT").unwrap(),
            GenerateGameOptions {
                seed: Some(42),
                ..Default::default()
            },
        )
        .unwrap();

        let entries = stellium_list(&game);

        assert_eq!(entries.len(), 100);
        assert!(entries.windows(2).all(|pair| {
            (pair[0].x, pair[0].y, pair[0].z, pair[0].id)
                < (pair[1].x, pair[1].y, pair[1].z, pair[1].id)
        }));
    }

    #[test]
    fn json_is_pretty_printed_with_a_trailing_newline() {
        let json = stellium_list_json(&[StelliumListEntry {
            id: 7,
            x: -2,
            y: 3,
            z: 4,
            stars: 5,
        }])
        .unwrap();

        assert_eq!(
            json,
            "[\n  {\n    \"id\": 7,\n    \"x\": -2,\n    \"y\": 3,\n    \"z\": 4,\n    \"stars\": 5\n  }\n]\n"
        );
    }

    #[test]
    fn players_are_sorted_by_email() {
        let mut game = generate_game(
            GameCode::new("PLAYERS").unwrap(),
            GenerateGameOptions::default(),
        )
        .unwrap();
        game.players = vec![
            crate::game::Player {
                id: crate::game::PlayerId::new(2),
                email: "zoe@example.com".to_owned(),
            },
            crate::game::Player {
                id: crate::game::PlayerId::new(1),
                email: "amy@example.com".to_owned(),
            },
        ];

        assert_eq!(
            player_list(&game),
            vec![
                PlayerListEntry {
                    email: "amy@example.com".to_owned()
                },
                PlayerListEntry {
                    email: "zoe@example.com".to_owned()
                }
            ]
        );
    }

    #[test]
    fn available_agents_come_from_the_engine_registry() {
        assert_eq!(
            available_agent_list(),
            vec![AgentListEntry {
                id: 1,
                identifier: "uncontrolled".to_owned(),
                name: "Uncontrolled".to_owned(),
            }]
        );
    }

    #[test]
    fn agent_factions_include_unassigned_agents_and_sorted_factions() {
        let mut game = generate_game(
            GameCode::new("AGENTS").unwrap(),
            GenerateGameOptions::default(),
        )
        .unwrap();
        game.factions = vec![
            crate::game::Faction {
                id: crate::game::FactionId::new(9),
                controller: crate::game::FactionController::Agent(
                    crate::game::UNCONTROLLED_AGENT_ID,
                ),
            },
            crate::game::Faction {
                id: crate::game::FactionId::new(2),
                controller: crate::game::FactionController::Agent(
                    crate::game::UNCONTROLLED_AGENT_ID,
                ),
            },
        ];

        assert_eq!(
            agent_factions_list(&game),
            vec![AgentFactionsListEntry {
                id: 1,
                identifier: "uncontrolled".to_owned(),
                name: "Uncontrolled".to_owned(),
                factions: vec![2, 9],
            }]
        );
    }
}
