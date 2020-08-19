//! Provides utilities to work with a Factorio server's executable.

mod executable_state;
mod game_event;
mod version_information;

use crate::error::ExecutableError;
pub use executable_state::{ExecutableEvent, GameState};
pub use game_event::GameEvent;
use log::*;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::mpsc,
    task,
};
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

    /// Runs this executable.
    pub async fn run(
        &self,
        stdout_tx: mpsc::Sender<String>,
        mut stdin_rx: mpsc::Receiver<String>,
    ) -> anyhow::Result<mpsc::Receiver<ExecutableEvent>> {
        let mut child = Command::new(&self.path)
            .args(&["--start-server", "test.zip"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| ExecutableError::NoStdioHandle)?;
        let mut stdin = child.stdin.take().ok_or_else(|| ExecutableError::NoStdioHandle)?;
        let mut stdout_reader = BufReader::new(stdout).lines();

        let (mut state_tx, state_rx) = mpsc::channel(64);
        let (mut stdout_proc_tx, mut stdout_proc_rx) = mpsc::channel::<String>(64);
        let (mut event_tx, mut event_rx) = mpsc::channel(64);

        task::spawn(async move {
            while let Some(stdout_line) = stdout_proc_rx.recv().await {
                trace!("Processing stdout line: {}", stdout_line);
                let event = match stdout_line.parse::<GameEvent>() {
                    Ok(event) => event,
                    Err(e) => {
                        trace!("Couldn't parse GameEvent: {}", e);
                        continue;
                    }
                };

                if let Err(e) = event_tx.send(event).await {
                    error!("Writing to event tx failed: {}", e);
                }
            }
        });

        task::spawn(async move {
            loop {
                tokio::select! {
                    child_result = wait_for_child(&mut child) => {
                        trace!("Child returned {:?}", child_result);
                        if let Err(e) = state_tx.send(ExecutableEvent::Exited(child_result)).await {
                            error!("Writing executable state to state tx failed: {}", e);
                        }
                        break;
                    }

                    msg = stdin_rx.recv() => {
                        if let Some(msg) = msg {
                            trace!("Got input from stdin channel: {}", msg);
                            if let Err(e) = stdin.write_all(msg.as_bytes()).await {
                                error!("Writing to child stdin failed: {}", e);
                            }
                        }
                    }

                    stdout_line = stdout_reader.next_line() => {
                        if let Some(stdout_line) = stdout_line.expect("failed to read child stdout line") {
                            debug!("Child stdout: {}", stdout_line);
                            if let Err(e) = stdout_proc_tx.send(stdout_line).await {
                                error!("Writing stdout line to stdout processor tx failed: {}", e);
                            }
                        }
                    }

                    event = event_rx.recv() => {
                        if let Some(event) = event {
                            trace!("Game event from executable: {:?}", event);

                            if let Err(e) = state_tx.send(ExecutableEvent::GameEvent(event)).await {
                                error!("Writing executable state to state tx failed: {}", e);
                            }
                        }
                    }
                };
            }

            trace!("Child monitor task returning");
        });

        Ok(state_rx)
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
            stdout: Some(stdout),
            stderr: Some(stderr),
        }
        .into());
    }

    Ok(stdout)
}

/// Asynchronously waits for a given child process to exit. Will not drop the child if the task is cancelled.
async fn wait_for_child(child: &mut Child) -> anyhow::Result<()> {
    let status = child.await?;
    if status.success() {
        Ok(())
    } else {
        Err(ExecutableError::Unsuccesfull {
            exit_code: status.code(),
            stdout: None,
            stderr: None,
        }
        .into())
    }
}
