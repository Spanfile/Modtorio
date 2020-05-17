mod dependency;
mod info;

use crate::{util::HumanVersion, ModPortal};
use bytesize::ByteSize;
use info::Info;
use log::*;
use std::path::Path;

pub use dependency::{Dependency, Requirement};
pub use info::Release;

#[derive(Debug)]
pub struct Mod<'a> {
    info: Info,
    portal: &'a ModPortal,
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
    pub async fn from_zip<P>(path: P, portal: &'a ModPortal) -> anyhow::Result<Mod<'a>>
    where
        P: 'static + AsRef<Path> + Send,
    {
        debug!("Creating mod from zip {}", path.as_ref().display());
        let info = Info::from_zip(path).await?;

        Ok(Mod { info, portal })
    }

    pub async fn from_portal(name: &str, portal: &'a ModPortal) -> anyhow::Result<Mod<'a>> {
        let info = Info::from_portal(name, portal).await?;

        Ok(Self { info, portal })
    }
}

impl<'a> Mod<'a> {
    pub async fn fetch_portal_info(&mut self, portal: &'a ModPortal) -> anyhow::Result<()> {
        self.info.populate_from_portal(portal).await
    }

    pub async fn download<P>(
        &mut self,
        version: Option<HumanVersion>,
        destination: P,
        portal: &'a ModPortal,
    ) -> anyhow::Result<DownloadResult>
    where
        P: AsRef<Path>,
    {
        let release = self.info.get_release(version)?;
        let (path, download_size) = portal.download_mod(release.url()?, destination).await?;

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

impl Mod<'_> {
    pub fn display(&self) -> anyhow::Result<String> {
        Ok(format!(
            "'{}' ('{}') ver. {}",
            self.title(),
            self.name(),
            self.own_version()?
        ))
    }

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
