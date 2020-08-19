//! Provides the `ServerStatus` struct, used to represent a server's status in terms of the server as a whole and the
//! in-game status.

use chrono::{DateTime, Duration, Utc};
use strum_macros::EnumString; // TODO: don't use these RPC enums, instead make own and convert to/from

/// Represent a server's status in terms of the server's execution and the in-game status.
#[derive(Debug, Copy, Clone)]
pub struct ServerStatus {
    /// The server executable's status.
    game_status: GameStatus,
    /// The in-game status.
    in_game_status: InGameStatus,
    /// Timestamp when the server was started.
    started_at: DateTime<Utc>,
}

/// Represents a server's execution status.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GameStatus {
    /// The executable is shut down.
    Shutdown = 0,
    /// The server is starting.
    Starting,
    /// The server is running.
    Running,
    /// The server is shutting down.
    ShuttingDown,
    /// The server is shut down after a crash.
    Crashed,
}

/// Represents the in-game status.
#[derive(Debug, EnumString, Eq, PartialEq, Copy, Clone)]
pub enum InGameStatus {
    /// The game is initialising.
    Initialising = 0,
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

impl Default for ServerStatus {
    fn default() -> Self {
        Self {
            game_status: GameStatus::Shutdown,
            in_game_status: InGameStatus::Initialising,
            started_at: Utc::now(),
        }
    }
}

impl ServerStatus {
    /// Returns the server executable's status.
    pub fn game_status(&self) -> GameStatus {
        self.game_status
    }

    /// Sets the server executable's status.
    pub fn set_game_status(&mut self, status: GameStatus) {
        self.game_status = status
    }

    /// Returns the server's in-game status.
    pub fn in_game_status(&self) -> InGameStatus {
        self.in_game_status
    }

    /// Sets the server executable's status.
    pub fn set_in_game_status(&mut self, status: InGameStatus) {
        self.in_game_status = status
    }

    /// Returns the server's uptime, or the time since the server was last started.
    pub fn get_uptime(&self) -> Duration {
        Utc::now() - self.started_at
    }

    /// Sets the server's started timestamp to the current time.
    pub fn reset_started_at(&mut self) {
        self.started_at = Utc::now()
    }
}

impl From<ServerStatus> for rpc::ServerStatus {
    fn from(status: ServerStatus) -> Self {
        Self {
            uptime: status.get_uptime().num_seconds(),
            status: status.game_status as i32,
            in_game_status: status.in_game_status as i32,
        }
    }
}
