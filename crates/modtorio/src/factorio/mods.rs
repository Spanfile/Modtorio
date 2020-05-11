use crate::{ext::PathExt, mod_common::Mod, util::HumanVersion, Config, ModPortal};
use anyhow::anyhow;
use glob::glob;
use log::*;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    path::Path,
};
use tokio::fs;

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

impl<'a, P> Mods<'a, P>
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

        let mut new_mod = Mod::from_portal(name, self.portal).await?;
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

    pub async fn update(&mut self) -> anyhow::Result<()> {
        info!("Checking for mod updates...");

        let mut updates = Vec::new();
        for m in self.mods.values_mut() {
            m.fetch_portal_info(self.portal).await?;
            let release = m.latest_release()?;
            debug!("Latest version for '{}': {}", m.name(), release.version());

            if m.own_version()? < release.version() {
                debug!(
                    "Found newer version of '{}': {} (over {}) released on {}",
                    m.title(),
                    release.version(),
                    m.own_version()?,
                    release.released_on()
                );

                updates.push(m.name().to_owned());
            }
        }

        info!("Found {} updates", updates.len());
        if !updates.is_empty() {
            debug!("{:?}", updates)
        };

        for update in &updates {
            let m = self
                .mods
                .get_mut(update)
                .ok_or_else(|| anyhow!("No such mod: {}", update))?;
            info!(
                "Updating {} to {}",
                m.display()?,
                m.latest_release()?.version()
            );

            m.download(None, &self.directory, self.portal).await?;
        }

        Ok(())
    }

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

    fn get_mod(&self, name: &str) -> anyhow::Result<&Mod> {
        Ok(self
            .mods
            .get(name)
            .ok_or_else(|| anyhow!("No such mod: {}", name))?)
    }

    async fn remove_mod_zip(&self, fact_mod: &Mod<'_>) -> anyhow::Result<()> {
        let path = self
            .directory
            .as_ref()
            .join(fact_mod.get_archive_filename()?);
        debug!("Removing mod zip {}", path.display());
        Ok(fs::remove_file(path).await?)
    }
}
