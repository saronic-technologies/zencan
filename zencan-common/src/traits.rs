//! Common traits

use core::{fmt::Debug};
use std::error::Error;

use async_trait::async_trait;

use crate::messages::CanMessage;

/// Error type for CAN send operations containing the failed message
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct CanSendError(pub CanMessage);

impl core::fmt::Display for CanSendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Failed to send CAN message: {:?}", self.0)
    }
}

impl Error for CanSendError {}

/// A trait for accessing a value
///
/// E.g. from an AtomicCell
pub trait LoadStore<T> {
    /// Read the value
    fn load(&self) -> T;
    /// Store a new value to the
    fn store(&self, value: T);
}

/// An async CAN sender trait
#[async_trait]
pub trait AsyncCanSender: Send + Sync {
    /// Send a message to the bus
    async fn send(
        &self,
        msg: CanMessage,
    ) -> anyhow::Result<()>;
}

/// An async CAN receiver trait
#[async_trait]
pub trait AsyncCanReceiver: Send + Sync {
    /// An async receive
    async fn recv(
        &self,
    ) -> anyhow::Result<CanMessage>;
}
