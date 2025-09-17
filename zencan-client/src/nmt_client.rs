//! nmt_client module for sending NMT commands to a specific node
use std::time::{Duration, Instant};

use zencan_common::{messages::{NmtCommand, NmtCommandSpecifier, ZencanMessage}, AsyncCanReceiver, AsyncCanSender};

/// Client struct to represent NMT commands for a specific node
pub struct NmtClient {
    sender :Box<dyn AsyncCanSender>,
    receiver :Box<dyn AsyncCanReceiver>,
    node_id :u8,
    last_seen: Option<Instant>,
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
            last_seen: None
        }
    }

    /// Get the time elapsed since the last heartbeat was received
    pub fn time_between_heartbeats(&self) -> Option<Duration> {
        if let Some(last_seen) = self.last_seen {
            Some(Instant::now().duration_since(last_seen))
        } else {
            None
        }
    }

    /// Check for and process any waiting heartbeat messages
    pub async fn update_heartbeat(&mut self) -> anyhow::Result<()> {
        let received_data = self.receiver.try_recv()?;

        if let Some(data) = received_data {
            match data.try_into() {
                Ok(ZencanMessage::Heartbeat(_)) => {
                    // Update our last_seen variable
                    self.last_seen = Some(Instant::now());
                },
                // No heartbeat, no sweat
                _ => return Ok(())
            }
        }
        Ok(())
    }

    /// Send application reset command
    pub async fn nmt_reset_app(&mut self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::ResetApp, self.node_id).await
    }

    /// Send communications reset command
    pub async fn nmt_reset_comms(&mut self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::ResetComm, self.node_id)
            .await
    }

    /// Send start operation command
    pub async fn nmt_start(&mut self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::Start, self.node_id).await
    }

    /// Send start operation command
    pub async fn nmt_stop(&mut self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::Stop, self.node_id).await
    }

    /// Send preop command
    pub async fn nmt_preop(&mut self) -> anyhow::Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::EnterPreOp, self.node_id).await
    }

    async fn send_nmt_cmd(&mut self, cmd: NmtCommandSpecifier, node: u8) -> anyhow::Result<()> {
        let message = NmtCommand { cs: cmd, node };
        self.sender.send(message.into()).await?;
        Ok(())
    }
}

/// Builder trait for creating NMT clients
pub trait INMTClientBuilder {
    /// Set the node ID for the NMT client to be built
    fn set_node_id(&mut self, node_id :u8) -> &mut dyn INMTClientBuilder;
    /// Build the NMT client with the configured node ID
    fn build(&self) -> std::result::Result<NmtClient, Box<dyn std::error::Error>>;
}
