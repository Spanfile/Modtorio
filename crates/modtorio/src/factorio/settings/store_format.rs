//! Provides the [`StoreFormatConversion`](StoreFormatConversion) trait used to translate a Modtorio server's
//! [`ServerSettings`](super::ServerSettings) into its store format and vice versa.

use crate::store::models::GameSettings;

/// Defines the functions used to convert a value into the kind used in an RPC message and vice versa.
pub trait StoreFormatConversion
where
    Self: Sized,
{
    /// Creates a new instance of `Self` from a given `GameSettings` struct.
    fn from_store_format(store_format: &GameSettings) -> anyhow::Result<Self>;
    /// Modifies an existing `GameSettings` struct with self's own settings.
    fn to_store_format(&self, store_format: &mut GameSettings) -> anyhow::Result<()>;
}
