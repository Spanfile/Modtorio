//! Provides utilities to track the players in a server.

use crate::error::ServerError;
use chrono::{DateTime, Utc};
use std::{borrow::Borrow, collections::HashSet, hash::Hash, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

/// Container for players currently in a server.
#[derive(Debug, Clone, Default)]
pub struct Players {
    /// The set of players currently in a server.
    players: Arc<Mutex<HashSet<Player>>>,
}

/// A single player.
#[derive(Debug, Clone)]
pub struct Player {
    /// The player's username.
    username: String,
    /// The player's peer address, if any.
    peer_address: Option<SocketAddr>,
    /// The player's join time.
    join_time: DateTime<Utc>,
}

impl Players {
    /// Returns a list of the current players.
    pub async fn players(&self) -> Vec<Player> {
        self.players.lock().await.iter().cloned().collect()
    }

    /// Adds a player with a given username. Returns `ServerError::PlayerAlreadyExists` if a player with the given name
    /// already exists in the server.
    pub async fn add_player(&self, username: String) -> anyhow::Result<()> {
        if self.players.lock().await.insert(Player {
            username: username.clone(),
            peer_address: None,
            join_time: Utc::now(),
        }) {
            Ok(())
        } else {
            Err(ServerError::PlayerAlreadyExists(username).into())
        }
    }

    /// Removes a player with a given username. Returns `ServerError::NoSuchPlayer` if the server doesn't have a player
    /// with the given name.
    pub async fn remove_player(&self, username: &str) -> anyhow::Result<()> {
        if self.players.lock().await.remove(username) {
            Ok(())
        } else {
            Err(ServerError::NoSuchPlayer(username.to_owned()).into())
        }
    }
}

impl From<Player> for rpc::server_status::Player {
    fn from(p: Player) -> Self {
        Self {
            username: p.username,
            peer_address: p.peer_address.map(rpc::SocketAddr::from),
            session_time: (Utc::now() - p.join_time).num_seconds(),
        }
    }
}

impl Eq for Player {}
impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        // the compared values have to be the same as the ones in the Hash impl below
        self.username == other.username
    }
}

impl Hash for Player {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.username.hash(state)
    }
}

impl PartialEq<String> for Player {
    fn eq(&self, other: &String) -> bool {
        &self.username == other
    }
}

impl Borrow<String> for Player {
    fn borrow(&self) -> &String {
        &self.username
    }
}

impl Borrow<str> for Player {
    fn borrow(&self) -> &str {
        &self.username
    }
}
