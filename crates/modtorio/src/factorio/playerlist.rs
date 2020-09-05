//! Provides the `Playerlist` struct which is used to manage the various server playerlists, such as the banlist and the
//! adminlist.

use crate::error::ServerError;
use log::*;
use serde::{de, Serialize};
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

/// Manages one of the server's playerlists.
#[derive(Debug)]
pub struct Playerlist<T> {
    /// The path where this playerlist is stored.
    path: PathBuf,
    /// The actual playerlist.
    playerlist: Vec<T>,
}

impl<T> Playerlist<T>
where
    T: Serialize + de::DeserializeOwned + std::fmt::Debug + std::string::ToString + PartialEq,
{
    /// Loads a new playerlist from a given playerlist file.
    pub fn from_file<P>(banlist_file: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(&banlist_file)?;
        let reader = BufReader::new(file);
        let playerlist: Vec<T> = serde_json::from_reader(reader)?;
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
    pub fn add(&mut self, player: T) -> anyhow::Result<()> {
        for p in &self.playerlist {
            if p == &player {
                return Err(ServerError::PlayerAlreadyExists(player.to_string()).into());
            }
        }

        self.playerlist.push(player);
        Ok(())
    }

    /// Removes a player from the playerlist.
    // clippy complains that the player argument isn't consumed in the method and it should be a reference instead, but
    // that assumes whatever is passed in is an owned type (i.e. not already a reference). trying to pass an &str for
    // example wouldn't compile
    #[allow(clippy::needless_pass_by_value)]
    pub fn remove<P>(&mut self, player: P) -> anyhow::Result<()>
    where
        P: PartialEq<T> + std::string::ToString,
    {
        if self.playerlist.drain_filter(|p| &player == p).next().is_none() {
            Err(ServerError::NoSuchPlayer(player.to_string()).into())
        } else {
            Ok(())
        }
    }
}
