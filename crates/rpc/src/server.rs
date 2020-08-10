mod mod_rpc {
    tonic::include_proto!("mod_rpc");
}

use common::net::NetAddress;
use log::*;
use mod_rpc::{
    mod_rpc_server::{ModRpc, ModRpcServer},
    Empty, StatusResponse,
};
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug, Default)]
pub struct MyModRpc {}

#[tonic::async_trait]
impl ModRpc for MyModRpc {
    async fn get_status(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<StatusResponse>, Status> {
        debug!("Got status request");

        let reply = StatusResponse { uptime: 1 };

        Ok(Response::new(reply))
    }
}

pub async fn run(listen: NetAddress) -> anyhow::Result<()> {
    let addr = match listen {
        NetAddress::TCP(addr) => addr,
        NetAddress::Unix(_) => unimplemented!(),
    };
    let server = MyModRpc::default();

    debug!("Starting RPC server. Listening on {}", addr);
    Server::builder()
        .add_service(ModRpcServer::new(server))
        .serve(addr)
        .await?;
    Ok(())
}
