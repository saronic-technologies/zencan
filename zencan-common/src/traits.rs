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
    fn send(&mut self, msg: CanMessage) -> impl core::future::Future<Output = Result<(), CanMessage>>;
}

pub trait AsyncCanReceiver {
    fn try_recv(&mut self) -> impl core::future::Future<Output = Option<CanMessage>>;
    /// A blocking receive
    fn recv(&mut self, timeout: Duration) ->  impl core::future::Future<Output = Result<CanMessage, ()>>;
}
