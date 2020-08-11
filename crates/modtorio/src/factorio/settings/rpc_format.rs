//! Provides the [`RpcFormatConversion`](RpcFormatConversion) trait used to translate a Modtorio server's
//! [`ServerSettings`](super::ServerSettings) into its RPC format and vice versa.

/// Defines the functions used to convert a value into the kind used in an RPC message and vice versa.
pub trait RpcFormatConversion
where
    Self: Sized,
{
    /// Creates a new instance of `Self` from a given `rpc::ServerSettings` struct.
    fn from_rpc_format(rpc_format: &rpc::ServerSettings) -> anyhow::Result<Self>;
    /// Modifies an existing `rpc::ServerSettings` struct with self's own settings.
    fn to_rpc_format(&self, rpc_format: &mut rpc::ServerSettings) -> anyhow::Result<()>;
}
