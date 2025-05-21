use core::time::Duration;

use crate::messages::CanMessage;

pub trait CanSender {
    fn send(&mut self, msg: CanMessage) -> Result<(), CanMessage>;
}

pub trait CanReceiver {
    fn try_recv(&mut self) -> Option<CanMessage>;
    /// A blocking receive
    fn recv(&mut self, timeout: Duration) -> Result<CanMessage, ()>;
}

pub trait AsyncCanSender {
    fn send(
        &mut self,
        msg: CanMessage,
    ) -> impl core::future::Future<Output = Result<(), CanMessage>>;
}

pub trait AsyncCanReceiver {
    type Error: core::fmt::Debug;

    /// Receive available message immediately
    fn try_recv(&mut self) -> Option<CanMessage>;

    /// A blocking receive
    fn recv(&mut self) -> impl core::future::Future<Output = Result<CanMessage, Self::Error>>;

    /// Remove any pending messages from the receiver
    fn flush(&mut self) {
        while let Some(_) = self.try_recv() {}
    }
}
