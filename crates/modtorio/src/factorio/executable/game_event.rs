use super::GameState;
use crate::error::GameEventError;
use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;

/// Describes an event that happened in-game in a server.
#[derive(Debug)]
pub enum GameEvent {
    /// The game's state changed.
    GameStateChanged {
        /// The previous state.
        from: GameState,
        /// The current state.
        to: GameState,
    },
}

/// Type of the string parser functions.
type ParserFn = fn(&str) -> Option<GameEvent>;
lazy_static! {
    static ref PARSERS: Vec<ParserFn> = vec![factorio_initialised, game_state_changed];
}

impl FromStr for GameEvent {
    type Err = GameEventError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for parser in PARSERS.iter() {
            if let Some(event) = parser(s) {
                return Ok(event);
            }
        }

        Err(GameEventError::FailedToParse(s.to_owned()))
    }
}

/// Parses the "Factorio initialised" message into `GameEvent::GameStateChanged`.
fn factorio_initialised(s: &str) -> Option<GameEvent> {
    if s.ends_with("Factorio initialised") {
        Some(GameEvent::GameStateChanged {
            from: GameState::Initialising,
            to: GameState::Ready,
        })
    } else {
        None
    }
}

/// Parses the game's state change message into `GameEvent::GameStateChanged`.
fn game_state_changed(s: &str) -> Option<GameEvent> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"changing state from\((\w*)\) to\((\w*)\)"#)
            .expect("failed to create game state change regex");
    }

    let captures = RE.captures(s)?;
    let from = GameState::from_str(captures.get(1)?.as_str()).ok()?;
    let to = GameState::from_str(captures.get(2)?.as_str()).ok()?;

    Some(GameEvent::GameStateChanged { from, to })
}
