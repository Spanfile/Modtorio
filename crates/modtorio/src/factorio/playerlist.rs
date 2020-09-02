//! Provides the `Playerlist` struct which is used to manage the various server playerlists, such as the banlist and the
//! adminlist.

use crate::error::ServerError;
use log::*;
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

/// Manages one of the server's playerlists.
#[derive(Debug)]
pub struct Playerlist {
    /// The path where this playerlist is stored.
    path: PathBuf,
    /// The actual playerlist.
    playerlist: Vec<String>,
}

impl Playerlist {
    /// Loads a new playerlist from a given playerlist file.
    pub fn from_file<P>(banlist_file: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(&banlist_file)?;
        let reader = BufReader::new(file);
        let playerlist: Vec<String> = serde_json::from_reader(reader)?;
        debug!("Loaded playerlist: {:?}", playerlist);

        Ok(Playerlist {
            path: banlist_file.as_ref().to_path_buf(),
            playerlist,
        })
    }

    /// Saves this playerlist to the file it was originally read from.
    pub fn save(&self) -> anyhow::Result<()> {
        debug!("Saving playerlist {:?} to {}", self.playerlist, self.path.display());

        let file = File::create(&self.path)?;
        serde_json::to_writer(file, &self.playerlist)?;
        Ok(())
    }

    /// Adds a player to the playerlist.
    pub fn add(&mut self, player: &str) -> anyhow::Result<()> {
        let player = player.to_lowercase();
        for p in &self.playerlist {
            if p == &player {
                return Err(ServerError::PlayerAlreadyExists(player).into());
            }
        }

        self.playerlist.push(player);
        Ok(())
    }

    /// Removes a player from the playerlist.
    pub fn remove(&mut self, player: &str) -> anyhow::Result<()> {
        let player = player.to_lowercase();
        for (i, p) in self.playerlist.iter().enumerate() {
            if p == &player {
                self.playerlist.remove(i);
                return Ok(());
            }
        }

        Err(ServerError::NoSuchPlayer(player).into())
    }
}
