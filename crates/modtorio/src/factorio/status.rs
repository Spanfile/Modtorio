//! Provides the `ServerStatus` struct, used to represent a server's status in terms of the server as a whole and the
//! in-game status.

mod players;

use chrono::{DateTime, Duration, Utc};
use strum_macros::EnumString;

pub use players::{Player, Players};

/// Represent a server's status in terms of the server's execution and the in-game status.
#[derive(Debug, Clone)]
pub struct ServerStatus {
    /// The server executable's status.
    game_status: ExecutionStatus,
    /// The in-game status.
    in_game_status: InGameStatus,
    /// Timestamp when the server was started.
    started_at: DateTime<Utc>,
    /// Players on the server.
    players: Players,
}

/// Represents a server's execution status.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ExecutionStatus {
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
            game_status: ExecutionStatus::Shutdown,
            in_game_status: InGameStatus::Initialising,
            started_at: Utc::now(),
            players: Players::default(),
        }
    }
}

impl ServerStatus {
    /// Returns the server executable's status.
    pub fn game_status(&self) -> ExecutionStatus {
        self.game_status
    }

    /// Sets the server executable's status.
    pub fn set_game_status(&mut self, status: ExecutionStatus) {
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

    /// Returns a list of the current players.
    pub async fn players(&self) -> Vec<Player> {
        self.players.players().await
    }

    /// Adds a new player.
    pub async fn add_player(&self, username: &str) -> anyhow::Result<()> {
        self.players.add_player(username.to_owned()).await
    }

    /// Removes an existing player.
    pub async fn remove_player(&self, username: &str) -> anyhow::Result<()> {
        self.players.remove_player(username).await
    }
}

impl ServerStatus {
    /// Returns a new RPC `ServerStatus` from this object.
    pub async fn to_rpc_server_status(&self) -> rpc::ServerStatus {
        rpc::ServerStatus {
            uptime: self.get_uptime().num_seconds(),
            status: self.game_status as i32,
            in_game_status: self.in_game_status as i32,
            players: self.players().await.into_iter().map(|p| p.into()).collect(),
        }
    }
}
