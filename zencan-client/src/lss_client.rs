// We use the LSS client to bind an LSS identity to an object that we
// can use to set a baud rate or a node ID

use std::time::Duration;

use tokio::time::timeout_at;
use zencan_common::{lss::{LssIdentity, LssRequest, LssResponse}, AsyncCanReceiver, AsyncCanSender, NodeId};

use crate::LssError;

/// LSS client for configuring devices with specific LSS identities
pub struct LssClient {
    // While our sender and receiver by themselves are "Sync", our protocol
    // is not, as it requires exclusive access to them.  This guard guarantees that.
    io_guard :tokio::sync::Mutex<(Box<dyn AsyncCanSender>, Box<dyn AsyncCanReceiver>)>,
    // LSS operates using an identity instead of a node ID; the identity
    // we can either get from a "fast_scan" (no node ID) or a bus scan,
    // which just loops over every possible node ID and sees which ones respond
    identity :LssIdentity
}

impl LssClient {
    /// Create a new LSS client bound to a specific device identity
    pub fn new(
        sender :Box<dyn AsyncCanSender>,
        receiver :Box<dyn AsyncCanReceiver>,
        identity :LssIdentity
    ) -> Self {
        Self {
            io_guard :(sender, receiver).into(),
            identity
        }
    }

    /// Send a sequence of messages to put a single node into configuration mode based on its identity
    pub async fn enter_configuration_mode(
        &self,
    ) -> anyhow::Result<()> {
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        // Send global mode to put all nodes into waiting state. No response expected.
        self.send_and_receive(LssRequest::SwitchModeGlobal { mode: 0 }, None)
            .await?;

        let vendor_id = self.identity.vendor_id;
        // Now send the identity messages. If a LSS slave node recognizes its identity, it will respond
        // to the serial setting message with a SwitchStateResponse message
        self.send_and_receive(LssRequest::SwitchStateVendor { vendor_id }, None)
            .await?;

        let product_code = self.identity.product_code;
        self.send_and_receive(
            LssRequest::SwitchStateProduct { product_code },
            None,
        )
        .await?;
        
        let revision = self.identity.revision;
        self.send_and_receive(LssRequest::SwitchStateRevision { revision }, None)
            .await?;

        let serial = self.identity.serial;
        match self
            .send_and_receive(LssRequest::SwitchStateSerial { serial }, Some(RESPONSE_TIMEOUT))
            .await?
        {
            // If we have a response, we're all good!
            Some(LssResponse::SwitchStateResponse) => Ok(()),
            // If we get anything else, including nothing
            _ => Err(LssError::Timeout.into())
        }
    }

    /// Places the client's node into waiting state
    /// This method technically places all nodes into the waiting state, as there
    /// doesn't seem to be a way to do it per node
    pub async fn exit_configuration_mode(
        &mut self
    ) -> anyhow::Result<()> {
        // Send global mode to put all nodes into waiting state. No response expected.
        self.send_and_receive(LssRequest::SwitchModeGlobal { mode: 0 }, None)
            .await?;
        Ok(())
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
    pub async fn set_baud_rate(&mut self, table: u8, index: u8) -> anyhow::Result<()> {
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        match self
            .send_and_receive(
                LssRequest::ConfigureBitTiming { table, index },
                Some(RESPONSE_TIMEOUT),
            )
            .await?
        {
            Some(LssResponse::ConfigureBitTimingAck { error, spec_error }) => {
                if error == 0 {
                    Ok(())
                } else {
                    Err(LssError::BitTimingConfigError { error, spec_error }.into())
                }
            }
            _ => Err(LssError::Timeout.into()),
        }
    }

    /// Send a command to set the node ID on the LSS slave current in configuration mode
    ///
    /// The node must have been put into configuration mode already.
    ///
    /// Returns Err(LssError::Timeout) if the node does not respond to the command, or
    /// Err(LssError::ConfigError) if the node responds with an error.
    pub async fn set_node_id(&self, node_id: u8) -> anyhow::Result<()> {
        let node_id_object = NodeId::new(node_id).map_err(|_| LssError::InvalidNodeIdError)?;
        
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        match self
            .send_and_receive(
                LssRequest::ConfigureNodeId {
                    node_id: node_id_object.into(),
                },
                Some(RESPONSE_TIMEOUT),
            )
            .await?
        {
            // If we get the correct ACK, then we can proceed
            Some(LssResponse::ConfigureNodeIdAck { error, spec_error }) => {
                if error == 0 {
                    Ok(())
                } else {
                    Err(LssError::NodeIdConfigError { error, spec_error }.into())
                }
            }
            _ => Err(LssError::Timeout.into()),
        }
    }

    /// Send command to store configuration
    ///
    /// The node must have been put into configuration mode already.
    ///
    /// Returns Err(LssError::Timeout) if the node does not respond to the command, or
    /// Err(LssError::ConfigError) if the node responds with an error.
    pub async fn store_config(&mut self) -> anyhow::Result<()> {
        const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
        match self
            .send_and_receive(LssRequest::StoreConfiguration, Some(RESPONSE_TIMEOUT))
            .await?
        {
            Some(LssResponse::StoreConfigurationAck { error, spec_error }) => {
                if error == 0 {
                    Ok(())
                } else {
                    Err(LssError::NodeStoreConfigError { error, spec_error }.into())
                }
            }
            _ => Err(LssError::Timeout.into()),
        }
    }

    /// Activates the configured baud rate

    pub async fn activate_baud_rate(&self, delay :u16) -> anyhow::Result<()> {
        // No response expected; the baud rate will activate after the delay, at which
        // point we should also be on the same baud rate.
        self.send_and_receive(
            LssRequest::ActivateBitTiming { delay },
            None
        ).await?;

        Ok(())
    }

    async fn send_and_receive(
        &self,
        msg: LssRequest,
        timeout: Option<Duration>,
    ) -> anyhow::Result<Option<LssResponse>> {
        let io = self.io_guard.lock().await;
        let sender = &io.0;
        let receiver = &io.1;

        sender.send(msg.into()).await?;

        let wait_until = tokio::time::Instant::now() 
          + (if timeout.is_none() {Duration::ZERO} else {timeout.unwrap()});

        loop {
            match timeout_at(wait_until, receiver.recv()).await {
                // Got a message
                Ok(Ok(msg)) => {
                    return Ok(Some(msg.try_into()?))
                }
                Ok(Err(e)) => {
                    return Err(e.into())
                }
                // Timeout waiting
                Err(_) => return Ok(None),
            }
        }
    }
}

/// Builder trait for creating LSS clients
pub trait LSSClientBuilder :Send + Sync {
    /// Set the LSS identity for the client to be built
    fn set_identity(self, identity: LssIdentity) -> Self;
    /// Build the LSS client with the configured identity
    fn build(self) -> anyhow::Result<LssClient>;
}
