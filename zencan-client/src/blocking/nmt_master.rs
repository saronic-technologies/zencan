//! Simple interface for sending NMT commands to a bus
use std::time::Instant;

use zencan_common::{
    can::{CanMessage, CanReceiver, CanSender},
    protocol::{NmtCommand, NmtCommandSpecifier, NmtState, ZencanMessage},
};

use crate::nmt_master::{Node, Result, MAX_NODES};

#[derive(Debug)]
/// A blocking NMT master which allows monitoring the bus for heartbeats and commanding state
/// changes
#[allow(clippy::result_unit_err)]
pub struct NmtMaster<S, R> {
    sender: S,
    receiver: R,
    nodes: [Node; MAX_NODES],
}

#[allow(clippy::result_unit_err)]
impl<S: CanSender, R: CanReceiver> NmtMaster<S, R> {
    /// Create a new NmtMaster
    ///
    /// # Arguments
    /// - `sender`: An object which implements [`CanSender`] to be used for sending messages to
    ///   the bus
    /// - `receiver`: An object which implements [`CanReceiver`] to be used for receiving
    ///   messages from the bus
    ///
    /// When using socketcan, these can be created with [`crate::common::open_socketcan_blocking`].
    pub fn new(sender: S, receiver: R) -> Self {
        let nodes = [Node::default(); MAX_NODES];
        Self {
            sender,
            receiver,
            nodes,
        }
    }

    /// Receive and process all messages available from the message receiver
    pub fn process_rx(&mut self) {
        while let Some(msg) = self.receiver.try_recv() {
            self.handle_message(msg);
        }
    }

    fn handle_message(&mut self, msg: CanMessage) {
        // Attempt to convert the raw message into a zencanMessage. This may fail, e.g. if
        // non zencan messages are received, and that's OK; those are ignored.
        let open_msg: ZencanMessage = match msg.try_into() {
            Ok(m) => m,
            Err(_) => return,
        };

        if let ZencanMessage::Heartbeat(heartbeat) = open_msg {
            self.handle_heartbeat(heartbeat.node, heartbeat.state, heartbeat.toggle)
        }
    }

    /// Get a list of all nodes detected on the bus via heartbeat/reset messages
    pub fn get_nodes(&mut self) -> &[Node] {
        self.process_rx();

        // Find the first empty slot; this indicates the end of the list
        let n = self
            .nodes
            .iter()
            .position(|n| n.id == 0)
            .unwrap_or(MAX_NODES);
        &self.nodes[0..n]
    }

    fn handle_heartbeat(&mut self, node: u8, state: NmtState, toggle: bool) {
        // Find the node in the ordered list, inserting if needed.
        for i in 0..self.nodes.len() {
            let list_node = &mut self.nodes[i];
            if list_node.id == node {
                // Node already in list. Update it
                list_node.last_status = Instant::now();
                list_node.last_toggle = toggle;
                list_node.state = state;
                break;
            } else if list_node.id == 0 || list_node.id > node {
                // Found end of list or higher node - insert here
                // Shift all higher nodes
                for j in self.nodes.len() - 1..i {
                    self.nodes[j] = self.nodes[j - 1];
                }
                self.nodes[i] = Node {
                    id: node,
                    state,
                    last_status: Instant::now(),
                    last_toggle: toggle,
                };
                break;
            }
        }
    }

    /// Send application reset command
    ///
    /// # Arguments
    ///
    /// - `node`: The node ID to command, or 0 to broadcast to all nodes
    pub fn nmt_reset_app(&mut self, node: u8) -> Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::ResetApp, node)
    }

    /// Send communications reset command
    ///
    /// # Arguments
    ///
    /// - `node`: The node ID to command, or 0 to broadcast to all nodes
    pub fn nmt_reset_comms(&mut self, node: u8) -> Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::ResetComm, node)
    }

    /// Send start operation command
    ///
    /// # Arguments
    ///
    /// - `node`: The node ID to command, or 0 to broadcast to all nodes
    pub fn nmt_start(&mut self, node: u8) -> Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::Start, node)
    }

    /// Send start operation command
    ///
    /// # Arguments
    ///
    /// - `node`: The node ID to command, or 0 to broadcast to all nodes
    pub fn nmt_stop(&mut self, node: u8) -> Result<()> {
        self.send_nmt_cmd(NmtCommandSpecifier::Stop, node)
    }

    fn send_nmt_cmd(&mut self, cmd: NmtCommandSpecifier, node: u8) -> Result<()> {
        let message = NmtCommand { cs: cmd, node };
        self.sender.send(message.into()).map_err(|_| ())?;
        Ok(())
    }
}
