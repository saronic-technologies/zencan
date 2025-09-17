//! nmt_client module for sending NMT commands to a specific node
use zencan_common::{messages::{NmtCommand, NmtCommandSpecifier}, traits::CanSendError, AsyncCanSender};

/// Client struct to represent NMT commands for a specific node
pub struct NmtClient {
    sender :Box<dyn AsyncCanSender>,
    node_id :u8
}

impl NmtClient {
    /// Create a new NmtClient
    pub fn new(
        sender :Box<dyn AsyncCanSender>,
        node_id :u8
    ) -> Self {
        Self {
            sender,
            node_id
        }
    }

    /// Send application reset command
    pub async fn nmt_reset_app(&mut self) -> Result<(), CanSendError> {
        self.send_nmt_cmd(NmtCommandSpecifier::ResetApp, self.node_id).await
    }

    /// Send communications reset command
    pub async fn nmt_reset_comms(&mut self) -> Result<(), CanSendError> {
        self.send_nmt_cmd(NmtCommandSpecifier::ResetComm, self.node_id)
            .await
    }

    /// Send start operation command
    pub async fn nmt_start(&mut self) -> Result<(), CanSendError> {
        self.send_nmt_cmd(NmtCommandSpecifier::Start, self.node_id).await
    }

    /// Send start operation command
    pub async fn nmt_stop(&mut self) -> Result<(), CanSendError> {
        self.send_nmt_cmd(NmtCommandSpecifier::Stop, self.node_id).await
    }

    /// Send preop command
    pub async fn nmt_preop(&mut self) -> Result<(), CanSendError> {
        self.send_nmt_cmd(NmtCommandSpecifier::EnterPreOp, self.node_id).await
    }

    async fn send_nmt_cmd(&mut self, cmd: NmtCommandSpecifier, node: u8) -> Result<(), CanSendError> {
        let message = NmtCommand { cs: cmd, node };
        self.sender.send(message.into()).await?;
        Ok(())
    }
}

pub trait INMTClientBuilder {
    fn set_node_id(&mut self, node_id :u8) -> &mut dyn INMTClientBuilder;
    fn build(&self) -> std::result::Result<NmtClient, Box<dyn std::error::Error>>;
}
