tonic::include_proto!("mod_rpc");

/// The version of the RPC protocol buffer specification.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

impl Into<Empty> for () {
    fn into(self) -> Empty {
        Empty {}
    }
}
