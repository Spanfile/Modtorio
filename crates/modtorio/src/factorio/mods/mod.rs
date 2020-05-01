mod fact_mod;
mod update;

use crate::{ext::PathExt, mod_portal::ModPortal};
use fact_mod::Mod;
use futures::stream::StreamExt;
use glob::glob;
use log::*;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    path::Path,
    sync::Mutex,
};
use tokio::{fs, stream};
use update::ModUpdate;
use util::HumanVersion;

const MOD_LOAD_BUFFER_SIZE: usize = 8;

pub enum ModSource<'a> {
    Portal {
        mod_portal: &'a ModPortal,
        name: &'a str,
        version: Option<HumanVersion>,
    },
    Zip {
        path: &'a (dyn AsRef<Path>),
    },
}

#[derive(Debug)]
pub struct Mods<P>
where
    P: AsRef<Path>,
{
    directory: P,
    mods: HashMap<String, Mod>,
}

impl<P> Mods<P>
where
    P: AsRef<Path>,
{
    pub async fn from_directory(path: P) -> anyhow::Result<Self> {
        let zips = path.as_ref().join("*.zip");
        let mods = Mutex::new(HashMap::new());

        let load_results = stream::iter(glob(zips.get_str()?)?.map(|entry| async {
            let entry = entry?;
            let m = Mod::from_zip(&entry).await?;
            debug!("Loaded mod '{}' from zip {}", m.info.name, entry.display());

            let name = m.info.name.clone();
            match mods.lock().unwrap().entry(name) {
                Entry::Occupied(mut entry) => {
                    let existing: &Mod = entry.get();

                    warn!(
                        "Found duplicate '{}' (new {} vs existing {}), preserving newer and removing older",
                        entry.key(),
                        m.info.version,
                        existing.info.version
                    );

                    if m.info.version > existing.info.version {
                        entry.insert(m);
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(m);
                }
            }
            Ok(())
        }))
        .buffer_unordered(MOD_LOAD_BUFFER_SIZE)
        .collect::<Vec<anyhow::Result<()>>>()
        .await;

        for result in &load_results {
            if let Err(e) = result {
                warn!("Mod failed to load: {}", e);
            }
        }

        Ok(Mods {
            directory: path,
            mods: mods.into_inner()?,
        })
    }
}

impl<P> Mods<P>
where
    P: AsRef<Path>,
{
    pub fn count(&self) -> usize {
        self.mods.len()
    }

    pub async fn add<'a>(&mut self, source: ModSource<'_>) -> anyhow::Result<()> {
        let zipfile = match source {
            ModSource::Portal {
                mod_portal,
                name,
                version,
            } => {
                debug!("Retrieve mod '{}' (ver. {:?}) from portal", name, version);

                let (path, written_bytes) = mod_portal
                    .download_mod(&name, version, &self.directory)
                    .await?;

                debug!("Downloaded {} bytes to {}", written_bytes, path.display());
                path
            }
            ModSource::Zip { path: _ } => unimplemented!(),
        };

        let new_mod = Mod::from_zip(zipfile).await?;
        debug!("{:?}", new_mod);

        match self.mods.entry(new_mod.info.name.clone()) {
            Entry::Occupied(mut entry) => {
                let new_version = new_mod.info.version;
                let old_mod = entry.insert(new_mod);
                self.remove_mod_zip(&old_mod).await?;

                info!(
                    "Replaced '{}' ver. {} with {}",
                    old_mod.info.name, old_mod.info.version, new_version
                );
            }
            Entry::Vacant(entry) => {
                info!(
                    "Added '{}' ver. {}",
                    new_mod.info.title, new_mod.info.version
                );
                entry.insert(new_mod);
            }
        }

        Ok(())
    }

    pub async fn check_updates(&self, portal: &ModPortal) -> anyhow::Result<Vec<ModUpdate>> {
        info!("Checking for mod updates");

        let updates = stream::iter(self.mods.values().map(|m| async move {
            match portal.latest_release(&m.info.name).await {
                Ok(latest) => {
                    debug!("Latest version for '{}': {}", m.info.name, latest.version);

                    if m.info.version < latest.version {
                        info!(
                            "Found newer version of '{}': {} (over {}) released on {}",
                            m.info.title, latest.version, m.info.version, latest.released_on
                        );

                        Some(ModUpdate {
                            name: m.info.name.clone(),
                            title: m.info.title.clone(),
                            current_version: m.info.version,
                            new_version: latest.version,
                            released_on: latest.released_on,
                        })
                    } else {
                        None
                    }
                }
                Err(e) => {
                    warn!("Failed to get latest release of '{}': {}", m.info.name, e);
                    None
                }
            }
        }))
        .buffer_unordered(MOD_LOAD_BUFFER_SIZE)
        .collect::<Vec<Option<ModUpdate>>>()
        .await
        .into_iter()
        .filter_map(|u| u)
        .collect::<Vec<ModUpdate>>();

        debug!("Mod updates: {:?}", updates);

        Ok(updates)
    }

    pub async fn apply_updates(
        &mut self,
        updates: &[ModUpdate],
        portal: &ModPortal,
    ) -> anyhow::Result<()> {
        for update in updates {
            info!(
                "Applying update for '{}' ver. {} (over {}) released on {}",
                update.title, update.new_version, update.current_version, update.released_on
            );

            self.add(ModSource::Portal {
                mod_portal: portal,
                name: &update.name,
                version: Some(update.new_version),
            })
            .await?;
        }

        Ok(())
    }
}

impl<P> Mods<P>
where
    P: AsRef<Path>,
{
    async fn remove_mod_zip(&self, fact_mod: &Mod) -> anyhow::Result<()> {
        let path = self
            .directory
            .as_ref()
            .join(fact_mod.get_archive_filename());
        debug!("Removing mod zip {}", path.display());
        Ok(fs::remove_file(path).await?)
    }
}
