use crate::{
    ext::PathExt,
    mod_common::{Mod, Requirement},
    util::HumanVersion,
    Config, ModPortal,
};
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
            info!("Adding '{}' ver. {:?}", name, version);
        } else {
            info!("Adding latest '{}'", name);
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
                debug!("{} is up to date", m.display()?);
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

    async fn remove_mod_zip(&self, fact_mod: &Mod<'_>) -> anyhow::Result<()> {
        let path = self
            .directory
            .as_ref()
            .join(fact_mod.get_archive_filename()?);
        debug!("Removing mod zip {}", path.display());
        Ok(fs::remove_file(path).await?)
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
