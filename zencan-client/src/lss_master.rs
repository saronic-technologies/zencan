//! A
use core::time::Duration;

use tokio::time::timeout_at;
use zencan_common::{
    lss::{LssIdentity, LssRequest, LssResponse, LssState},
    traits::{AsyncCanReceiver, AsyncCanSender},
};

use snafu::Snafu;

#[derive(Debug)]
/// Struct to interact with nodes using the LSS protocol
pub struct LssMaster<S, R> {
    sender: S,
    receiver: R,
}

/// Error returned by [`LssMaster`]
#[derive(Debug, Snafu, Clone, Copy)]
pub enum LssError {
    /// Timed out while waiting for an expected LSS response
    #[snafu(display("Timed out waiting for LSS response"))]
    Timeout,
    /// The LSS slave returned an error code in response to a ConfigBitTiming command
    #[snafu(display(
        "LSS slave returned an error in response to ConfigBitTiming command. error: {}, Spec error: {}",
        error,
        spec_error
    ))]
    BitTimingConfigError {
        /// Error code
        ///
        /// 1 - Baudrate not supported
        /// 255 - Special error code in spec_error
        error: u8,
        /// Manufacturer specific error code
        ///
        /// Only supposed to be valid when error is 255
        spec_error: u8,
    },
    /// The LSS slave returned an error code in response to a ConfigNodeId command
    #[snafu(display(
        "LSS slave returned an error in response to ConfigNodeId command. error: {}, Spec error: {}",
        error,
        spec_error
    ))]
    NodeIdConfigError {
        /// Error code
        ///
        /// 1 - Node address is invalid
        /// 255 - Special error code in spec_error
        error: u8,
        /// Manufacturer specific error code
        ///
        /// Only supposed to be valid when error is 255
        spec_error: u8,
    },
    /// The LSS slave returned an error code in response to a StoreConfiguration command
    #[snafu(display(
        "LSS slave returned an error in response to StoreConfiguration. error: {}, Spec error: {}",
        error,
        spec_error
    ))]
    NodeStoreConfigError {
        /// Error code
        ///
        /// 1 - Node does not support storing configuration
        /// 255 - Special error code in spec_error
        error: u8,
        /// Manufacturer specific error code
        ///
        /// Only supposed to be valid when error is 255
        spec_error: u8,
    },
    /// The provided node ID is invalid (must be 1-127)
    InvalidNodeIdError
}

impl<S: AsyncCanSender, R: AsyncCanReceiver> LssMaster<S, R> {
    /// Create a new LssMaster
    ///
    /// # Arguments
    /// - `sender`: An object which implements [`AsyncCanSender`] to be used for sending messages to
    ///   the bus
    /// - `receiver`: An object which implements [`AsyncCanReceiver`] to be used for receiving
    ///   messages from the bus
    ///
    /// When using socketcan, these can be created with [`crate::open_socketcan`].
    pub fn new(sender: S, receiver: R) -> Self {
        Self { sender, receiver }
    }

    /// Perform a fast scan of the network to find unconfigured nodes
    ///
    /// # Arguments
    /// * `timeout` - The duration of time to wait for responses after each message.
    ///   Duration::from_millis(20) is probably a pretty safe value, but this depends on the
    ///   responsiveness of the slaves, and on the amount of bus traffic. If the timeout is set too
    ///   short, the scan may fail to find existing nodes.
    pub async fn fast_scan(&mut self, timeout: Duration) -> Option<LssIdentity> {
        let mut id = [0, 0, 0, 0];
        let mut sub = 0;
        let mut next = 0;
        let mut bit_check;

        let mut send_fs = async |id: &[u32; 4], bit_check: u8, sub: u8, next: u8| -> bool {
            // Unlike send_and_receive, this function always waits the full timeout, because we don't know
            // how many nodes will respond to us, so we need to give them time.
            self.sender
                .send(
                    LssRequest::FastScan {
                        id: id[sub as usize],
                        bit_check,
                        sub,
                        next,
                    }
                    .into(),
                )
                .await
                .ok();

            let wait_until = tokio::time::Instant::now() + timeout;
            let mut resp_flag = false;
            loop {
                match timeout_at(wait_until, self.receiver.recv()).await {
                    // timeout
                    Err(_) => break,
                    Ok(Ok(msg)) => {
                        if let Ok(LssResponse::IdentifySlave) = LssResponse::try_from(msg) {
                            resp_flag = true;
                        }
                    }
                    _ => (),
                }
            }
            resp_flag
        };

        // !!! I don't think this is correct for FastScan; it's checking to see if there is an
        // !!! unconfigured node with the FASTSCAN_CONFIRM, but it's checking against all 0's,
        // !!! which doesn't seem correct either

        // if !send_fs(&id, LSS_FASTSCAN_CONFIRM, sub, next).await {
        //     return None;
        // }
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
            serial: id[3],
        })
    }

    /// Send command to the bus to set the LSS mode for all nodes
    pub async fn set_global_mode(&mut self, mode: LssState) {
        // Send global mode to put all nodes into waiting state. No response expected.
        self.send_and_receive(
            LssRequest::SwitchModeGlobal { mode: mode as u8 },
            Duration::ZERO,
        )
        .await;
    }

    async fn send_and_receive(
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
                // !!! Is this correct??
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
