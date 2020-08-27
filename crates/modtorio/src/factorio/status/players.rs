//! Provides utilities to track the players in a server.

use crate::error::ServerError;
use chrono::{DateTime, Utc};
use log::*;
use std::{
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use strum_macros::EnumString;
use tokio::sync::Mutex;

/// Container for players currently in a server.
#[derive(Debug, Clone, Default)]
pub struct Players {
    /// The set of players currently in a server.
    players: Arc<Mutex<Vec<Player>>>,
    /// The index of the connecting player that was last modified.
    last_modified: Arc<AtomicUsize>,
}

/// A single player.
#[derive(Debug, Clone)]
pub struct Player {
    /// The player's username.
    username: Option<String>,
    /// The player's peer ID.
    peer_id: Option<i32>,
    /// The player's peer address, if any.
    peer_address: Option<SocketAddr>,
    /// Timestamp when the player last connected to the server.
    connection_time: DateTime<Utc>,
    /// Timestamp when the player joined the server.
    join_time: Option<DateTime<Utc>>,
    /// Timestamp when the player left the server.
    leave_time: Option<DateTime<Utc>>,
    /// The player's peer state.
    state: PeerState,
}

/// Represents a player's peer state.
#[derive(Debug, EnumString, Eq, PartialEq, Copy, Clone)]
pub enum PeerState {
    /// The peer is disconnected.
    Disconnected,
    /// The peer has opened a connection.
    Ready,
    /// The peer is waiting for the map download.
    ConnectedWaitingForMap,
    /// The peer is downloading the map.
    ConnectedDownloadingMap,
    /// The peer has downloaded the map and is loading it.
    ConnectedLoadingMap,
    /// The peer is catching up to the server.
    TryingToCatchUp,
    /// The peer is waiting for the server to start.
    WaitingForCommandToStartSendingTickClosures,
    /// The peer is in-game.
    InGame,
    /// The peer is disconnecting.
    DisconnectScheduled,
}

impl Players {
    /// Returns a list of the current players.
    pub async fn get(&self) -> Vec<Player> {
        self.players.lock().await.iter().cloned().collect()
    }

    /// Adds a new player with a given socket address in the `Ready`-state. Sets the last modified player index, thus
    /// creating state for a new connecting player.
    pub async fn connection_accepted(&self, address: &str) -> anyhow::Result<()> {
        let new_player = Player {
            peer_address: Some(address.parse()?),
            connection_time: Utc::now(),
            state: PeerState::Ready,
            username: None,
            peer_id: None,
            join_time: None,
            leave_time: None,
        };
        let mut players = self.players.lock().await;
        players.push(new_player);
        self.last_modified.store(players.len() - 1, Ordering::Relaxed);

        Ok(())
    }

    /// Sets the last modified player's peer ID.
    pub async fn new_peer(&self, id: &str) -> anyhow::Result<()> {
        let last_modified_index = self.last_modified.load(Ordering::Relaxed);
        let mut players = self.players.lock().await;
        let last_modified_player = match players.get_mut(last_modified_index) {
            Some(p) => p,
            None => return Err(ServerError::NoLastModifiedPlayer(last_modified_index, players.len()).into()),
        };

        last_modified_player.peer_id = Some(id.parse()?);
        Ok(())
    }

    /// Sets a player's state based on their peer ID.
    pub async fn peer_state_change(&self, id: &str, state: &str) -> anyhow::Result<()> {
        let mut players = self.players.lock().await;
        let id = id.parse()?;
        let (index, mut player) = match players.iter_mut().enumerate().find(|(_i, p)| p.peer_id == Some(id)) {
            Some(p) => p,
            None => return Err(ServerError::NoSuchPlayer(id.to_string()).into()),
        };

        let state = PeerState::from_str(state)?;
        player.state = state;

        // if the player changed state to InGame, it was a previously connecting player. update the last modified index
        if state == PeerState::InGame {
            self.last_modified.store(index, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Finalises a joining player either by creating a new player or updating a previous one.
    pub async fn joined(&self, username: &str) -> anyhow::Result<()> {
        let mut last_modified_player = {
            let last_modified_index = self.last_modified.load(Ordering::Relaxed);
            let mut players = self.players.lock().await;

            if last_modified_index >= players.len() {
                return Err(ServerError::NoLastModifiedPlayer(last_modified_index, players.len()).into());
            }

            players.remove(last_modified_index)
        };

        let mut players = self.players.lock().await;
        if let Some(already_existing_player) = players.iter_mut().find(|p| p.username.as_deref() == Some(username)) {
            if already_existing_player.state != PeerState::Disconnected {
                return Err(ServerError::PlayerAlreadyExists(username.to_string()).into());
            }

            debug!("Found previous player with the same username {}, updating", username);

            already_existing_player.peer_id = last_modified_player.peer_id;
            already_existing_player.peer_address = last_modified_player.peer_address;
            already_existing_player.state = last_modified_player.state;
            already_existing_player.connection_time = last_modified_player.connection_time;

            already_existing_player.join_time = Some(Utc::now());
            already_existing_player.leave_time = None;
        } else {
            debug!("No previous player with username {}, adding new", username);

            last_modified_player.username = Some(username.to_string());
            last_modified_player.join_time = Some(Utc::now());
            last_modified_player.leave_time = None;

            players.push(last_modified_player);
        }

        Ok(())
    }

    /// Removes a player identified by their username.
    pub async fn remove(&self, username: &str) -> anyhow::Result<()> {
        let mut players = self.players.lock().await;
        let mut player = match players.iter_mut().find(|p| p.username.as_deref() == Some(username)) {
            Some(p) => p,
            None => return Err(ServerError::NoSuchPlayer(username.to_string()).into()),
        };

        player.state = PeerState::Disconnected;
        player.leave_time = Some(Utc::now());
        Ok(())
    }
}

impl From<PeerState> for rpc::server_status::player::PlayerStatus {
    fn from(state: PeerState) -> Self {
        match state {
            PeerState::Disconnected => Self::Disconnected,
            PeerState::Ready => Self::Ready,
            PeerState::ConnectedWaitingForMap => Self::ConnectedWaitingForMap,
            PeerState::ConnectedDownloadingMap => Self::ConnectedDownloadingMap,
            PeerState::ConnectedLoadingMap => Self::ConnectedLoadingMap,
            PeerState::TryingToCatchUp => Self::TryingToCatchUp,
            PeerState::WaitingForCommandToStartSendingTickClosures => Self::WaitingForCommandToStartSendingTickClosures,
            PeerState::InGame => Self::InGame,
            PeerState::DisconnectScheduled => Self::DisconnectScheduled,
        }
    }
}

impl From<Player> for rpc::server_status::Player {
    fn from(p: Player) -> Self {
        Self {
            username: p.username.unwrap_or_else(|| "unknown".to_string()),
            peer_address: p.peer_address.map(rpc::SocketAddr::from),
            status: rpc::server_status::player::PlayerStatus::from(p.state) as i32,
            connection_time: p.connection_time.to_rfc3339(),
            join_time: p.join_time.map_or_else(String::new, |t| t.to_rfc3339()),
            leave_time: p.leave_time.map_or_else(String::new, |t| t.to_rfc3339()),
        }
    }
}
