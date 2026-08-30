use std::error::Error;
use std::fmt;

use crate::game::{FactionId, Game, ShipId};
use crate::orders::{LocatedOrder, Order, OrderFileOwner, ParsedOrderFile};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedOrder(LocatedOrder);

impl ValidatedOrder {
    pub fn line(&self) -> usize {
        self.0.line
    }

    pub fn order(&self) -> &Order {
        &self.0.order
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError {
    line: usize,
    explanation: String,
}

impl ValidationError {
    pub fn line(&self) -> usize {
        self.line
    }

    pub fn explanation(&self) -> &str {
        &self.explanation
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.explanation)
    }
}

impl Error for ValidationError {}

/// Validates the authority declared by `authenticate` against an immutable game.
///
/// A move commands its ship, and a transfer commands its source ship. A transfer's
/// destination is only a recipient, so the issuing faction need not own it.
pub fn validate_orders(
    game: &Game,
    order_file: &ParsedOrderFile,
) -> Result<Vec<ValidatedOrder>, Vec<ValidationError>> {
    if order_file.header.game != game.code {
        return Err(vec![ValidationError {
            line: 1,
            explanation: format!(
                "order file is for game `{}`, not `{}`",
                order_file.header.game, game.code
            ),
        }]);
    }

    let faction = match order_file.owner {
        OrderFileOwner::Faction(faction) => faction,
        _ => {
            return Err(vec![ValidationError {
                line: 2,
                explanation: "ship orders must authenticate a faction".to_owned(),
            }]);
        }
    };
    if !game
        .factions
        .iter()
        .any(|candidate| candidate.id == faction)
    {
        return Err(vec![ValidationError {
            line: 2,
            explanation: format!("faction {} does not exist", faction.number()),
        }]);
    }

    let mut validated = Vec::with_capacity(order_file.orders.len());
    let mut errors = Vec::new();
    for located in &order_file.orders {
        let commanded_ship = match &located.order {
            Order::Move { ship, .. } => *ship,
            Order::Transfer { source_ship, .. } => *source_ship,
        };
        match validate_ship_owner(game, faction, commanded_ship, located.line) {
            Ok(()) => validated.push(ValidatedOrder(located.clone())),
            Err(error) => errors.push(error),
        }
    }

    if errors.is_empty() {
        Ok(validated)
    } else {
        Err(errors)
    }
}

fn validate_ship_owner(
    game: &Game,
    faction: FactionId,
    ship: ShipId,
    line: usize,
) -> Result<(), ValidationError> {
    let Some(found) = game.ships.iter().find(|candidate| candidate.id == ship) else {
        return Err(ValidationError {
            line,
            explanation: format!("ship {} does not exist", ship.number()),
        });
    };
    if found.faction != faction {
        return Err(ValidationError {
            line,
            explanation: format!(
                "faction {} does not own ship {}",
                faction.number(),
                ship.number()
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{
        Faction, FactionController, GameCode, GenerateGameOptions, Ship, UNCONTROLLED_AGENT_ID,
        generate_game,
    };
    use crate::orders::parse_order_file;

    fn game() -> Game {
        let mut game = generate_game(
            GameCode::new("ECRA").unwrap(),
            GenerateGameOptions {
                seed: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        game.factions = vec![
            Faction {
                id: FactionId::new(7),
                controller: FactionController::Agent(UNCONTROLLED_AGENT_ID),
            },
            Faction {
                id: FactionId::new(8),
                controller: FactionController::Agent(UNCONTROLLED_AGENT_ID),
            },
        ];
        game.ships = vec![
            Ship {
                id: ShipId::new(1001),
                faction: FactionId::new(7),
            },
            Ship {
                id: ShipId::new(1002),
                faction: FactionId::new(8),
            },
        ];
        game
    }

    #[test]
    fn authenticated_faction_may_command_its_ships() {
        let parsed = parse_order_file(
            "orders.txt",
            concat!(
                "game ECRA turn 1;\n",
                "authenticate faction 7 with token \"unverified\";\n",
                "MOVE 1001 12;\n",
                "TRANSFER 1001 FOOD AVAILABLE 2 1002;\n",
            ),
        )
        .unwrap();

        let validated = validate_orders(&game(), &parsed).unwrap();

        assert_eq!(validated.len(), 2);
        assert_eq!(validated[0].line(), 3);
    }

    #[test]
    fn rejects_foreign_and_missing_commanded_ships_together() {
        let parsed = parse_order_file(
            "orders.txt",
            concat!(
                "game ECRA turn 1;\n",
                "authenticate faction 7 with token \"unverified\";\n",
                "MOVE 1002 12;\n",
                "TRANSFER 9999 FOOD AVAILABLE 2 1002;\n",
            ),
        )
        .unwrap();

        let errors = validate_orders(&game(), &parsed).unwrap_err();

        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].line(), 3);
        assert_eq!(errors[0].explanation(), "faction 7 does not own ship 1002");
        assert_eq!(errors[1].line(), 4);
        assert_eq!(errors[1].explanation(), "ship 9999 does not exist");
    }

    #[test]
    fn requires_faction_authentication_for_ship_orders() {
        let parsed = parse_order_file(
            "orders.txt",
            concat!(
                "game ECRA turn 1;\n",
                "authenticate player 7 with token \"unverified\";\n",
                "MOVE 1001 12;\n",
            ),
        )
        .unwrap();

        let errors = validate_orders(&game(), &parsed).unwrap_err();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line(), 2);
        assert_eq!(
            errors[0].explanation(),
            "ship orders must authenticate a faction"
        );
    }
}
