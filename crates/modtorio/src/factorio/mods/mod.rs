mod fact_mod;

use crate::{ext::PathExt, mod_portal::PortalMod, Config, ModPortal};
use anyhow::anyhow;
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
use util::HumanVersion;

const MOD_LOAD_BUFFER_SIZE: usize = 8;

#[derive(Debug)]
pub struct Mods<'a, P>
where
    P: AsRef<Path>,
{
    directory: P,
    mods: HashMap<String, Mod>,
    config: &'a Config,
    portal: &'a ModPortal,
}

pub struct ModsBuilder<P>
where
    P: AsRef<Path>,
{
    directory: P,
}

#[derive(Debug)]
pub struct ModVersionPair {
    portal_mod: PortalMod,
    version: HumanVersion,
}

impl<'a, P> ModsBuilder<P>
where
    P: AsRef<Path>,
{
    pub fn root(directory: P) -> Self {
        ModsBuilder { directory }
    }

    pub async fn build(
        self,
        config: &'a Config,
        portal: &'a ModPortal,
    ) -> anyhow::Result<Mods<'a, P>> {
        let zips = self.directory.as_ref().join("*.zip");
        let mods = Mutex::new(HashMap::new());

        let load_results = stream::iter(glob(zips.get_str()?)?.map(|entry| async {
            let entry = entry?;
            let m = Mod::from_zip(entry).await?;
            debug!("Loaded mod '{}' (\"{}\") ver. {}", m.info.name, m.info.title, m.info.version);

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
            directory: self.directory,
            mods: mods.into_inner()?,
            config,
            portal,
        })
    }
}

impl<P> Mods<'_, P>
where
    P: AsRef<Path>,
{
    pub fn count(&self) -> usize {
        self.mods.len()
    }

    pub async fn add<'a>(
        &self,
        name: &str,
        version: Option<HumanVersion>,
    ) -> anyhow::Result<ModVersionPair> {
        debug!("Fetch mod '{}' (ver. {:?}) from portal", name, version);

        let portal_mod = self.portal.fetch_mod(name).await?;
        let version = portal_mod.get_release(None)?.version;
        Ok(ModVersionPair {
            portal_mod,
            version,
        })
    }

    pub async fn install<'a>(&mut self, install: &ModVersionPair) -> anyhow::Result<()> {
        let (path, written_bytes) = self
            .portal
            .download_mod(&install.portal_mod, &self.directory)
            .await?;

        debug!("Downloaded {} bytes to {}", written_bytes, path.display());

        let new_mod = Mod::from_zip(path).await?;
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

    pub async fn check_updates(&self) -> anyhow::Result<Vec<ModVersionPair>> {
        info!("Checking for mod updates");

        let mut updates = Vec::new();
        for m in self.mods.values() {
            let portal_mod = self.portal.fetch_mod(&m.info.name).await?;
            let latest_release = portal_mod.get_release(None)?;
            debug!(
                "Latest version for '{}': {}",
                m.info.name, latest_release.version
            );

            if m.info.version < latest_release.version {
                info!(
                    "Found newer version of '{}': {} (over {}) released on {}",
                    m.info.title,
                    latest_release.version,
                    m.info.version,
                    latest_release.released_on
                );

                let version = latest_release.version;
                updates.push(ModVersionPair {
                    portal_mod,
                    version,
                });
            }
        }

        Ok(updates)
    }

    pub async fn apply_updates(&mut self, updates: &[ModVersionPair]) -> anyhow::Result<()> {
        for update in updates {
            let release = update.portal_mod.get_release(Some(update.version))?;
            let current = self.get_mod(&update.portal_mod.name)?.info.version;
            info!(
                "Applying update for '{}' ver. {} (over {}) released on {}",
                update.portal_mod.title, update.version, current, release.released_on
            );

            self.install(update).await?;
        }

        Ok(())
    }
}

impl<P> Mods<'_, P>
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

    fn get_mod(&self, name: &str) -> anyhow::Result<&Mod> {
        Ok(self
            .mods
            .get(name)
            .ok_or_else(|| anyhow!("No such mod: {}", name))?)
    }
}
