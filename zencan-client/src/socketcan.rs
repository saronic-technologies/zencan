use std::sync::Arc;

use socketcan::tokio::CanSocket;

#[derive(Debug, Clone)]
pub struct SocketCanReceiver {
    socket: Arc<CanSocket>,
}

#[derive(Debug, Clone)]
pub struct SocketCanSender {
    socket: Arc<CanSocket>,
}

/// Open a socketcan device and split it into a sender and receiver object for use with zencan
/// library
///
/// # Arguments
/// * `device` - The name of the socketcan device to open, e.g. "vcan0", or "can0"
///
/// A key benefit of this is that by creating both sender and receiver objects from a shared socket,
/// the receiver will not receive messages sent by the sender.
pub fn open_socketcan<S: AsRef<str>>(device: S) -> (SocketCanSender, SocketCanReceiver) {
    let device: &str = device.as_ref();
    let socket = CanSocket::open(device).unwrap();
    let socket = Arc::new(socket);
    let receiver = SocketCanReceiver { socket: socket.clone() };
    let sender = SocketCanSender {socket };
    (sender, receiver)
}