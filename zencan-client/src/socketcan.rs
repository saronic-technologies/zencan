use std::sync::Arc;

use futures::FutureExt as _;
use snafu::{ResultExt, Snafu};
use socketcan::{tokio::CanSocket, CanFrame, EmbeddedFrame, Frame, ShouldRetry};
use zencan_common::{
    messages::{CanError, CanMessage},
    traits::{AsyncCanReceiver, AsyncCanSender},
};

fn socketcan_id_to_zencan_id(id: socketcan::CanId) -> zencan_common::messages::CanId {
    match id {
        socketcan::CanId::Standard(id) => zencan_common::messages::CanId::std(id.as_raw()),
        socketcan::CanId::Extended(id) => zencan_common::messages::CanId::extended(id.as_raw()),
    }
}

fn zencan_id_to_socketcan_id(id: zencan_common::messages::CanId) -> socketcan::CanId {
    match id {
        zencan_common::messages::CanId::Extended(id) => {
            socketcan::ExtendedId::new(id).unwrap().into()
        }
        zencan_common::messages::CanId::Std(id) => socketcan::StandardId::new(id).unwrap().into(),
    }
}

fn socketcan_frame_to_zencan_message(frame: socketcan::CanFrame) -> Result<CanMessage, CanError> {
    let id = socketcan_id_to_zencan_id(frame.can_id());

    match frame {
        CanFrame::Data(frame) => Ok(CanMessage::new(id, frame.data())),
        CanFrame::Remote(_) => Ok(CanMessage::new_rtr(id)),
        CanFrame::Error(frame) => Err(CanError::from_raw(frame.error_bits() as u8)),
    }
}

fn zencan_message_to_socket_frame(frame: CanMessage) -> socketcan::CanFrame {
    let id = zencan_id_to_socketcan_id(frame.id());

    if frame.is_rtr() {
        socketcan::CanFrame::new_remote(id, 0).unwrap()
    } else {
        socketcan::CanFrame::new(id, frame.data()).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct SocketCanReceiver {
    socket: Arc<CanSocket>,
}

#[derive(Debug, Snafu)]
pub enum ReceiveError {
    Io { source: socketcan::IoError },
    Can { source: zencan_common::CanError }

}

impl AsyncCanReceiver for SocketCanReceiver {
    type Error = ReceiveError;

    fn try_recv(&mut self) -> Option<CanMessage> {
        // let mut socket = self.socket.lock().unwrap();
        // let x = futures::poll!(socket.next());
        // match x {
        //     std::task::Poll::Ready(Some(Ok(frame))) => {
        //         let msg = socketcan_frame_to_zencan_message(frame);
        //         if msg.is_err() {
        //             return None;
        //         }
        //         Some(msg.unwrap())
        //     }
        //     _ => None,
        // }
        match self.socket.try_read_frame() {
            Ok(frame) => Some(socketcan_frame_to_zencan_message(frame).unwrap()),
            _ => None,
        }
    }

    async fn recv(&mut self) -> Result<CanMessage, ReceiveError> {
        loop {
            match self.socket.read_frame().await {
                Ok(frame) => return socketcan_frame_to_zencan_message(frame).context(CanSnafu),
                Err(e) => {
                    if !e.should_retry() {
                        return Err(ReceiveError::Io { source: e })
                    }
                },
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SocketCanSender {
    socket: Arc<CanSocket>,
}

impl AsyncCanSender for SocketCanSender {
    async fn send(&mut self, msg: CanMessage) -> Result<(), CanMessage> {
        let socketcan_frame = zencan_message_to_socket_frame(msg);

        let result = self.socket.write_frame(socketcan_frame).await;
        if result.is_err() { Err(msg) } else { Ok(()) }
    }
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
    let receiver = SocketCanReceiver {
        socket: socket.clone(),
    };
    let sender = SocketCanSender { socket };
    (sender, receiver)
}
