use crate::{
    ext::PathExt,
    mod_common::{DownloadResult, Mod, Requirement},
    util::HumanVersion,
    Config, ModPortal,
};
use anyhow::anyhow;
use futures::future;
use glob::glob;
use log::*;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    path::Path,
    sync::{Arc, Mutex, Weak},
};
use tokio::fs;

type WeakModContainer = Weak<Mutex<Mod>>;
type ModContainer = Arc<Mutex<Mod>>;
type ModsContainer = Arc<Mutex<HashMap<String, ModContainer>>>;

pub struct ModsBuilder<P>
where
    P: AsRef<Path>,
{
    directory: P,
}

#[derive(Debug)]
pub struct Mods<P>
where
    P: AsRef<Path>,
{
    directory: P,
    mods: ModsContainer,
    config: Arc<Config>,
    portal: Arc<ModPortal>,
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
        config: Arc<Config>,
        portal: Arc<ModPortal>,
    ) -> anyhow::Result<Mods<P>> {
        let zips = self.directory.as_ref().join("*.zip");
        let mods = Arc::new(Mutex::new(HashMap::new()));

        let mut handles = Vec::new();
        for entry in glob(zips.get_str()?)? {
            let (mods, portal) = (Arc::clone(&mods), Arc::clone(&portal));
            handles.push(async move || -> anyhow::Result<()> {
                let entry = entry?;
                let m = Mod::from_zip(entry, Arc::clone(&portal)).await?;

                debug!("Loaded mod {}", m.display()?);

                let name = m.name().to_owned();
                let mut mods = mods.lock().unwrap();
                match mods.entry(name) {
                    Entry::Occupied(entry) => {
                        let existing: &Arc<Mutex<Mod>> = entry.get();

                        warn!(
                            "Found duplicate '{}' (new {} vs existing {})",
                            entry.key(),
                            m.own_version()?,
                            existing.lock().unwrap().own_version()?
                        );

                        // TODO: update mod in place?
                        panic!();
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(Arc::new(Mutex::new(m)));
                    }
                }

                Ok(())
            }());
        }

        future::try_join_all(handles).await?;

        Ok(Mods {
            directory: self.directory,
            mods,
            config,
            portal: Arc::clone(&portal),
        })
    }
}

impl<'a, P> Mods<P>
where
    P: AsRef<Path>,
{
    pub fn count(&self) -> usize {
        self.mods.lock().unwrap().len()
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
        info!(
            "Added {}",
            new_mod
                .upgrade()
                .ok_or_else(|| anyhow!("Weak mod pointer dropped"))?
                .lock()
                .unwrap()
                .display()?
        );
        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        info!("Checking for mod updates...");

        let mut handles = Vec::new();
        let updates = Arc::new(Mutex::new(Vec::new()));
        let mods = self.mods.lock().unwrap();

        for m in mods.values() {
            let m = Arc::clone(&m);
            let updates = Arc::clone(&updates);
            let portal = Arc::clone(&self.portal);

            handles.push(async move || -> anyhow::Result<()> {
                let mut m = m.lock().unwrap();

                m.fetch_portal_info(Arc::clone(&portal)).await?;
                let release = m.latest_release()?;

                if m.own_version()? < release.version() {
                    debug!(
                        "Found newer version of '{}': {} (over {}) released on {}",
                        m.title(),
                        release.version(),
                        m.own_version()?,
                        release.released_on()
                    );

                    updates.lock().unwrap().push(m.name().to_owned());
                } else {
                    debug!("{} is up to date", m.display()?);
                }

                Ok(())
            }());
        }

        future::try_join_all(handles).await?;

        let updates = updates.lock().unwrap();
        info!("Found {} updates", updates.len());
        if !updates.is_empty() {
            debug!("{:?}", updates);

            for update in updates.iter() {
                self.add_or_update_in_place(update, None).await?;
            }
        };

        Ok(())
    }

    pub async fn ensure_dependencies(&mut self) -> anyhow::Result<()> {
        info!("Ensuring dependencies are met...");

        let mut missing: Vec<String> = Vec::new();

        for m in self.mods.lock().unwrap().values() {
            missing.extend(
                self.ensure_single_dependencies(Arc::clone(m))
                    .await?
                    .into_iter(),
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

impl<'a, P> Mods<P>
where
    P: AsRef<Path>,
{
    fn get_mod(&self, name: &str) -> anyhow::Result<WeakModContainer> {
        Ok(self
            .mods
            .lock()
            .unwrap()
            .get(name)
            .map(|m| Arc::downgrade(m))
            .ok_or_else(|| anyhow!("No such mod: {}", name))?)
    }

    async fn add_or_update_in_place(
        &self,
        name: &str,
        version: Option<HumanVersion>,
    ) -> anyhow::Result<WeakModContainer> {
        let mut mods = self.mods.lock().unwrap();
        match mods.entry(name.to_owned()) {
            Entry::Occupied(entry) => {
                let existing_mod = entry.get();
                let mut unlocked = existing_mod.lock().unwrap();

                match unlocked
                    .download(version, &self.directory, Arc::clone(&self.portal))
                    .await?
                {
                    DownloadResult::New => info!("{} added", unlocked.display()?),
                    DownloadResult::Unchanged => info!("{} unchanged", unlocked.display()?,),
                    DownloadResult::Replaced {
                        old_version,
                        old_archive,
                    } => {
                        debug!("Removing old mod archive {}", old_archive);
                        let path = self.directory.as_ref().join(old_archive);
                        fs::remove_file(path).await?;

                        info!("{} replaced from ver. {}", unlocked.display()?, old_version);
                    }
                }

                Ok(Arc::downgrade(existing_mod))
            }
            Entry::Vacant(entry) => {
                let new_mod = Arc::new(Mutex::new(
                    Mod::from_portal(name, Arc::clone(&self.portal)).await?,
                ));

                Ok(Arc::downgrade(entry.insert(new_mod)))
            }
        }
    }

    async fn ensure_single_dependencies(
        &self,
        target_mod: ModContainer,
    ) -> anyhow::Result<Vec<String>> {
        let mut missing = Vec::new();
        let target_mod = target_mod.lock().unwrap();

        for dep in target_mod.dependencies()? {
            if dep.name() == "base" {
                continue;
            }

            match dep.requirement() {
                Requirement::Mandatory => {
                    match self.get_mod(dep.name()) {
                        Ok(required_mod) => {
                            let required_mod = required_mod
                                .upgrade()
                                .ok_or_else(|| anyhow!("Weak mod pointer dropped"))?;
                            let required_mod = required_mod.lock().unwrap();

                            match dep.version() {
                                Some(version_req)
                                    if !required_mod.own_version()?.meets(version_req) =>
                                {
                                    debug!("Dependency {} of '{}' not met: version requirement mismatch (found {})", dep, target_mod.name(), required_mod.own_version()?);
                                    missing.push(dep.name().to_owned());
                                }
                                _ => debug!(
                                    "Dependency {} of '{}' met (found {})",
                                    dep,
                                    target_mod.name(),
                                    required_mod.own_version()?
                                ),
                            }
                        }
                        Err(_) => {
                            debug!(
                                "Dependency {} of '{}' not met: required mod not found",
                                dep,
                                target_mod.name()
                            );

                            // TODO: resolve version
                            missing.push(dep.name().to_owned());
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
