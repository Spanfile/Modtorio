//! Provides objects that represent a Factorio server's state, both the executable's and the server's in-game state.

/// Represesnts the server executable's state.
#[derive(Debug)]
pub enum ExecutableState {
    /// Represents the in-game state.
    GameState(GameState),
    /// The executable exited with a given result.
    Exited(anyhow::Result<()>),
}

/// Represents the in-game state.
#[derive(Debug)]
pub enum GameState {
    /// The game is being created.
    Ready,
    /// The server is preparing to host the game.
    PreparedToHostGame,
    /// The server is creating the game.
    CreatingGame,
    /// The game is running.
    InGame,
    /// The game is preparing to disconnect.
    DisconnectingScheduled,
    /// The game is disconnecting.
    Disconnecting,
    /// The game is closed.
    Closed,
}
