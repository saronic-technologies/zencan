//! Common traits

use core::time::Duration;
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

/// A synchronous can sender
pub trait CanSender {
    /// Send a message to the bus
    fn send(&mut self, msg: CanMessage) -> Result<(), CanSendError>;
}

/// A synchronous can receiver
pub trait CanReceiver {
    /// The error type returned by recv
    type Error;
    /// Attempt to read a message from the receiver, and return None immediately if no message is
    /// available
    fn try_recv(&mut self) -> Option<CanMessage>;
    /// A blocking receive with timeout
    fn recv(&mut self, timeout: Duration) -> Result<CanMessage, Self::Error>;
}

/// An async CAN sender trait
#[async_trait]
pub trait AsyncCanSender: Send {
    /// Send a message to the bus
    async fn send(
        &mut self,
        msg: CanMessage,
    ) -> anyhow::Result<()>;
}

/// An async CAN receiver trait
#[async_trait]
pub trait AsyncCanReceiver: Send {
    /// Receive available message immediately
    fn try_recv(&mut self) -> anyhow::Result<Option<CanMessage>>;

    /// A blocking receive
    async fn recv(
        &mut self,
    ) -> anyhow::Result<CanMessage>;

    /// Remove any pending messages from the receiver
    fn flush(&mut self) -> anyhow::Result<()> {
        while self.try_recv()?.is_some() {}
        Ok(())
    }
}
