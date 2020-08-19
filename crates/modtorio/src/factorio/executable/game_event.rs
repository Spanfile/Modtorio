//! Provides the `GameEvent` enum which represents a single event that happened in-game in a server.

use crate::{error::GameEventError, factorio::status::InGameStatus};
use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;

/// Represents a single event that happened in-game in a server.
#[derive(Debug)]
pub enum GameEvent {
    /// The game's state changed.
    GameStateChanged {
        /// The previous state.
        from: InGameStatus,
        /// The current state.
        to: InGameStatus,
    },
    /// A peer's connection was refused.
    RefusingConnection {
        /// The peer's address-port-pair.
        peer: String,
        /// The peer's username.
        username: String,
        /// Reason the connection was refused.
        reason: String,
    },
    /// A peer's state changed.
    PeerStateChanged {
        /// The peer's ID.
        peer_id: String,
        /// The peer's previous state.
        old_state: String,
        /// The peer's current state.
        new_state: String,
    },
    /// A peer joined the game.
    PeerJoined {
        /// The peer's username
        username: String,
    },
    /// A peer left the game.
    PeerLeft {
        /// The peer's username
        username: String,
    },
}

/// Type of the string parser functions.
type ParserFn = fn(&str) -> Option<GameEvent>;
lazy_static! {
    static ref PARSERS: Vec<ParserFn> = vec![
        factorio_initialised,
        game_state_changed,
        refusing_connection,
        peer_state_change,
        peer_joined,
        peer_left
    ];
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
            from: InGameStatus::Initialising,
            to: InGameStatus::Ready,
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
    let from = InGameStatus::from_str(captures.get(1)?.as_str()).ok()?;
    let to = InGameStatus::from_str(captures.get(2)?.as_str()).ok()?;

    Some(GameEvent::GameStateChanged { from, to })
}

/// Parses the peer connection refused message into `GameEvent::RefusingConnection`.
fn refusing_connection(s: &str) -> Option<GameEvent> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"Refusing connection for address \(IP ADDR:\(\{(\S+)\}\)\), username \((\S+)\)\. (\S+)"#)
                .expect("failed to create connection refused regex");
    }

    let captures = RE.captures(s)?;
    let peer = captures.get(1)?.as_str().to_owned();
    let username = captures.get(2)?.as_str().to_owned();
    let reason = captures.get(3)?.as_str().to_owned();

    Some(GameEvent::RefusingConnection { peer, username, reason })
}

/// Parses the peer state change message into `GameEvent::PeerStateChanged`.
fn peer_state_change(s: &str) -> Option<GameEvent> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"received stateChanged peerID\((\S+)\) oldState\((\S+)\) newState\((\S+)\)"#)
                .expect("failed to create peer state change regex");
    }

    let captures = RE.captures(s)?;
    let peer_id = captures.get(1)?.as_str().to_owned();
    let old_state = captures.get(2)?.as_str().to_owned();
    let new_state = captures.get(3)?.as_str().to_owned();

    Some(GameEvent::PeerStateChanged {
        peer_id,
        old_state,
        new_state,
    })
}

/// Parses the peer join message into `GameEvent::PeerJoined`.
fn peer_joined(s: &str) -> Option<GameEvent> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"\[JOIN\] (\S+) joined the game"#).expect("failed to create peer join regex");
    }

    let captures = RE.captures(s)?;
    let username = captures.get(1)?.as_str().to_owned();

    Some(GameEvent::PeerJoined { username })
}

/// Parses the peer leave message into `GameEvent::PeerLeft`.
fn peer_left(s: &str) -> Option<GameEvent> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"\[LEAVE\] (\S+) left the game"#).expect("failed to create peer leave regex");
    }

    let captures = RE.captures(s)?;
    let username = captures.get(1)?.as_str().to_owned();

    Some(GameEvent::PeerLeft { username })
}
