mod dependency;
mod info;

use crate::ModPortal;
use bytesize::ByteSize;
use info::Info;
use log::*;
use std::{cell::RefCell, path::Path};
use util::HumanVersion;

pub use dependency::{Dependency, Requirement};
pub use info::Release;

#[derive(Debug)]
pub struct Mod<'a> {
    info: RefCell<Info>,
    portal: &'a ModPortal,
}

impl<'a> Mod<'a> {
    pub async fn from_zip<P>(path: P, portal: &'a ModPortal) -> anyhow::Result<Mod<'a>>
    where
        P: 'static + AsRef<Path> + Send,
    {
        debug!("Creating mod from zip {}", path.as_ref().display());
        let info = Info::from_zip(path).await?;

        Ok(Mod {
            info: RefCell::new(info),
            portal,
        })
    }

    pub async fn from_portal(name: &str, portal: &'a ModPortal) -> anyhow::Result<Mod<'a>> {
        let info = Info::from_portal(name, portal).await?;

        Ok(Self {
            info: RefCell::new(info),
            portal,
        })
    }
}

impl<'a> Mod<'a> {
    pub async fn download<P>(
        &self,
        version: Option<HumanVersion>,
        destination: P,
        portal: &'a ModPortal,
    ) -> anyhow::Result<usize>
    where
        P: AsRef<Path>,
    {
        let mut info = self.info.borrow_mut();
        let release = info.get_release(version)?;
        let (path, download_size) = portal.download_mod(release.url()?, destination).await?;

        debug!(
            "{} ({} bytes) downloaded, populating info from {}",
            ByteSize::b(download_size as u64),
            download_size,
            path.display()
        );
        info.populate_from_zip(path).await?;

        Ok(download_size)
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
        let info = self.info.borrow();
        Ok(format!("{}_{}.zip", info.name(), info.own_version()?))
    }

    pub fn name(&self) -> String {
        let info = self.info.borrow();
        info.name().to_owned()
    }

    pub fn title(&self) -> String {
        let info = self.info.borrow();
        info.title().to_owned()
    }

    pub fn own_version(&self) -> anyhow::Result<HumanVersion> {
        let info = self.info.borrow();
        info.own_version()
    }

    pub fn factorio_version(&self) -> anyhow::Result<HumanVersion> {
        let info = self.info.borrow();
        info.factorio_version()
    }
}
