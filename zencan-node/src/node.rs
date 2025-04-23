use zencan_common::{
    messages::{ZencanMessage, Heartbeat, NmtCommandCmd, NmtState},
    objects::ODEntry,
    traits::{CanFdMessage, CanId},
};

use crate::sdo_server::SdoServer;
use crate::node_mbox::NodeMboxRead;

use defmt_or_log::warn;


pub struct Node<'table, 'state> {
    node_id: Option<u8>,
    node_state: NmtState,
    sdo_server: SdoServer,
    message_count: u32,
    od: &'table [ODEntry<'table>],
    state: &'state dyn NodeMboxRead,
}

impl<'table, 'state> Node<'table, 'state> {
    pub fn new(state: &'state dyn NodeMboxRead, od: &'table [ODEntry<'table>]) -> Self {
        let message_count = 0;
        let sdo_server = SdoServer::new();
        let node_state = NmtState::Bootup;
        let node_id = None;
        Self {
            node_id,
            node_state,
            sdo_server,
            message_count,
            od,
            state,
        }
    }

    pub fn set_node_id(&mut self, node_id: u8) {
        self.node_id = Some(node_id);
        self.state.set_sdo_cob_id(Some(self.sdo_rx_cob_id()));
    }

    pub fn process(&mut self, send_cb: &mut dyn FnMut(CanFdMessage)) {
        // Some messages can only be handled after we have a node id
        if let Some(msg) = self.state.read_sdo_mbox() {
            self.message_count += 1;
            if let Ok(req) = msg.data().try_into() {
                if let Some(resp) = self.sdo_server.handle_request(&req, &self.od) {
                    send_cb(resp.to_can_message(self.sdo_tx_cob_id()));
                }
            } else {
                warn!("Failed to parse an SDO request message");
            }
        }

        if let Some(msg) = self.state.read_nmt_mbox() {
            if let Ok(ZencanMessage::NmtCommand(cmd)) = msg.try_into() {
                self.message_count += 1;
                // We cannot respond to NMT commands if we do not have a valid node ID
                if let Some(node_id) = self.node_id {
                    if cmd.node == 0 || cmd.node == node_id {
                        self.handle_nmt_command(cmd.cmd, send_cb);
                    }
                }
            }
        }
    }

    fn handle_nmt_command(&mut self, cmd: NmtCommandCmd, sender: &mut dyn FnMut(CanFdMessage)) {
        let prev_state = self.node_state;

        match cmd {
            NmtCommandCmd::Start => self.node_state = NmtState::Operational,
            NmtCommandCmd::Stop => self.node_state = NmtState::Stopped,
            NmtCommandCmd::EnterPreOp => self.node_state = NmtState::PreOperational,
            NmtCommandCmd::ResetApp => {
                // if let Some(cb) = self.app_reset_callback.as_mut() {
                //     cb();
                // }
                self.node_state = NmtState::PreOperational;
            }
            NmtCommandCmd::ResetComm => self.node_state = NmtState::PreOperational,
        }

        if prev_state != NmtState::PreOperational && self.node_state == NmtState::PreOperational {
            self.boot_up(sender);
        }
        // if self.node_id.is_some() && self.node_state == NmtState::Bootup {
        //     if let Some(cb) = self.app_reset_callback.as_mut() {
        //         cb();
        //     }
        //     self.node_state = NmtState::PreOperational;
        // }

        // if self.node_state != prev_state {
        //     if let Some(cb) = self.nmt_state_callback.as_mut() {
        //         cb(self.node_state);
        //     }
        // }
    }

    pub fn node_id(&self) -> Option<u8> {
        self.node_id
    }

    pub fn nmt_state(&self) -> NmtState {
        self.node_state
    }

    pub fn rx_message_count(&self) -> u32 {
        self.message_count
    }

    pub fn sdo_tx_cob_id(&self) -> CanId {
        let node_id = self.node_id.unwrap_or(0);
        CanId::Std(0x580 + node_id as u16)
    }

    pub fn sdo_rx_cob_id(&self) -> CanId {
        let node_id = self.node_id.unwrap_or(0);
        CanId::Std(0x600 + node_id as u16)
    }

    fn boot_up(&mut self, sender: &mut dyn FnMut(CanFdMessage)) {

        //self.sdo_server = Some(SdoServer::new());
        if let Some(node_id) = self.node_id {
            sender(
                Heartbeat {
                    node: node_id,
                    toggle: false,
                    state: self.node_state,
                }
                .into(),
            );
        }
    }

    pub fn enter_preop(&mut self, sender: &mut dyn FnMut(CanFdMessage)) {
        self.handle_nmt_command(NmtCommandCmd::EnterPreOp, sender);
    }
}

// pub struct PdoServer<const N_RX {

// }
