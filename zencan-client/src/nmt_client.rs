//! nmt_client module for sending NMT commands to a specific node
use std::time::{Duration};

use zencan_common::{messages::{NmtCommand, NmtCommandSpecifier, ZencanMessage}, AsyncCanReceiver, AsyncCanSender};

/// Client struct to represent NMT commands for a specific node
pub struct NmtClient {
    sender :Box<dyn AsyncCanSender>,
    receiver :Box<dyn AsyncCanReceiver>,
    node_id :u8,
}

impl NmtClient {
    /// Create a new NmtClient
    pub fn new(
        sender :Box<dyn AsyncCanSender>,
        receiver :Box<dyn AsyncCanReceiver>,
        node_id :u8
    ) -> Self {
        Self {
            sender,
            receiver,
            node_id,
        }
    }

    /// Returns true if we received a heartbeat within the allotted time
    /// Useful to check the presence of a device without clotting up a runloop
    pub async fn wait_for_heartbeat(&self, wait_time :Duration) -> anyhow::Result<bool> {
        // let wait_until = tokio::time::Instant::now() + wait_time;
        loop {
            tokio::select! {
                data = self.receiver.recv() => {
                    match data?.try_into() {
                        Ok(ZencanMessage::Heartbeat(_)) => {
                            return Ok(true)
                        }
                        Ok(_) => {
                            // Any other message is fine, just continue
                        }
                        Err(error_code) => return Err(error_code.into())
                    }
                }
                _ = tokio::time::sleep(wait_time) => {
                    // We didn't receive a heartbeat in the requested time,
                    // so return false.
                    return Ok(false);
                }
            }
        }
    }

    /// Send application reset command
    pub async fn nmt_reset_app(&self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::ResetApp, self.node_id).await
    }

    /// Send communications reset command
    pub async fn nmt_reset_comms(&self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::ResetComm, self.node_id)
            .await
    }

    /// Send start operation command
    pub async fn nmt_start(&self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::Start, self.node_id).await
    }

    /// Send start operation command
    pub async fn nmt_stop(&self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::Stop, self.node_id).await
    }

    /// Send preop command
    pub async fn nmt_preop(&self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::EnterPreOp, self.node_id).await
    }

    async fn send_nmt_cmd(&self, cmd: NmtCommandSpecifier, node: u8) -> anyhow::Result<()> {
        let message = NmtCommand { cs: cmd, node };
        self.sender.send(message.into()).await?;
        Ok(())
    }
}

/// Builder trait for creating NMT clients
pub trait NMTClientBuilder :Send + Sync {
    /// Set the node ID for the NMT client to be built
    fn set_node_id(self, node_id :u8) -> Self;
    /// Build the NMT client with the configured node ID
    fn build(self) -> anyhow::Result<NmtClient>;
}
