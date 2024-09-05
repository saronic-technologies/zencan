use canopen_common::{
    messages::{is_std_sdo_request, CanOpenMessage, Heartbeat, NmtCommandCmd, NmtState}, objects::ObjectDict, traits::{CanFdMessage, CanId, CanReceiver, CanSender}
};

use crate::{nmt::NmtSlave, sdo_server::SdoServer};

use defmt_or_log::warn;


pub struct Node<'a, S: CanSender> {
    node_id: u8,
    node_state: NmtState,
    sdo_server: Option<SdoServer>,
    sender: S,
    message_count: u32,
    od: ObjectDict<'a>
}

impl<'a, S: CanSender> Node<'a, S> {
    pub fn new(node_id: u8, sender: S, od: ObjectDict<'a>) -> Self {
        let message_count = 0;
        let sdo_server = None;
        let node_state = NmtState::Bootup;
        Self { node_id, node_state, sdo_server, sender, message_count, od }
    }

    pub fn handle_message(&mut self, msg: CanFdMessage) {
        // Some messages can only be handled after we have a node id
        if self.node_id != 0 {
            if is_std_sdo_request(msg.id(), self.node_id) {
                self.message_count += 1;
                if let Some(sdo_server) = &mut self.sdo_server {
                    // Convert message into an SDO request and
                    if let Ok(req) = msg.data().try_into() {
                        sdo_server.handle_request(&req, &self.od, &mut self.sender);
                    } else {
                        warn!("Failed to parse an SDO request message");
                    }
                }
            }
        }

        if let Ok(CanOpenMessage::NmtCommand(nmt)) = msg.try_into() {
            // We cannot respond to NMT commands if we do not have a valid node ID
            if self.node_id != 0 && nmt.node == 0 || nmt.node == self.node_id {
                self.handle_nmt_command(nmt.cmd);
                self.message_count += 1;
            }
        }
    }

    fn handle_nmt_command(&mut self, cmd: NmtCommandCmd) {
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
            },
            NmtCommandCmd::ResetComm => self.node_state = NmtState::PreOperational,
        }

        if prev_state != NmtState::PreOperational && self.node_state == NmtState::PreOperational {
            self.boot_up();
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

    pub fn node_id(&self) -> u8 {
        self.node_id
    }

    pub fn nmt_state(&self) -> NmtState {
        self.node_state
    }


    pub fn rx_message_count(&self) -> u32 {
        self.message_count
    }


    fn boot_up(&mut self) {
        self.sdo_server = Some(SdoServer::new(CanId::Std(
            0x580 + self.node_id as u16,
        )));
        self.sender.send(Heartbeat{
            node: self.node_id,
            toggle: false,
            state: self.node_state,
        }.into()).ok();
    }

    pub fn enter_preop(&mut self) {
        self.handle_nmt_command(NmtCommandCmd::EnterPreOp);
    }
}