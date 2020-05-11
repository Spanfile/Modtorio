use anyhow::anyhow;
use async_trait::async_trait;
use tokio::prelude::*;

#[async_trait]
pub trait ResponseExt {
    async fn to_writer<W>(&mut self, dest: &mut W) -> anyhow::Result<usize>
    where
        W: AsyncWrite + Unpin + Send;

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
            .ok_or_else(|| {
                anyhow!(
                    "Response URL doesn't have a file name component ({})",
                    self.url().as_str()
                )
            })
    }
}
