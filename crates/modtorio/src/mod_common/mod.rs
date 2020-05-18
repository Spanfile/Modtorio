mod dependency;
mod info;

use crate::{util::HumanVersion, Cache, ModPortal};
use bytesize::ByteSize;
use info::Info;
use log::*;
use std::{fmt, path::Path};

pub use dependency::{Dependency, Requirement};
pub use info::Release;

pub struct Mod<'a> {
    info: Info,
    portal: &'a ModPortal,
    cache: &'a Cache,
    cache_id: Option<i32>,
}

#[derive(Debug)]
pub enum DownloadResult {
    New,
    Unchanged,
    Replaced {
        old_version: HumanVersion,
        old_archive: String,
    },
}

impl<'a> Mod<'a> {
    pub async fn from_zip<P>(
        path: P,
        portal: &'a ModPortal,
        cache: &'a Cache,
    ) -> anyhow::Result<Mod<'a>>
    where
        P: 'static + AsRef<Path> + Send,
    {
        let info = Info::from_zip(path).await?;

        Ok(Self {
            info,
            portal,
            cache,
            cache_id: None,
        })
    }

    pub async fn from_portal(
        name: &str,
        portal: &'a ModPortal,
        cache: &'a Cache,
    ) -> anyhow::Result<Mod<'a>> {
        let info = Info::from_portal(name, portal).await?;

        Ok(Self {
            info,
            portal,
            cache,
            cache_id: None,
        })
    }
}

impl<'a> Mod<'a> {
    pub fn update_cache(&mut self) -> anyhow::Result<i32> {
        let id = if let Some(cache_id) = self.cache_id {
            0
        } else {
            0
        };

        Ok(id)
    }

    /// Fetch the latest info from portal
    pub async fn fetch_portal_info(&mut self) -> anyhow::Result<()> {
        self.info.populate_from_portal(self.portal).await
    }

    /// Load the potentially missing portal info by first reading it from cache, and then fetching
    /// from the mod portal if the cache has expired
    pub async fn load_portal_info(&mut self) -> anyhow::Result<()> {
        if let Some(_cache_mod) = self.cache.get_mod(&self.info.name())? {
            // TODO: check expiry
            self.info.populate_from_cache(self.cache)?;
            return Ok(());
        }

        self.fetch_portal_info().await
    }

    pub async fn download<P>(
        &mut self,
        version: Option<HumanVersion>,
        destination: P,
    ) -> anyhow::Result<DownloadResult>
    where
        P: AsRef<Path>,
    {
        let release = self.info.get_release(version)?;
        let (path, download_size) = self
            .portal
            .download_mod(release.url()?, destination)
            .await?;

        debug!(
            "{} ({} bytes) downloaded, populating info from {}",
            ByteSize::b(download_size as u64),
            download_size,
            path.display()
        );

        let old_version = self.own_version().ok();
        let old_archive = self.get_archive_filename()?;
        self.info.populate_from_zip(path).await?;

        if let Some(old_version) = old_version {
            if old_version == self.own_version()? {
                debug!("'{}' unchaged after download", self.name());
                Ok(DownloadResult::Unchanged)
            } else {
                debug!("'{}' changed from ver. {}", self.name(), old_version);
                Ok(DownloadResult::Replaced {
                    old_version,
                    old_archive,
                })
            }
        } else {
            debug!("'{}' newly downloaded", self.name());
            Ok(DownloadResult::New)
        }
    }
}

impl fmt::Display for Mod<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "'{}' ('{}') ver. {}",
            self.title(),
            self.name(),
            self.own_version()
                .map_or_else(|_| String::from("unknown"), |v| v.to_string())
        ))
    }
}

impl Mod<'_> {
    pub fn get_archive_filename(&self) -> anyhow::Result<String> {
        Ok(format!(
            "{}_{}.zip",
            self.info.name(),
            self.info.own_version()?
        ))
    }

    pub fn name(&self) -> &str {
        self.info.name()
    }

    pub fn title(&self) -> &str {
        self.info.title()
    }

    pub fn own_version(&self) -> anyhow::Result<HumanVersion> {
        self.info.own_version()
    }

    pub fn latest_release(&self) -> anyhow::Result<&Release> {
        self.info.get_release(None)
    }

    pub fn dependencies(&self) -> anyhow::Result<&[Dependency]> {
        self.info.dependencies()
    }
}
