//! Provides utilities to work with a Factorio server's executable.

mod game_event;
mod version_information;

use crate::error::ExecutableError;
use chrono::{DateTime, Utc};
use futures::future::{AbortHandle, Abortable};
pub use game_event::{EventType, GameEvent};
use log::*;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{mpsc, Mutex},
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
    child_monitor_abort_handle: Mutex<Option<AbortHandle>>,
}

/// Represesnts an event that happened with the executable.
#[derive(Debug)]
pub enum ExecutableEvent {
    /// Represents an event that happened in the server.
    GameEvent(GameEvent),
    /// The executable exited with a given result.
    Exited(anyhow::Result<()>),
}

impl Executable {
    /// Returns a new `Executable` from a given path to a server executable.
    pub async fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref().to_path_buf();
        let exec = Self {
            path,
            child_monitor_abort_handle: Mutex::new(None),
        };
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
        args: &[String],
    ) -> anyhow::Result<mpsc::Receiver<ExecutableEvent>> {
        let mut child = Command::new(&self.path)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| ExecutableError::NoStdioHandle)?;
        let mut stdin = child.stdin.take().ok_or_else(|| ExecutableError::NoStdioHandle)?;
        let mut stdout_reader = BufReader::new(stdout).lines();

        let (mut state_tx, state_rx) = mpsc::channel(64);
        let (mut stdout_proc_tx, mut stdout_proc_rx) = mpsc::channel::<(DateTime<Utc>, String)>(64);

        let mut stdout_proc_state_tx = state_tx.clone();
        task::spawn(async move {
            while let Some((timestamp, stdout_line)) = stdout_proc_rx.recv().await {
                trace!("Processing stdout line: {}", stdout_line);
                let event = match GameEvent::new(timestamp, &stdout_line) {
                    Ok(event) => event,
                    Err(e) => {
                        trace!("Couldn't parse GameEvent: {}", e);
                        continue;
                    }
                };

                if let Err(e) = stdout_proc_state_tx.send(ExecutableEvent::GameEvent(event)).await {
                    error!("Writing executable state to state tx failed: {}", e);
                }
            }

            trace!("Child stdout processor terminating");
        });

        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        *self.child_monitor_abort_handle.lock().await = Some(abort_handle);

        task::spawn(async move {
            match Abortable::new(
                async {
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
                                    if let Err(e) = stdout_proc_tx.send((Utc::now(), stdout_line)).await {
                                        error!("Writing stdout line to stdout processor tx failed: {}", e);
                                    }
                                }
                            }
                        };
                    }
                },
                abort_registration,
            )
            .await
            {
                Ok(_) => trace!("Child monitor task terminating succesfully"),
                Err(e) => {
                    trace!("Child monitor task terminated: aborted");
                    match child.kill() {
                        Ok(_) => trace!("Child executable killed succesfully"),
                        Err(_) => error!("Failed to kill child executable: {}", e),
                    }
                    match child.await {
                        Ok(status) => trace!("Child exited with status '{}' after kill", status),
                        Err(e) => error!("Failed to read child exit status after kill: {}", e),
                    }
                }
            };
        });

        Ok(state_rx)
    }

    /// Aborts the running executable.
    pub async fn abort(&self) {
        if let Some(abort_handle) = self.child_monitor_abort_handle.lock().await.as_ref() {
            debug!("Aborting child executable");
            abort_handle.abort();
        }
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
