//! Provides utilities to work with a Factorio server's executable.

mod version_information;

use crate::error::ExecutableError;
use log::*;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use version_information::VersionInformation;

/// The server executable's default path relative to the server installation's root directory.
pub const DEFAULT_PATH: &str = "bin/x64/factorio";

/// Represents a Factorio server's executable.
#[derive(Debug)]
pub struct Executable {
    /// The path to the executable.
    path: PathBuf,
}

impl Executable {
    /// Returns a new `Executable` from a given path to a server executable.
    pub async fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref().to_path_buf();
        let exec = Self { path };
        match exec.detect_version().await {
            Ok(ver) => debug!(
                "{} is a valid Factorio executable. Version information: {:?}",
                exec.path().display(),
                ver
            ),
            Err(e) => {
                return Err(ExecutableError::InvalidExecutable {
                    path: exec.into_path(),
                    source: e,
                }
                .into())
            }
        };

        Ok(exec)
    }

    /// Returns the server's version information by running the executable with the `--version` parameter.
    pub async fn detect_version(&self) -> anyhow::Result<VersionInformation> {
        let stdout = run_executable(&self.path, &["--version"]).await?;
        Ok(stdout.parse()?)
    }

    /// Immutably borrows the `Executable`'s path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Consumes the `Executable` and returns its path.
    fn into_path(self) -> PathBuf {
        self.path
    }
}

/// Runs a given executable asynchronously and returns its standard output.
async fn run_executable<P>(path: P, args: &[&str]) -> anyhow::Result<String>
where
    P: AsRef<Path>,
{
    let output = Command::new(path.as_ref()).args(args).output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        return Err(ExecutableError::Unsuccesfull {
            exit_code: output.status.code(),
            stdout,
            stderr,
        }
        .into());
    }

    Ok(stdout)
}
