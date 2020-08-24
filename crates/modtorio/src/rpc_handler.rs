//! Provides the `RpcHandler` object which is used to ease responding to an RPC request.

use crate::{error::RpcError, util::async_status::AsyncProgressChannel, Modtorio, RpcResult};
use futures::Future;
use log::*;
use rpc::instance_status;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// Used to ease responding to an RPC request.
pub struct RpcHandler<'a, TReq> {
    /// The incoming RPC request.
    request: Request<TReq>,
    /// The Modtorio instance used with building the response.
    instance: &'a Modtorio,
    /// The Modtorio instance's required status, if any.
    required_status: Option<instance_status::Status>,
}

impl<'a, TReq> RpcHandler<'a, TReq>
where
    TReq: std::fmt::Debug,
{
    /// Returns a new `RpcHandler` out of a given Modtorio instance and an RPC request.
    pub fn new(instance: &'a Modtorio, request: Request<TReq>) -> RpcHandler<'a, TReq> {
        Self {
            request,
            instance,
            required_status: None,
        }
    }

    /// Requires the Modtorio instance have a certain status.
    pub fn require_status(self, status: instance_status::Status) -> Self {
        Self {
            required_status: Some(status),
            ..self
        }
    }

    /// Returns a response to the request, built with a simple response object.
    pub async fn respond<T>(self, response: T) -> Result<Response<T>, Status>
    where
        T: std::fmt::Debug,
    {
        self.log_request();
        self.assert_status().await?;

        let resp = Response::new(response);
        debug!("{:?}", resp);
        Ok(resp)
    }

    /// Returns a response to the request by invoking a given asynchronous callback that returns the final response, or
    /// an error.
    pub async fn result<T, TRet, F, Fut>(self, process: F) -> Result<Response<T>, Status>
    where
        F: Fn(&'a Modtorio, TReq) -> Fut,
        Fut: Future<Output = RpcResult<TRet>>,
        TRet: Into<T>,
    {
        self.log_request();
        self.assert_status().await?;

        let msg = self.request.into_inner();
        match process(self.instance, msg).await {
            Ok(result) => Ok(Response::new(result.into())),
            Err(e) => {
                error!("RPC: {}", e);
                Err(e.into())
            }
        }
    }

    /// Returns a response to the request by invoking a given asynchronous worker callback that returns a stream of
    /// responses.
    pub async fn stream<F, Fut>(
        self,
        process: F,
    ) -> Result<Response<mpsc::Receiver<Result<rpc::Progress, Status>>>, Status>
    where
        F: FnOnce(Modtorio, TReq, AsyncProgressChannel) -> Fut,
        Fut: Future,
    {
        self.log_request();
        self.assert_status().await?;

        let (tx, rx) = channel();
        let msg = self.request.into_inner();
        let instance = self.instance.clone();
        process(instance, msg, tx).await;

        let resp = Response::new(rx);
        Ok(resp)
    }

    /// Logs the handler's request.
    fn log_request(&self) {
        debug!(
            "RPC request from {}: {:?}",
            self.request
                .remote_addr()
                // TODO: this is a bit of stupid hack but; when using an Unix socket, the RPC server takes in a stream
                // of incoming connections which *don't* include the peer's socket address. in which
                // case the socket address here is None, so just call it "Unix"
                .map_or_else(|| String::from("Unix"), |addr| addr.to_string()),
            self.request.get_ref()
        );
        debug!("{:?}", self.request);
    }

    /// Asserts that the Modtorio instance's status is the wanted one, if any.
    async fn assert_status(&self) -> Result<(), RpcError> {
        if let Some(required) = self.required_status {
            let status = self.instance.get_instance_status().await;
            if status != required {
                error!(
                    "RPC instance status assertion failed: wanted {:?}, actual {:?}",
                    required, status
                );

                return Err(RpcError::InvalidInstanceStatus {
                    wanted: required,
                    actual: status,
                });
            }
        }

        Ok(())
    }
}

/// Returns an `mpsc` channel.
fn channel<T>() -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
    let (tx, rx) = mpsc::channel(64);
    (tx, rx)
}
