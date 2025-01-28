use std::time::Instant;

use zencan_common::{
    messages::{zencanMessage, NmtCommand, NmtCommandCmd, NmtState},
    traits::{CanFdMessage, CanReceiver, CanSender},
};

type Result<T> = std::result::Result<T, ()>;

#[derive(Copy, Clone, Debug)]
pub struct Node {
    pub id: u8,
    pub state: NmtState,
    pub last_status: Instant,
    last_toggle: bool,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            id: 0,
            state: NmtState::Bootup,
            last_status: Instant::now(),
            last_toggle: true,
        }
    }
}

const MAX_NODES: usize = 127;

pub struct Master<S, R> {
    sender: S,
    receiver: R,
    nodes: [Node; MAX_NODES],
}

impl<S: CanSender, R: CanReceiver> Master<S, R> {
    pub fn new(sender: S, receiver: R) -> Self {
        let nodes = [Node::default(); MAX_NODES];
        Self {
            sender,
            receiver,
            nodes,
        }
    }

    pub fn process_rx(&mut self) {
        while let Some(msg) = self.receiver.try_recv() {
            self.handle_message(msg);
        }
    }

    pub fn handle_message(&mut self, msg: CanFdMessage) {
        // Attempt to convert the raw message into a zencanMessage. This may fail, e.g. if
        // non zencan messages are received, and that's OK; those are ignored.
        let open_msg: zencanMessage = match msg.try_into() {
            Ok(m) => m,
            Err(_) => return,
        };

        match open_msg {
            zencanMessage::Heartbeat(heartbeat) => {
                self.handle_heartbeat(heartbeat.node, heartbeat.state, heartbeat.toggle)
            }
            _ => (),
        }
    }

    /// Get a list of all nodes detected on the bus via heartbeat/reset messages
    pub fn get_nodes(&mut self) -> &[Node] {
        self.process_rx();

        // Find the first empty slot; this indicates the end of the list
        let n = self.nodes.iter().position(|n| n.id == 0).unwrap_or(MAX_NODES);
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
    /// node - The node ID to command, or 0 to broadcast to all nodes
    pub fn nmt_reset_app(&mut self, node: u8) -> Result<()> {
        self.send_nmt_cmd(NmtCommandCmd::ResetApp, node)
    }

    /// Send communications reset command
    ///
    /// node - The node ID to command, or 0 to broadcast to all nodes
    pub fn nmt_reset_comms(&mut self, node: u8) -> Result<()> {
        self.send_nmt_cmd(NmtCommandCmd::ResetComm, node)
    }

    /// Send start operation command
    ///
    /// node - The node ID to command, or 0 to broadcast to all nodes
    pub fn nmt_start(&mut self, node: u8) -> Result<()> {
        self.send_nmt_cmd(NmtCommandCmd::Start, node)
    }

    /// Send start operation command
    ///
    /// node - The node ID to command, or 0 to broadcast to all nodes
    pub fn nmt_stop(&mut self, node: u8) -> Result<()> {
        self.send_nmt_cmd(NmtCommandCmd::Stop, node)
    }

    fn send_nmt_cmd(&mut self, cmd: NmtCommandCmd, node: u8) -> Result<()> {
        let message = NmtCommand { cmd, node };
        self.sender.send(message.into()).map_err(|_| ())?;
        Ok(())
    }
}
