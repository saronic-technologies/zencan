use core::time::Duration;

use crate::messages::CanMessage;

pub trait CanSender {
    fn send(&mut self, msg: CanMessage) -> Result<(), CanMessage>;
}

pub trait CanReceiver {
    type Error;
    fn try_recv(&mut self) -> Option<CanMessage>;
    /// A blocking receive
    fn recv(&mut self, timeout: Duration) -> Result<CanMessage, Self::Error>;
}

pub trait AsyncCanSender: Send {
    fn send(
        &mut self,
        msg: CanMessage,
    ) -> impl core::future::Future<Output = Result<(), CanMessage>>;
}

pub trait AsyncCanReceiver: Send {
    type Error: core::fmt::Debug + Send;

    /// Receive available message immediately
    fn try_recv(&mut self) -> Option<CanMessage>;

    /// A blocking receive
    fn recv(
        &mut self,
    ) -> impl core::future::Future<Output = Result<CanMessage, Self::Error>> + Send;

    /// Remove any pending messages from the receiver
    fn flush(&mut self) {
        while self.try_recv().is_some() {}
    }
}
