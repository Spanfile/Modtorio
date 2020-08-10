use log::*;
use rpc::Progress;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

type AsyncProgressResult = Result<Progress, tonic::Status>;
pub type AsyncProgressChannel = Arc<Mutex<mpsc::Sender<AsyncProgressResult>>>;

pub async fn send_status(channel: Option<AsyncProgressChannel>, status: AsyncProgressResult) {
    if let Some(channel) = channel {
        trace!("Sending status update: {:?}", status);
        channel
            .lock()
            .await
            .send(status)
            .await
            .expect("failed to send status message");
    }
}

pub fn indefinite(message: &str) -> AsyncProgressResult {
    Ok(Progress {
        message: String::from(message),
        prog_type: 0,
        value: 0,
        max: 0,
    })
}

pub fn definite(message: &str, value: u32, max: u32) -> AsyncProgressResult {
    Ok(Progress {
        message: String::from(message),
        prog_type: 1,
        value,
        max,
    })
}

pub fn error(message: &str) -> AsyncProgressResult {
    Err(tonic::Status::internal(message))
}
