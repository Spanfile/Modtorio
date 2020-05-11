use crate::{ext::PathExt, mod_common::Mod, Config, ModPortal};
use anyhow::anyhow;
use glob::glob;
use log::*;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    path::Path,
};
use tokio::fs;
use util::HumanVersion;

pub struct ModsBuilder<P>
where
    P: AsRef<Path>,
{
    directory: P,
}

#[derive(Debug)]
pub struct Mods<'a, P>
where
    P: AsRef<Path>,
{
    directory: P,
    mods: HashMap<String, Mod<'a>>,
    config: &'a Config,
    portal: &'a ModPortal,
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
        let mut mods = HashMap::new();

        for entry in glob(zips.get_str()?)? {
            let entry = entry?;
            let m = match Mod::from_zip(entry, portal).await {
                Ok(m) => m,
                Err(e) => {
                    warn!("Mod failed to load: {}", e);
                    continue;
                }
            };

            debug!("Loaded mod {}", m.display()?);

            let name = m.name().to_owned();
            match mods.entry(name) {
                Entry::Occupied(mut entry) => {
                    let existing: &Mod = entry.get();

                    warn!(
                        "Found duplicate '{}' (new {} vs existing {})",
                        entry.key(),
                        m.own_version()?,
                        existing.own_version()?
                    );

                    if m.own_version()? > existing.own_version()? {
                        entry.insert(m);
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(m);
                }
            }
        }

        Ok(Mods {
            directory: self.directory,
            mods,
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

    pub async fn add_from_portal(
        &mut self,
        name: &str,
        version: Option<HumanVersion>,
    ) -> anyhow::Result<()> {
        if let Some(version) = version {
            info!("Add '{}' ver. {:?}", name, version);
        } else {
            info!("Add latest '{}'", name);
        }

        let new_mod = Mod::from_portal(name, self.portal).await?;
        new_mod
            .download(version, &self.directory, self.portal)
            .await?;

        match self.mods.entry(new_mod.name().to_owned()) {
            Entry::Occupied(mut entry) => {
                let new_version = new_mod.own_version()?;
                let old_mod = entry.insert(new_mod);
                self.remove_mod_zip(&old_mod).await?;

                info!("Replaced {} with {}", old_mod.display()?, new_version);
            }
            Entry::Vacant(entry) => {
                info!("Added {}", new_mod.display()?);
                entry.insert(new_mod);
            }
        }

        Ok(())
    }

    // pub async fn install<'a>(
    //     &self,
    //     name: &str,
    //     version: Option<HumanVersion>,
    // ) -> anyhow::Result<ModVersionPair> {
    //     if let Some(version) = version {
    //         info!("Fetch '{}' ver. {:?}", name, version);
    //     } else {
    //         info!("Fetch latest '{}'", name);
    //     }

    //     self.fetch_mod(name, version).await
    // }

    // pub async fn apply_install<'a>(&mut self, install: &ModVersionPair) -> anyhow::Result<()> {
    //     let (path, written_bytes) = self
    //         .portal
    //         .download_mod(&install.portal_mod, &self.directory)
    //         .await?;

    //     debug!(
    //         "Downloaded {} ({} bytes) to {}",
    //         ByteSize::b(written_bytes as u64),
    //         written_bytes,
    //         path.display()
    //     );

    //     let new_mod = Mod::from_zip(path, self.portal).await?;
    //     debug!("{:?}", new_mod);

    //     match self.mods.entry(new_mod.name().to_owned()) {
    //         Entry::Occupied(mut entry) => {
    //             let new_version = new_mod.own_version();
    //             let old_mod = entry.insert(new_mod);
    //             self.remove_mod_zip(&old_mod).await?;

    //             info!("Replaced {} with {}", old_mod, new_version);
    //         }
    //         Entry::Vacant(entry) => {
    //             info!("Added {}", new_mod);
    //             entry.insert(new_mod);
    //         }
    //     }

    //     Ok(())
    // }

    // pub async fn check_updates(&self) -> anyhow::Result<Vec<ModVersionPair>> {
    //     info!("Checking for mod updates...");

    //     let mut updates = Vec::new();
    //     for m in self.mods.values() {
    //         let install = self.fetch_mod(&m.name(), None).await?;
    //         let release = install.get_release()?;
    //         debug!("Latest version for '{}': {}", m.name(), install.version);

    //         if m.own_version() < install.version {
    //             debug!(
    //                 "Found newer version of '{}': {} (over {}) released on {}",
    //                 m.title(),
    //                 install.version,
    //                 m.own_version(),
    //                 release.released_on
    //             );

    //             updates.push(install);
    //         }
    //     }

    //     info!("Found {} updates", updates.len());
    //     Ok(updates)
    // }

    // pub async fn apply_updates(&mut self, updates: &[ModVersionPair]) -> anyhow::Result<()> {
    //     if updates.is_empty() {
    //         info!("No updates to apply");
    //     } else {
    //         for update in updates {
    //             let release = update.get_release()?;
    //             let current = self.get_mod(&update.portal_mod.name)?.own_version();
    //             info!(
    //                 "Applying update for '{}' ('{}') ver. {} (over {}) released on {}",
    //                 update.portal_mod.title,
    //                 update.portal_mod.name,
    //                 update.version,
    //                 current,
    //                 release.released_on
    //             );

    //             self.apply_install(update).await?;
    //         }
    //     }

    //     Ok(())
    // }

    // pub async fn ensure_dependencies(
    //     &self,
    //     install: &ModVersionPair,
    // ) -> anyhow::Result<Vec<ModVersionPair>> {
    //     info!("Ensuring dependencies for '{}'", install.portal_mod.name);

    //     let mut additional_installs = Vec::new();
    //     let release = install.get_release()?;

    //     for dep in &release.info.dependencies {
    //         if dep.name == "base" {
    //             continue;
    //         }

    //         match dep.requirement {
    //             Requirement::Mandatory => match self.get_mod(&dep.name) {
    //                 Ok(_) => {
    //                     debug!(
    //                         "Dependency '{:?}' of '{}' met",
    //                         dep, install.portal_mod.name
    //                     );
    //                 }
    //                 Err(_) => {
    //                     debug!(
    //                         "Dependency '{:?}' of '{}' not met, installing",
    //                         dep, install.portal_mod.name
    //                     );
    //                     // TODO: resolve version
    //                     additional_installs.push(self.fetch_mod(&dep.name, None).await?);
    //                 }
    //             },
    //             Requirement::Incompatible => match self.get_mod(&dep.name) {
    //                 Ok(_) => {
    //                     return Err(anyhow::anyhow!(
    //                         "Cannot ensure dependency '{:?}' of '{}'",
    //                         dep,
    //                         install.portal_mod.name
    //                     ));
    //                 }
    //                 Err(_) => {
    //                     debug!(
    //                         "Dependency '{:?}' of '{}' met",
    //                         dep, install.portal_mod.name
    //                     );
    //                 }
    //             },
    //             _ => {}
    //         }
    //     }

    //     Ok(additional_installs)
    // }
}

impl<P> Mods<'_, P>
where
    P: AsRef<Path>,
{
    async fn remove_mod_zip(&self, fact_mod: &Mod<'_>) -> anyhow::Result<()> {
        let path = self
            .directory
            .as_ref()
            .join(fact_mod.get_archive_filename()?);
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
