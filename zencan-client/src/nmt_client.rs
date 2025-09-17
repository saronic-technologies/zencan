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

    // Returns true if our receiver received a heartbeat
    fn has_heartbeat(&mut self) -> anyhow::Result<bool> {
        while let Some(data) = self.receiver.try_recv()? {
            match data.try_into() {
                Ok(ZencanMessage::Heartbeat(_)) => {
                    return Ok(true)
                },
                // No heartbeat, no sweat
                _ => break,
            }
        }

        Ok(false)
    }

    /// Check for and process any waiting heartbeat messages
    pub fn update_heartbeat(&mut self) -> anyhow::Result<()> {
        if true == self.has_heartbeat()? {
            // Update our last_seen variable
            self.last_seen = Some(Instant::now());
        }

        Ok(())
    }

    /// Returns true if we received a heartbeat within the allotted time
    /// Useful to check the presence of a device without clotting up a runloop
    pub async fn wait_for_heartbeat(&mut self, wait_time :Duration) -> anyhow::Result<bool> {
        let wait_until = tokio::time::Instant::now() + wait_time;
        loop {
            if true == self.has_heartbeat()? {
                return Ok(true)
            }

            if tokio::time::Instant::now() >= wait_until {
                return Ok(false)
            }

            // Yield time back to the executor
            tokio::task::yield_now().await;
        }
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
    fn build(&self) -> anyhow::Result<NmtClient>;
}
