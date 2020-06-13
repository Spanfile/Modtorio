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
    path::PathBuf,
    sync::Arc,
};
use tokio::{fs, sync::Mutex, task};

pub struct ModsBuilder {
    directory: PathBuf,
}

pub struct Mods {
    directory: PathBuf,
    mods: HashMap<String, Arc<Mod>>,
    config: Arc<Config>,
    portal: Arc<ModPortal>,
    cache: Arc<Cache>,
}

impl<'a> ModsBuilder {
    pub fn root(directory: PathBuf) -> Self {
        ModsBuilder { directory }
    }

    pub async fn build(
        self,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Mods> {
        let zips = self.directory.join("*.zip");
        let mods = Arc::new(Mutex::new(HashMap::new()));

        for entry in glob(zips.get_str()?)? {
            let entry = entry?;
            let mods = Arc::clone(&mods);
            let (config, portal, cache) =
                (Arc::clone(&config), Arc::clone(&portal), Arc::clone(&cache));
            task::spawn(async move || -> anyhow::Result<()> {
                info!("Creating mod from zip {}", entry.display());

                let m = match Mod::from_zip(
                    entry,
                    Arc::clone(&config),
                    Arc::clone(&portal),
                    Arc::clone(&cache),
                )
                .await
                {
                    Ok(m) => Arc::new(m),
                    Err(e) => {
                        warn!("Mod failed to load: {}", e);
                        return Ok(());
                    }
                };

                let mod_name = m.name().await.to_string();
                debug!("Loaded mod {} from zip", mod_name);

                match m.fetch_cache_info().await {
                    Ok(_) => trace!("Mod '{}' populated from cache", mod_name),
                    Err(e) => debug!("Mod cache loading failed with: {}", e),
                }

                let name = m.name().await.to_owned();
                let mut mods = mods.lock().await;
                match mods.entry(name) {
                    Entry::Occupied(mut entry) => {
                        let existing: &Arc<Mod> = entry.get();

                        warn!(
                            "Found duplicate '{}' (new {} vs existing {})",
                            entry.key(),
                            m.own_version().await?,
                            existing.own_version().await?
                        );

                        let own_version = m.own_version().await?;
                        let existing_version = existing.own_version().await?;
                        if own_version > existing_version {
                            entry.insert(m);
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(m);
                    }
                }

                Ok(())
            }())
            .await??;
        }

        let mods = mods
            .lock()
            .await
            .iter()
            .map(|(k, v)| (k.to_owned(), Arc::clone(v)))
            .collect();

        Ok(Mods {
            directory: self.directory,
            mods,
            config,
            portal,
            cache,
        })
    }
}

impl Mods {
    pub fn count(&self) -> usize {
        self.mods.len()
    }

    pub async fn update_cache(&self, game_id: i64) -> anyhow::Result<()> {
        for game_mod in self.mods.values() {
            debug!("Updating cache for '{}'", game_mod.name().await);
            game_mod.update_cache().await?;

            info!("Updated cache for {}", game_mod.display().await);
        }

        let new_game_mods = Mutex::new(Vec::new());

        for m in self.mods.values() {
            let mut mods = new_game_mods.lock().await;
            mods.push(models::NewGameMod {
                game: game_id,
                factorio_mod: m.name().await.to_string(),
            });
        }

        self.cache
            .set_mods_of_game(new_game_mods.into_inner())
            .await?;

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
        info!("Added {}", new_mod.display().await);
        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        info!("Checking for mod updates...");

        let mut updates = Vec::new();
        for m in self.mods.values_mut() {
            info!("Checking for updates to {}...", m.display().await);

            m.ensure_portal_info().await?;
            let release = m.latest_release().await?;

            if m.own_version().await? < release.version() {
                debug!(
                    "Found newer version of '{}': {} (over {}) released on {}",
                    m.title().await,
                    release.version(),
                    m.own_version().await?,
                    release.released_on()
                );

                updates.push(m.name().await.to_owned());
            } else {
                debug!("{} is up to date", m.display().await);
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
                    .map(|m| m),
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

impl Mods {
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
    ) -> anyhow::Result<&Mod> {
        match self.mods.entry(name.to_owned()) {
            Entry::Occupied(entry) => {
                let existing_mod = entry.into_mut();

                match existing_mod.download(version, &self.directory).await? {
                    DownloadResult::New => info!("{} added", existing_mod.display().await),
                    DownloadResult::Unchanged => {
                        info!("{} unchanged", existing_mod.display().await)
                    }
                    DownloadResult::Replaced {
                        old_version,
                        old_archive,
                    } => {
                        debug!("Removing old mod archive {}", old_archive);
                        let path = self.directory.join(old_archive);
                        fs::remove_file(path).await?;

                        info!(
                            "{} replaced from ver. {}",
                            existing_mod.display().await,
                            old_version
                        );
                    }
                }

                Ok(existing_mod)
            }
            Entry::Vacant(entry) => {
                let new_mod = Arc::new(
                    Mod::from_portal(
                        name,
                        Arc::clone(&self.config),
                        Arc::clone(&self.portal),
                        Arc::clone(&self.cache),
                    )
                    .await?,
                );
                Ok(entry.insert(new_mod))
            }
        }
    }

    async fn ensure_single_dependencies(&self, target_mod: &Mod) -> anyhow::Result<Vec<String>> {
        let mut missing = Vec::new();
        let target_name = target_mod.name().await;

        for dep in target_mod.dependencies().await? {
            if dep.name() == "base" {
                continue;
            }

            match dep.requirement() {
                Requirement::Mandatory => {
                    match self.get_mod(dep.name()) {
                        Ok(required_mod) => {
                            let required_version = required_mod.own_version().await?;

                            match dep.version() {
                                Some(version_req) if !required_version.meets(version_req) => {
                                    debug!(
                                        "Dependency {} of '{}' not met: version requirement \
                                         mismatch (found {})",
                                        dep, target_name, required_version
                                    );
                                    missing.push(dep.name().to_string());
                                }
                                _ => debug!(
                                    "Dependency {} of '{}' met (found {})",
                                    dep, target_name, required_version
                                ),
                            }
                        }
                        Err(_) => {
                            debug!(
                                "Dependency {} of '{}' not met: required mod not found",
                                dep, target_name
                            );

                            // TODO: resolve version
                            missing.push(dep.name().to_string());
                        }
                    }
                }
                Requirement::Incompatible => match self.get_mod(dep.name()) {
                    Ok(_) => {
                        return Err(anyhow::anyhow!(
                            "Cannot ensure dependency {} of '{}'",
                            dep,
                            target_name
                        ));
                    }
                    Err(_) => debug!("Dependency {} of '{}' met", dep, target_name),
                },
                _ => (),
            }
        }

        Ok(missing)
    }
}
