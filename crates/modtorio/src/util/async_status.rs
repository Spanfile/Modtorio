//! Provides types and functions used to report status updates into a `tokio::mpsc` channel from an async task.

use async_trait::async_trait;
use log::*;
use rpc::{progress::ProgressType, Progress};
use tokio::sync::mpsc;

/// The Result type used to reprsent the status of an async task.
pub type AsyncProgressResult = Result<Progress, tonic::Status>;
/// The channel used to send async task results into.
pub type AsyncProgressChannel = mpsc::Sender<AsyncProgressResult>;

/// Defines functions used with `AsyncProgressChannel`.
#[async_trait]
pub trait AsyncProgressChannelExt {
    /// Sends a given `AsyncProgressResult` status update to this channel.
    async fn send_status(&self, status: AsyncProgressResult) -> anyhow::Result<()>;
}

#[async_trait]
impl AsyncProgressChannelExt for AsyncProgressChannel {
    async fn send_status(&self, status: AsyncProgressResult) -> anyhow::Result<()> {
        send_status(Some(self.clone()), status).await
    }
}

#[async_trait]
impl AsyncProgressChannelExt for Option<AsyncProgressChannel> {
    async fn send_status(&self, status: AsyncProgressResult) -> anyhow::Result<()> {
        send_status(self.clone(), status).await
    }
}

/// Sends a given status update to an optional progress channel.
pub async fn send_status(channel: Option<AsyncProgressChannel>, status: AsyncProgressResult) -> anyhow::Result<()> {
    if let Some(mut channel) = channel {
        trace!("Sending status update: {:?}", status);
        if let Err(e) = channel.try_send(status) {
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

/// Returns a new done status.
pub fn done() -> AsyncProgressResult {
    Ok(Progress {
        message: String::new(),
        prog_type: ProgressType::Done.into(),
        value: 1,
        max: 1,
    })
}
