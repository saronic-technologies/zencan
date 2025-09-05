use std::sync::Arc;

use zencan_common::{
    messages::{CanError, CanId, CanMessage},
    traits::{AsyncCanReceiver, AsyncCanSender, CanSendError},
};
use snafu::{ResultExt, Snafu};

#[cfg(feature = "socketcan")]
use socketcan::{tokio::CanSocket, CanFilter, CanFrame, EmbeddedFrame, Frame, ShouldRetry, IoError, SocketOptions};

fn socketcan_id_to_zencan_id(id: socketcan::CanId) -> CanId {
    match id {
        socketcan::CanId::Standard(id) => CanId::std(id.as_raw()),
        socketcan::CanId::Extended(id) => CanId::extended(id.as_raw()),
    }
}

fn zencan_id_to_socketcan_id(id: CanId) -> socketcan::CanId {
    match id {
        CanId::Extended(id) => socketcan::ExtendedId::new(id).unwrap().into(),
        CanId::Std(id) => socketcan::StandardId::new(id).unwrap().into(),
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

// We want to support filtering, but we also want socketcan-rs to only be used
// here, so external callers use this struct instead of socketcan's filter
/// A CAN filter for socketcan interfaces.
/// 
/// This struct wraps socketcan filter functionality, allowing callers to filter
/// incoming CAN messages by ID and mask without directly depending on socketcan types.
#[derive(Copy, Clone)]
pub struct SocketCanFilter {
    id :u32,
    mask :u32
}

impl SocketCanFilter {
    /// Create a new CAN filter with the specified ID and mask.
    /// 
    /// # Arguments
    /// * `id` - The CAN ID to filter for
    /// * `mask` - The mask to apply to the ID filter
    /// 
    /// # Returns
    /// A new `SocketCanFilter` instance
    pub fn new(
        id :u32,
        mask :u32
    ) -> Self {
        Self {
            id,
            mask
        }
    }
}

/// A socketcan-based CAN message receiver.
/// 
/// This struct implements `AsyncCanReceiver` for receiving CAN messages from a socketcan interface.
/// Multiple receivers can share the same underlying socket through `Arc<CanSocket>`.
#[derive(Debug, Clone)]
pub struct SocketCanReceiver {
    socket: Arc<CanSocket>,
}

#[derive(Debug, Snafu)]
pub enum ReceiveError {
    Io { source: socketcan::IoError },
    Can { source: CanError },
}

impl AsyncCanReceiver for SocketCanReceiver {
    type Error = ReceiveError;

    fn try_recv(&mut self) -> Result<Option<CanMessage>, ReceiveError> {
        panic!("Not supported with socketcan::tokio");
    //    match self.socket.try_read_frame() {
    //        Ok(frame) => Some(socketcan_frame_to_zencan_message(frame).unwrap()),
    //        _ => None,
    //    }
    }

    async fn recv(&mut self) -> Result<CanMessage, ReceiveError> {
        loop {
            match self.socket.read_frame().await {
                Ok(frame) => return socketcan_frame_to_zencan_message(frame).context(CanSnafu),
                Err(e) => {
                    if !e.should_retry() {
                        return Err(ReceiveError::Io { source: e });
                    }
                }
            }
        }
    }
}

/// A socketcan-based CAN message sender.
/// 
/// This struct implements `AsyncCanSender` for sending CAN messages to a socketcan interface.
/// Multiple senders can share the same underlying socket through `Arc<CanSocket>`.
#[derive(Debug, Clone)]
pub struct SocketCanSender {
    socket: Arc<CanSocket>,
}

impl AsyncCanSender for SocketCanSender {
    async fn send(&mut self, msg: CanMessage) -> Result<(), CanSendError> {
        let result = self.socket.write_frame(zencan_message_to_socket_frame(msg)).await;
        if result.is_err() {
            Err(CanSendError(msg))
        } else {
            Ok(())
        }
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
#[cfg_attr(docsrs, doc(cfg(feature = "socketcan")))]
pub fn open_socketcan<S: AsRef<str>>(
    device: S,
    filters :Option<&[SocketCanFilter]>
) -> Result<(SocketCanSender, SocketCanReceiver), IoError> {
    let device: &str = device.as_ref();
    let socket = CanSocket::open(device)?;
    if let Some(socket_filters) = filters {
        // Map our SocketCanFilters to the native CanFilter
        let mapped_filters :Vec<CanFilter> = socket_filters.iter().
            map(|filter| CanFilter::new(filter.id, filter.mask)).collect();
        socket.set_filters(
            &mapped_filters
        )?;
    }
    let socket = Arc::new(socket);
    let receiver = SocketCanReceiver {
        socket: socket.clone(),
    };
    let sender = SocketCanSender { socket };
    Ok((sender, receiver))
}
