use crate::error::ResponseError;
use async_trait::async_trait;
use tokio::prelude::*;

/// Collection of common functions used with HTTP responses.
#[async_trait]
pub trait ResponseExt {
    /// Asynchronously copies the response body to a `Writer` object.
    async fn to_writer<W>(&mut self, dest: &mut W) -> anyhow::Result<usize>
    where
        W: AsyncWrite + Unpin + Send;

    /// Extracts the file name from the response URL. Returns `ResponseError::NoFilename` if the URL
    /// doesn't have a file name.
    fn url_file_name(&self) -> anyhow::Result<&str>;
}

#[async_trait]
impl ResponseExt for reqwest::Response {
    async fn to_writer<W>(&mut self, dest: &mut W) -> anyhow::Result<usize>
    where
        W: AsyncWrite + Unpin + Send,
    {
        let mut written = 0;
        while let Some(chunk) = self.chunk().await? {
            written += chunk.len();
            dest.write_all(&chunk).await?;
        }

        Ok(written)
    }

    fn url_file_name(&self) -> anyhow::Result<&str> {
        self.url()
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .ok_or_else(|| ResponseError::NoFilename.into())
    }
}
