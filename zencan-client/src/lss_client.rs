// We use the LSS client to bind an LSS identity to an object that we
// can use to set a baud rate or a node ID

use std::time::Duration;

use tokio::time::timeout_at;
use zencan_common::{lss::{LssIdentity, LssRequest, LssResponse}, AsyncCanReceiver, AsyncCanSender, NodeId};

use crate::LssError;

/// LSS client for configuring devices with specific LSS identities
pub struct LssClient<'struct_lifetime, ErrorType>
    where ErrorType: std::error::Error + Send + 'static {
    sender :Box<dyn AsyncCanSender + 'struct_lifetime>,
    receiver :Box<dyn AsyncCanReceiver<Error = ErrorType> + 'struct_lifetime>,
    identity :LssIdentity
}

impl<'struct_lifetime, ErrorType :std::error::Error + Send + 'static> LssClient<'struct_lifetime, ErrorType> {
    /// Create a new LSS client bound to a specific device identity
    pub fn new(
        sender :Box<dyn AsyncCanSender +'struct_lifetime>,
        receiver :Box<dyn AsyncCanReceiver<Error = ErrorType> + 'struct_lifetime>,
        identity :LssIdentity
    ) -> Self {
        Self {
            sender,
            receiver,
            identity
        }
    }

    /// Send a sequence of messages to put a single node into configuration mode based on its identity
    pub async fn enter_configuration_mode(
        &mut self,
    ) -> Result<(), LssError> {
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        // Send global mode to put all nodes into waiting state. No response expected.
        self.send_and_receive(LssRequest::SwitchModeGlobal { mode: 0 }, None)
            .await;

        let vendor_id = self.identity.vendor_id;
        // Now send the identity messages. If a LSS slave node recognizes its identity, it will respond
        // to the serial setting message with a SwitchStateResponse message
        self.send_and_receive(LssRequest::SwitchStateVendor { vendor_id }, None)
            .await;

        let product_code = self.identity.product_code;
        self.send_and_receive(
            LssRequest::SwitchStateProduct { product_code },
            None,
        )
        .await;
        
        let revision = self.identity.revision;
        self.send_and_receive(LssRequest::SwitchStateRevision { revision }, None)
            .await;

        let serial = self.identity.serial;
        match self
            .send_and_receive(LssRequest::SwitchStateSerial { serial }, None)
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
                Some(RESPONSE_TIMEOUT),
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
    pub async fn set_node_id(&mut self, node_id: u8) -> Result<(), LssError> {
        let node_id_object = NodeId::new(node_id).map_err(|_| LssError::InvalidNodeIdError)?;
        
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        match self
            .send_and_receive(
                LssRequest::ConfigureNodeId {
                    node_id: node_id_object.into(),
                },
                Some(RESPONSE_TIMEOUT),
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

    /// Send command to store configuration
    ///
    /// The node must have been put into configuration mode already.
    ///
    /// Returns Err(LssError::Timeout) if the node does not respond to the command, or
    /// Err(LssError::ConfigError) if the node responds with an error.
    pub async fn store_config(&mut self) -> Result<(), LssError> {
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        match self
            .send_and_receive(LssRequest::StoreConfiguration, Some(RESPONSE_TIMEOUT))
            .await
        {
            Some(LssResponse::StoreConfigurationAck { error, spec_error }) => {
                if error == 0 {
                    Ok(())
                } else {
                    Err(LssError::NodeStoreConfigError { error, spec_error })
                }
            }
            _ => Err(LssError::Timeout),
        }
    }

    /// Activates the configured baud rate

    pub async fn activate_baud_rate(&mut self, delay :u16) -> Result<(), LssError> {
        // No response expected; the baud rate will activate after the delay, at which
        // point we should also be on the same baud rate.
        self.send_and_receive(
            LssRequest::ActivateBitTiming { delay },
            None
        ).await;

        Ok(())
    }

    async fn send_and_receive(
        &mut self,
        msg: LssRequest,
        timeout: Option<Duration>,
    ) -> Option<LssResponse> {
        self.sender.send(msg.into()).await.ok()?;

        let wait_until = tokio::time::Instant::now() 
          + (if timeout.is_none() {Duration::ZERO} else {timeout.unwrap()});

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
