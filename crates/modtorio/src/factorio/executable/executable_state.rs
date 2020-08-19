//! Provides objects that represent a Factorio server's state, both the executable's and the server's in-game state.

use super::game_event::GameEvent;
use strum_macros::EnumString;

/// Represesnts an event that happened with the executable.
#[derive(Debug)]
pub enum ExecutableEvent {
    /// Represents an event that happened in the server.
    GameEvent(GameEvent),
    /// The executable exited with a given result.
    Exited(anyhow::Result<()>),
}

/// Represents the in-game state.
#[derive(Debug, EnumString, Eq, PartialEq)]
pub enum GameState {
    /// The game is initialising.
    Initialising,
    /// The game is initialised.
    Ready,
    /// The server is preparing to host the game.
    PreparedToHostGame,
    /// The server is creating the game.
    CreatingGame,
    /// The game is running.
    InGame,
    /// The game is saving the map.
    InGameSavingMap,
    /// The game is preparing to disconnect.
    DisconnectingScheduled,
    /// The game is disconnecting.
    Disconnecting,
    /// The game is closed.
    Closed,
}
