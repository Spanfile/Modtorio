//! Provides types and functions used to report status updates into a `tokio::mpsc` channel from an async task.

use log::*;
use rpc::{progress::ProgressType, Progress};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

type AsyncProgressResult = Result<Progress, tonic::Status>;
pub type AsyncProgressChannel = Arc<Mutex<mpsc::Sender<AsyncProgressResult>>>;

/// Sends a given status update to an optional progress channel.
pub async fn send_status(channel: Option<AsyncProgressChannel>, status: AsyncProgressResult) -> anyhow::Result<()> {
    if let Some(channel) = channel {
        trace!("Sending status update: {:?}", status);
        if let Err(e) = channel.lock().await.try_send(status) {
            error!("Caught error while sendig RPC status update: {}", e);
            return Err(anyhow::anyhow!(""));
        }
    }
    Ok(())
}

/// Returns a new indefinite progress status.
pub fn indefinite(message: &str) -> AsyncProgressResult {
    Ok(Progress {
        message: String::from(message),
        prog_type: ProgressType::Indefinite.into(),
        value: 0,
        max: 0,
    })
}

/// Returns a new definite progress status.
pub fn definite(message: &str, value: u32, max: u32) -> AsyncProgressResult {
    Ok(Progress {
        message: String::from(message),
        prog_type: ProgressType::Definite.into(),
        value,
        max,
    })
}

/// Returns a new internal error status.
pub fn internal_error(message: &str) -> AsyncProgressResult {
    Err(tonic::Status::internal(message))
}

/// Returns a new failed precondition error status.
pub fn failed_precondition(message: &str) -> AsyncProgressResult {
    Err(tonic::Status::failed_precondition(message))
}
