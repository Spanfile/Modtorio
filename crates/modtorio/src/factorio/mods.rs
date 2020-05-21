use crate::{
    cache::models,
    ext::PathExt,
    mod_common::{DownloadResult, Mod, Requirement},
    util::HumanVersion,
    Cache, Config, ModPortal,
};
use anyhow::anyhow;
use glob::glob;
use log::*;
use std::{
    collections::{hash_map::Entry, HashMap},
    path::Path,
};
use tokio::fs;

pub struct ModsBuilder<P>
where
    P: AsRef<Path>,
{
    directory: P,
}

pub struct Mods<'a, P>
where
    P: AsRef<Path>,
{
    directory: P,
    mods: HashMap<String, Mod<'a>>,
    config: &'a Config,
    portal: &'a ModPortal,
    cache: &'a Cache,
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
        cache: &'a Cache,
    ) -> anyhow::Result<Mods<'a, P>> {
        let zips = self.directory.as_ref().join("*.zip");
        let mut mods = HashMap::new();

        for entry in glob(zips.get_str()?)? {
            let entry = entry?;
            info!("Creating mod from zip {}", entry.display());

            let m = match Mod::from_zip(entry, portal, cache).await {
                Ok(m) => m,
                Err(e) => {
                    warn!("Mod failed to load: {}", e);
                    continue;
                }
            };

            debug!("Loaded mod {}", m);

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
            cache,
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

    pub async fn update_cache(&mut self, game_id: i32) -> anyhow::Result<()> {
        for game_mod in self.mods.values_mut() {
            debug!("Updating cache for {}", game_mod);
            game_mod.update_cache().await?;
        }

        let cache_mods = self
            .mods
            .values()
            .map(|m| models::NewGameMod {
                game: game_id,
                factorio_mod: m.name(),
            })
            .collect::<Vec<models::NewGameMod>>();

        self.cache.set_mods_of_game(&cache_mods)?;

        Ok(())
    }

    pub async fn add_from_portal(
        &mut self,
        name: &str,
        version: Option<HumanVersion>,
    ) -> anyhow::Result<()> {
        if let Some(version) = version {
            info!("Adding '{}' ver. {:?}", name, version);
        } else {
            info!("Adding latest '{}'", name);
        }

        let new_mod = self.add_or_update_in_place(name, version).await?;
        info!("Added {}", new_mod);
        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        info!("Checking for mod updates...");

        let mut updates = Vec::new();
        for m in self.mods.values_mut() {
            info!("Checking for updates to {}...", m);

            m.fetch_portal_info().await?;
            let release = m.latest_release()?;

            if m.own_version()? < release.version() {
                debug!(
                    "Found newer version of '{}': {} (over {}) released on {}",
                    m.title(),
                    release.version(),
                    m.own_version()?,
                    release.released_on()
                );

                updates.push(m.name().to_owned());
            } else {
                debug!("{} is up to date", m);
            }
        }

        info!("Found {} updates", updates.len());
        if !updates.is_empty() {
            debug!("{:?}", updates)
        };

        for update in &updates {
            let _ = self.add_or_update_in_place(update, None).await?;
        }

        Ok(())
    }

    pub async fn ensure_dependencies(&mut self) -> anyhow::Result<()> {
        info!("Ensuring dependencies are met...");

        let mut missing: Vec<String> = Vec::new();

        for m in self.mods.values() {
            missing.extend(
                self.ensure_single_dependencies(m)
                    .await?
                    .into_iter()
                    .map(|m| m.to_owned()),
            );
        }

        if !missing.is_empty() {
            info!("Found {} missing dependencies, installing", missing.len());
            for miss in &missing {
                self.add_from_portal(&miss, None).await?;
            }
        }

        Ok(())
    }
}

impl<'a, P> Mods<'a, P>
where
    P: AsRef<Path>,
{
    fn get_mod(&self, name: &str) -> anyhow::Result<&Mod> {
        Ok(self
            .mods
            .get(name)
            .ok_or_else(|| anyhow!("No such mod: {}", name))?)
    }

    async fn add_or_update_in_place(
        &mut self,
        name: &str,
        version: Option<HumanVersion>,
    ) -> anyhow::Result<&Mod<'a>> {
        match self.mods.entry(name.to_owned()) {
            Entry::Occupied(entry) => {
                let existing_mod = entry.into_mut();

                match existing_mod.download(version, &self.directory).await? {
                    DownloadResult::New => info!("{} added", existing_mod),
                    DownloadResult::Unchanged => info!("{} unchanged", existing_mod),
                    DownloadResult::Replaced {
                        old_version,
                        old_archive,
                    } => {
                        debug!("Removing old mod archive {}", old_archive);
                        let path = self.directory.as_ref().join(old_archive);
                        fs::remove_file(path).await?;

                        info!("{} replaced from ver. {}", existing_mod, old_version);
                    }
                }

                Ok(existing_mod)
            }
            Entry::Vacant(entry) => {
                let new_mod = Mod::from_portal(name, self.portal, self.cache).await?;
                Ok(entry.insert(new_mod))
            }
        }
    }

    async fn ensure_single_dependencies(
        &self,
        target_mod: &'a Mod<'_>,
    ) -> anyhow::Result<Vec<&str>> {
        let mut missing = Vec::new();

        for dep in target_mod.dependencies()? {
            if dep.name() == "base" {
                continue;
            }

            match dep.requirement() {
                Requirement::Mandatory => {
                    match self.get_mod(dep.name()) {
                        Ok(required_mod) => match dep.version() {
                            Some(version_req)
                                if !required_mod.own_version()?.meets(version_req) =>
                            {
                                debug!("Dependency {} of '{}' not met: version requirement mismatch (found {})", dep, target_mod.name(), required_mod.own_version()?);
                                missing.push(dep.name());
                            }
                            _ => debug!(
                                "Dependency {} of '{}' met (found {})",
                                dep,
                                target_mod.name(),
                                required_mod.own_version()?
                            ),
                        },
                        Err(_) => {
                            debug!(
                                "Dependency {} of '{}' not met: required mod not found",
                                dep,
                                target_mod.name()
                            );

                            // TODO: resolve version
                            missing.push(dep.name());
                        }
                    }
                }
                Requirement::Incompatible => match self.get_mod(dep.name()) {
                    Ok(_) => {
                        return Err(anyhow::anyhow!(
                            "Cannot ensure dependency {} of '{}'",
                            dep,
                            target_mod.name()
                        ));
                    }
                    Err(_) => debug!("Dependency {} of '{}' met", dep, target_mod.name()),
                },
                _ => (),
            }
        }

        Ok(missing)
    }
}
