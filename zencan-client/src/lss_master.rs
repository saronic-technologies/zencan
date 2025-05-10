use core::time::Duration;

use tokio::time::timeout_at;
use zencan_common::{
    NodeId,
    lss::{LSS_FASTSCAN_CONFIRM, LssIdentity, LssRequest, LssResponse},
    traits::{AsyncCanReceiver, AsyncCanSender},
};

use snafu::Snafu;

pub struct LssMaster<S, R> {
    sender: S,
    receiver: R,
}

#[derive(Debug, Snafu, Clone, Copy)]
pub enum LssError {
    #[snafu(display("Timed out waiting for LSS response"))]
    Timeout,
    #[snafu(display(
        "LSS slave returned an error in response to ConfigBitTiming command. error: {}, Spec error: {}",
        error,
        spec_error
    ))]
    BitTimingConfigError { error: u8, spec_error: u8 },
    #[snafu(display(
        "LSS slave returned an error in response to ConfigNodeId command. error: {}, Spec error: {}",
        error,
        spec_error
    ))]
    NodeIdConfigError { error: u8, spec_error: u8 },
}

// /// Send a sequence of messages to put a single node into configuration mode based on its identity
// ///
// /// Returns `Ok(())` if the node was successfully put into configuration mode, or an
// /// `Err(LssError::Timeout)` if no node responded
// pub async fn activate_configure_by_identity(
//     vendor_id: u32,
//     product_code: u32,
//     revision: u32,
//     serial: u32,
//     mut send_fn: impl AsyncFnMut(LssRequest, Duration) -> Option<LssResponse>,
// ) -> Result<(), LssError> {
//     const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
//     // Send global mode to put all nodes into waiting state. No response expected.
//     send_fn(LssRequest::SwitchModeGlobal { mode: 0 }, Duration::ZERO).await;

//     // Now send the identity messages. If a LSS slave node recognizes its identity, it will respond
//     // to the serial setting message with a SwitchStateResponse message
//     send_fn(LssRequest::SwitchStateVendor { vendor_id }, Duration::ZERO).await;
//     send_fn(
//         LssRequest::SwitchStateProduct { product_code },
//         Duration::ZERO,
//     )
//     .await;
//     send_fn(LssRequest::SwitchStateRevision { revision }, Duration::ZERO).await;
//     match send_fn(LssRequest::SwitchStateSerial { serial }, RESPONSE_TIMEOUT).await {
//         Some(LssResponse::SwitchStateResponse) => Ok(()),
//         _ => Err(LssError::Timeout),
//     }
// }

// /// Create a closure that sends a message and waits for a response which matches the filter
// ///
// /// # Arguments:
// ///
// /// * `receiver` - A AsyncCanReceiver implementation for reading messages
// /// * `sender` - A AsyncCanSender implementation for sending messages
// /// * `filter` - A closure that takes a CanMessage and returns an Option<T> when it is passed a
// ///   matching message
// async fn filtered_send_fn<'a, R: AsyncCanReceiver, S: AsyncCanSender, REQ: Into<CanMessage>, T>(
//     receiver: &'a mut R,
//     sender: &'a mut S,
//     filter: &'a impl Fn(CanMessage) -> Option<T>,
// ) -> impl AsyncFnMut(REQ, Duration) -> Option<T> + 'a {
//     async |send_msg: REQ, timeout: Duration| {
//         let send_msg = send_msg.into();
//         receiver.flush();
//         if let Err(e) = sender.send(send_msg).await {
//             log::error!("Failed to send message: {:?}", send_msg);
//             return None;
//         }

//         let wait_until = Instant::now() + timeout;
//         loop {
//             let timeout = wait_until.saturating_duration_since(Instant::now());
//             if timeout.is_zero() {
//                 return None;
//             }
//             if let Ok(msg) = receiver.recv(timeout).await {
//                 if let Some(response) = filter(msg) {
//                     return Some(response);
//                 }
//             }
//         }
//     }
// }

impl<S: AsyncCanSender, R: AsyncCanReceiver> LssMaster<S, R> {
    pub fn new(sender: S, receiver: R) -> Self {
        Self { sender, receiver }
    }

    pub async fn configure_by_identity(
        &mut self,
        vendor_id: u32,
        product_code: u32,
        revision: u32,
        serial: u32,
        node_id: NodeId,
        baud_rate_table: u8,
        baud_rate_index: u8,
    ) -> Result<(), LssError> {
        // Put the specified node into configuration mode
        self.enter_config_by_identity(vendor_id, product_code, revision, serial)
            .await?;
        // set the node ID
        self.set_node_id(node_id).await?;
        // Set the bit rate
        self.set_baud_rate(baud_rate_table, baud_rate_index).await?;

        Ok(())
    }

    /// Send a sequence of messages to put a single node into configuration mode based on its identity
    pub async fn enter_config_by_identity(
        &mut self,
        vendor_id: u32,
        product_code: u32,
        revision: u32,
        serial: u32,
    ) -> Result<(), LssError> {
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        // Send global mode to put all nodes into waiting state. No response expected.
        self.send_and_receive(LssRequest::SwitchModeGlobal { mode: 0 }, Duration::ZERO)
            .await;

        // Now send the identity messages. If a LSS slave node recognizes its identity, it will respond
        // to the serial setting message with a SwitchStateResponse message
        self.send_and_receive(LssRequest::SwitchStateVendor { vendor_id }, Duration::ZERO)
            .await;
        self.send_and_receive(
            LssRequest::SwitchStateProduct { product_code },
            Duration::ZERO,
        )
        .await;
        self.send_and_receive(LssRequest::SwitchStateRevision { revision }, Duration::ZERO)
            .await;
        match self
            .send_and_receive(LssRequest::SwitchStateSerial { serial }, RESPONSE_TIMEOUT)
            .await
        {
            Some(LssResponse::SwitchStateResponse) => Ok(()),
            _ => Err(LssError::Timeout),
        }
    }

    /// Send a command to set the baud rate on the LSS slave current in configuration mode
    ///
    /// The node must have been put into configuration mode already.
    ///
    /// Returns Err(LssError::Timeout) if the node does not respond to the command, or
    /// Err(LssError::ConfigError) if the node responds with an error.
    ///
    /// # Arguments
    /// * `table` - The index of the table of baud rate settings to use (0 for the default CANOpen
    ///   table)
    /// * `index` - The index into the table of the baud rate setting to use
    pub async fn set_baud_rate(&mut self, table: u8, index: u8) -> Result<(), LssError> {
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        match self
            .send_and_receive(
                LssRequest::ConfigureBitTiming { table, index },
                RESPONSE_TIMEOUT,
            )
            .await
        {
            Some(LssResponse::ConfigureBitTimingAck { error, spec_error }) => {
                if error == 0 {
                    Ok(())
                } else {
                    Err(LssError::BitTimingConfigError { error, spec_error })
                }
            }
            _ => Err(LssError::Timeout),
        }
    }

    /// Send a command to set the node ID on the LSS slave current in configuration mode
    ///
    /// The node must have been put into configuration mode already.
    ///
    /// Returns Err(LssError::Timeout) if the node does not respond to the command, or
    /// Err(LssError::ConfigError) if the node responds with an error.
    pub async fn set_node_id(&mut self, node_id: NodeId) -> Result<(), LssError> {
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        match self
            .send_and_receive(
                LssRequest::ConfigureNodeId {
                    node_id: node_id.into(),
                },
                RESPONSE_TIMEOUT,
            )
            .await
        {
            Some(LssResponse::ConfigureNodeIdAck { error, spec_error }) => {
                if error == 0 {
                    Ok(())
                } else {
                    Err(LssError::NodeIdConfigError { error, spec_error })
                }
            }
            _ => Err(LssError::Timeout),
        }
    }

    /// Perform a fast scan of the network to find unconfigured nodes
    pub async fn fast_scan(&mut self) -> Option<LssIdentity> {
        const TIMEOUT: Duration = Duration::from_millis(15);
        let mut id = [0, 0, 0, 0];
        let mut sub = 0;
        let mut next = 0;
        let mut bit_check;

        let mut send_fs = async |id: &[u32; 4], bit_check: u8, sub: u8, next: u8| -> bool {
            match self
                .send_and_receive(
                    LssRequest::FastScan {
                        id: id[sub as usize],
                        bit_check,
                        sub,
                        next,
                    },
                    TIMEOUT,
                )
                .await
            {
                Some(LssResponse::IdentifySlave) => true,
                _ => false,
            }
        };

        // The first message resets the LSS state machines, and a response confirms that there is at
        // least one unconfigured slave to discover
        if !send_fs(&id, LSS_FASTSCAN_CONFIRM, sub, next).await {
            return None;
        }
        while sub < 4 {
            bit_check = 32;
            while bit_check > 0 {
                bit_check -= 1;
                if !send_fs(&id, bit_check, sub, next).await {
                    id[sub as usize] |= 1 << bit_check;
                }
            }
            next = (sub + 1) % 4;
            if !send_fs(&id, bit_check, sub, next).await {
                return None;
            }
            sub += 1;
        }

        Some(LssIdentity {
            vendor_id: id[0],
            product_code: id[1],
            revision: id[2],
            serial_number: id[3],
        })
    }

    pub async fn send_and_receive(
        &mut self,
        msg: LssRequest,
        timeout: Duration,
    ) -> Option<LssResponse> {
        self.sender.send(msg.into()).await.ok()?;

        let wait_until = tokio::time::Instant::now() + timeout;
        loop {
            match timeout_at(wait_until, self.receiver.recv()).await {
                // Got a message
                Ok(Ok(msg)) => {
                    match msg.try_into() {
                        Ok(lss_resp) => return Some(lss_resp),
                        // Failed to convert message into LSS response. Skip it.
                        Err(_) => {}
                    }
                }
                // `recv` returned without a message. Keep waiting.
                Ok(Err(e)) => {
                    log::error!("Error reading can socket: {e:?}");
                    return None;
                }
                // Timeout waiting
                Err(_) => return None,
            }
        }
    }
}
