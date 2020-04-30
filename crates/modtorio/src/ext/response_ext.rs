use async_trait::async_trait;
use tokio::prelude::*;

#[async_trait]
pub trait ResponseExt {
    async fn to_writer<W>(&mut self, dest: &mut W) -> anyhow::Result<usize>
    where
        W: AsyncWrite + Unpin + Send;
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
}
