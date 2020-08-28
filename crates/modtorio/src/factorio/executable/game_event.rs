//! Provides the `GameEvent` enum which represents a single event that happened in-game in a server.

use crate::{error::GameEventError, factorio::status::InGameStatus};
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use log::*;
use regex::Regex;
use std::str::FromStr;

/// Represents a single event that happened in-game in a server.
#[derive(Debug)]
pub struct GameEvent {
    /// The timestamp when the event happened.
    pub timestamp: DateTime<Utc>,
    /// The type of the event.
    pub event_type: EventType,
}

/// Represents the type of event that happened in-game in a server.
#[derive(Debug)]
pub enum EventType {
    /// The game's state changed.
    GameStateChanged {
        /// The previous state.
        from: InGameStatus,
        /// The current state.
        to: InGameStatus,
    },
    /// A player's connection was refused.
    RefusingConnection {
        /// The peer's address-port-pair.
        address: String,
        /// The peer's username.
        username: String,
        /// Reason the connection was refused.
        reason: String,
    },
    /// A player's connection was accepted.
    ConnectionAccepted {
        /// The peer's address-port pair.
        address: String,
    },
    /// A new peer was added.
    NewPeer {
        /// The peer's ID
        id: String,
    },
    /// A peer was removed.
    PeerRemoved {
        /// The peer's ID.
        id: String,
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
    /// A player joined the game.
    PlayerJoined {
        /// The peer's username
        username: String,
    },
    /// A player left the game.
    PlayerLeft {
        /// The peer's username
        username: String,
    },
    /// The server is saving the map.
    SavingMap {
        /// The filename of the save.
        filename: String,
    },
    /// The server finished saving the map.
    SavingFinished,
    /// A player was banned from the server.
    PlayerBanned {
        /// The player that was banned.
        player: String,
        /// The player (or <server>) who banned the player.
        banned_by: String,
        /// The reason the player was banned.
        reason: String,
    },
    /// A player was unbanned from the server.
    PlayerUnbanned {
        /// The player was unbanned.
        player: String,
        /// The player (or <server>) who unbanned the player.
        unbanned_by: String,
    },
    /// A player was kicked from the server.
    PlayerKicked {
        /// The player that was kicked.
        player: String,
        /// The player (or <server>) who kicked the player.
        kicked_by: String,
        /// The reason the player was kicked.
        reason: String,
    },
    /// A player was promoted to admin.
    PlayerPromoted {
        /// The player that was promoted.
        player: String,
        /// The player (or <server>) who promoted the player.
        promoted_by: String,
    },
    /// A player was demoted from admin.
    PlayerDemoted {
        /// The player that was demoted.
        player: String,
        /// The player (or <server>) who demoted the player.
        demoted_by: String,
    },
    /// A player sent a chat message.
    Chat {
        /// The player who sent the message.
        player: String,
        /// The message.
        message: String,
    },
}

/// Type of the string parser functions.
type ParserFn = fn(&str) -> Option<EventType>;
lazy_static! {
    static ref PARSERS: Vec<ParserFn> = vec![
        factorio_initialised,
        game_state_changed,
        refusing_connection,
        connection_accepted,
        new_peer,
        peer_removed,
        peer_state_change,
        player_joined,
        player_left,
        saving_map,
        saving_finished,
        player_banned,
        player_unbanned,
        player_kicked,
        player_promoted,
        player_demoted,
        chat,
    ];
}

impl GameEvent {
    /// Returns a new `GameEvent` from a given timestamp and server stdout line.
    pub fn new(timestamp: DateTime<Utc>, line: &str) -> Result<GameEvent, GameEventError> {
        Ok(Self {
            timestamp,
            event_type: line.parse()?,
        })
    }
}

impl FromStr for EventType {
    type Err = GameEventError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let start_time = chrono::Utc::now();
        for parser in PARSERS.iter() {
            if let Some(event) = parser(s) {
                let duration = chrono::Utc::now() - start_time;
                trace!("Parsing GameEvent took {}ms", duration.num_milliseconds());
                return Ok(event);
            }
        }

        Err(GameEventError::FailedToParse(s.to_owned()))
    }
}

/// Parses the "Factorio initialised" message into `EventType::GameStateChanged`.
fn factorio_initialised(s: &str) -> Option<EventType> {
    if s.ends_with("Factorio initialised") {
        Some(EventType::GameStateChanged {
            from: InGameStatus::Initialising,
            to: InGameStatus::Ready,
        })
    } else {
        None
    }
}

/// Parses the game's state change message into `EventType::GameStateChanged`.
fn game_state_changed(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"changing state from\((\w*)\) to\((\w*)\)"#)
            .expect("failed to create game state change regex");
    }

    let captures = RE.captures(s)?;
    let from = InGameStatus::from_str(captures.get(1)?.as_str()).ok()?;
    let to = InGameStatus::from_str(captures.get(2)?.as_str()).ok()?;

    Some(EventType::GameStateChanged { from, to })
}

/// Parses the connection refused message into `EventType::RefusingConnection`.
fn refusing_connection(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"Refusing connection for address \(IP ADDR:\(\{(\S+)\}\)\), username \((\S+)\)\. (\S+)"#)
                .expect("failed to create connection refused regex");
    }

    let captures = RE.captures(s)?;
    let address = captures.get(1)?.as_str().to_owned();
    let username = captures.get(2)?.as_str().to_owned();
    let reason = captures.get(3)?.as_str().to_owned();

    Some(EventType::RefusingConnection {
        address,
        username,
        reason,
    })
}

/// Parses the connection accepted message into `EventType::ConnectionAccepted`.
fn connection_accepted(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"Replying to connectionRequest for address\(IP ADDR:\(\{(\S+)\}\)\)\."#)
            .expect("failed to create connection accepted regex");
    }

    let captures = RE.captures(s)?;
    let address = captures.get(1)?.as_str().to_owned();

    Some(EventType::ConnectionAccepted { address })
}

/// Parses the new peer message into `EventType::RefusingConnection`.
fn new_peer(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"adding peer\((\w+)\)"#).expect("failed to create new peer regex");
    }

    let captures = RE.captures(s)?;
    let id = captures.get(1)?.as_str().to_owned();

    Some(EventType::NewPeer { id })
}

/// Parses the removing peer message into `EventType::PeerRemoved`.
fn peer_removed(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"removing peer\((\w+)\)"#).expect("failed to create peer remove regex");
    }

    let captures = RE.captures(s)?;
    let id = captures.get(1)?.as_str().to_owned();

    Some(EventType::PeerRemoved { id })
}

/// Parses the peer state change message into `EventType::PeerStateChanged`.
fn peer_state_change(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"received stateChanged peerID\((\S+)\) oldState\((\S+)\) newState\((\S+)\)"#)
                .expect("failed to create peer state change regex");
    }

    let captures = RE.captures(s)?;
    let peer_id = captures.get(1)?.as_str().to_owned();
    let old_state = captures.get(2)?.as_str().to_owned();
    let new_state = captures.get(3)?.as_str().to_owned();

    Some(EventType::PeerStateChanged {
        peer_id,
        old_state,
        new_state,
    })
}

/// Parses the player join message into `EventType::PlayerJoined`.
fn player_joined(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"\[JOIN\] (\S+) joined the game"#).expect("failed to create player join regex");
    }

    let captures = RE.captures(s)?;
    let username = captures.get(1)?.as_str().to_owned();

    Some(EventType::PlayerJoined { username })
}

/// Parses the player leave message into `EventType::PlayerLeft`.
fn player_left(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"\[LEAVE\] (\S+) left the game"#).expect("failed to create player leave regex");
    }

    let captures = RE.captures(s)?;
    let username = captures.get(1)?.as_str().to_owned();

    Some(EventType::PlayerLeft { username })
}

/// Parses the map saving message into `EventType::SavingMap`.
fn saving_map(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"Saving (?:game|map) as (.+)"#).expect("failed to create saving map regex");
    }

    let captures = RE.captures(s)?;
    let filename = captures.get(1)?.as_str().to_owned();

    Some(EventType::SavingMap { filename })
}

/// Parses the map saving finished message into `EventType::SavingFinished`.
fn saving_finished(s: &str) -> Option<EventType> {
    if s.ends_with("Saving finished") {
        Some(EventType::SavingFinished)
    } else {
        None
    }
}

/// Parses the player ban message into `EventType::PlayerBanned`.
fn player_banned(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"\[BAN\] (.+?) (?:\(not on map\) )?was banned by (.+?)\. Reason: (.+)\."#)
            .expect("failed to create player banned regex");
    }

    let captures = RE.captures(s)?;
    let player = captures.get(1)?.as_str().to_owned();
    let banned_by = captures.get(2)?.as_str().to_owned();
    let reason = captures.get(3)?.as_str().to_owned();

    Some(EventType::PlayerBanned {
        player,
        banned_by,
        reason,
    })
}

/// Parses the player unban message into `EventType::PlayerUnbanned`.
fn player_unbanned(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"\[UNBANNED\] (.+?) was unbanned by (.+?)\."#)
            .expect("failed to create player unbanned regex");
    }

    let captures = RE.captures(s)?;
    let player = captures.get(1)?.as_str().to_owned();
    let unbanned_by = captures.get(2)?.as_str().to_owned();

    Some(EventType::PlayerUnbanned { player, unbanned_by })
}

/// Parses the player kick message into `EventType::PlayerKicked`.
fn player_kicked(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"\[KICK\] (.+?) was kicked by (.+?)\. Reason: (.+)\."#)
            .expect("failed to create player kicked regex");
    }

    let captures = RE.captures(s)?;
    let player = captures.get(1)?.as_str().to_owned();
    let kicked_by = captures.get(2)?.as_str().to_owned();
    let reason = captures.get(3)?.as_str().to_owned();

    Some(EventType::PlayerKicked {
        player,
        kicked_by,
        reason,
    })
}

/// Parses the player promote message into `EventType::PlayerPromoted`.
fn player_promoted(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"\[PROMOTE\] (.+?) was promoted to admin by (.+?)\."#)
            .expect("failed to create player promoted regex");
    }

    let captures = RE.captures(s)?;
    let player = captures.get(1)?.as_str().to_owned();
    let promoted_by = captures.get(2)?.as_str().to_owned();

    Some(EventType::PlayerPromoted { player, promoted_by })
}

/// Parses the player demote message into `EventType::PlayerDemoted`.
fn player_demoted(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"\[DEMOTE\] (.+?) was demoted from admin by (.+?)\."#)
            .expect("failed to create player demoted regex");
    }

    let captures = RE.captures(s)?;
    let player = captures.get(1)?.as_str().to_owned();
    let demoted_by = captures.get(2)?.as_str().to_owned();

    Some(EventType::PlayerDemoted { player, demoted_by })
}

/// Parses the chat message into `EventType::Chat`.
fn chat(s: &str) -> Option<EventType> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"\[CHAT\] (.+): (.+)"#).expect("failed to create chat regex");
    }

    let captures = RE.captures(s)?;
    let player = captures.get(1)?.as_str().to_owned();
    let message = captures.get(2)?.as_str().to_owned();

    Some(EventType::Chat { player, message })
}
