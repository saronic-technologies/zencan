//! Utility for sharing a single socket among tasks
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;

use zencan_common::{traits::{AsyncCanSender, CanSendError}, CanMessage};

#[derive(Debug)]
pub struct SharedSender<S: AsyncCanSender> {
    inner: Arc<Mutex<S>>,
}

impl<S: AsyncCanSender> Clone for SharedSender<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<S: AsyncCanSender> SharedSender<S> {
    pub fn new(sender: Arc<Mutex<S>>) -> Self {
        Self { inner: sender }
    }

    async fn send(&mut self, msg: CanMessage) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        Ok(inner.send(msg).await?)
    }
}

#[async_trait]
impl<S: AsyncCanSender> AsyncCanSender for SharedSender<S> {
    async fn send(
        &mut self,
        msg: CanMessage,
    ) -> anyhow::Result<()> {
        Ok(self.send(msg).await?)
    }
}
