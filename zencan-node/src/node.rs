use crossbeam::atomic::AtomicCell;
use zencan_common::{
    messages::{is_std_sdo_request, zencanMessage, Heartbeat, NmtCommandCmd, NmtState},
    objects::ODEntry,
    traits::{CanFdMessage, CanId, CanReceiver, CanSender},
};

use crate::{nmt::NmtSlave, sdo_server::SdoServer};

use defmt_or_log::warn;

/// A trait for the (typically auto-generated) shared data struct to implement for reading data from mailboxes
pub trait NodeStateAccess {
    /// Get the number of RX PDO mailboxes available
    fn num_rx_pdos(&self) -> usize;
    /// Read a pending message for one of the RX PDOs. Will panic if idx is out of bounds. Will
    /// return None if there is no message.
    fn read_rx_pdo(&self, idx: usize) -> Option<CanFdMessage>;
    /// Update the COB ID assignment for the given RX PDO mailbox
    fn set_rx_pdo_cob_id(&self, idx: usize, cob_id: Option<CanId>);
    /// Read a pending message for the main SDO mailbox. Will return None if there is no message.
    fn read_sdo_mbox(&self) -> Option<CanFdMessage>;
}

/// A trait for the (typically auto-generated) shared data struct to implement for storing recieved messages
pub trait MessageReceiver {
    /// Attempt to store a message to the node
    ///
    /// If the message ID is a valid CanOpen message handled by the node, returns `OK(())`,
    /// otherwise the unhandled message is returned wrapped in an Err.
    fn store_message(msg: CanFdMessage) -> Result<(), CanFdMessage>;
}

/// A mailbox for communication of RX PDO messages between a message receiving thread and the main thread
#[derive(Debug, Default)]
pub struct RxPdo {
    /// The current COB ID for this PDO
    pub cob_id: AtomicCell<Option<CanId>>,
    /// Holds any pending message for this PDO
    pub mbox: AtomicCell<Option<CanFdMessage>>,
    // TODO: Other PDO config can be store here (e.g. mirrored from PDO config objects)
}

impl RxPdo {
    pub const fn new() -> Self {
        Self {
            cob_id: AtomicCell::new(None),
            mbox: AtomicCell::new(None),
        }
    }
}

pub struct TxPdo {
    pub cob_id: AtomicCell<CanId>,
}

pub struct NodeState<const N_RPDO: usize> {
    rx_pdos: [RxPdo; N_RPDO],
    sdo_cob_id: Option<CanId>,
    sdo_mbox: AtomicCell<Option<CanFdMessage>>,
}

impl<const N_RPDO: usize> NodeState<N_RPDO> {
    pub const fn new() -> Self {
        let rx_pdos = [const { RxPdo::new() }; N_RPDO];
        let sdo_cob_id = None;
        let sdo_mbox = AtomicCell::new(None);
        Self {
            rx_pdos,
            sdo_cob_id,
            sdo_mbox,
        }
    }
}

impl<const N_RPDO: usize> NodeStateAccess for NodeState<N_RPDO> {
    fn set_rx_pdo_cob_id(&self, idx: usize, cob_id: Option<CanId>) {
        self.rx_pdos[idx].cob_id.store(cob_id);
    }

    fn num_rx_pdos(&self) -> usize {
        self.rx_pdos.len()
    }

    fn read_rx_pdo(&self, idx: usize) -> Option<CanFdMessage> {
        self.rx_pdos[idx].mbox.take()
    }

    /// Read a pending message for the main SDO mailbox. Will return None if there is no message.
    fn read_sdo_mbox(&self) -> Option<CanFdMessage> {
        self.sdo_mbox.take()
    }
}

pub struct Node<'table, 'state> {
    node_id: u8,
    node_state: NmtState,
    sdo_server: Option<SdoServer>,
    message_count: u32,
    od: &'table [ODEntry<'table>],
    state: &'state dyn NodeStateAccess,
}

impl<'table, 'state> Node<'table, 'state> {
    pub fn new(
        node_id: u8,
        state: &'state dyn NodeStateAccess,
        od: &'table [ODEntry<'table>],
    ) -> Self {
        let message_count = 0;
        let sdo_server = None;
        let node_state = NmtState::Bootup;
        Self {
            node_id,
            node_state,
            sdo_server,
            message_count,
            od,
            state,
        }
    }

    pub fn handle_message(&mut self, msg: CanFdMessage, send_cb: &mut dyn FnMut(CanFdMessage)) {
        // Some messages can only be handled after we have a node id
        if self.node_id != 0 {
            if is_std_sdo_request(msg.id(), self.node_id) {
                self.message_count += 1;
                if let Some(sdo_server) = &mut self.sdo_server {
                    // Convert message into an SDO request and
                    if let Ok(req) = msg.data().try_into() {
                        if let Some(resp) = sdo_server.handle_request(&req, &self.od) {
                            send_cb(resp.to_can_message(self.sdo_tx_cob_id()));
                        }
                    } else {
                        warn!("Failed to parse an SDO request message");
                    }
                }
            }

            if msg.id() == zencan_common::messages::SYNC_ID {}
        }

        if let Ok(zencanMessage::NmtCommand(nmt)) = msg.try_into() {
            // We cannot respond to NMT commands if we do not have a valid node ID
            if self.node_id != 0 && nmt.node == 0 || nmt.node == self.node_id {
                self.handle_nmt_command(nmt.cmd, send_cb);
                self.message_count += 1;
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

    pub fn node_id(&self) -> u8 {
        self.node_id
    }

    pub fn nmt_state(&self) -> NmtState {
        self.node_state
    }

    pub fn rx_message_count(&self) -> u32 {
        self.message_count
    }

    pub fn sdo_tx_cob_id(&self) -> CanId {
        CanId::Std(0x580 + self.node_id as u16)
    }

    fn boot_up(&mut self, sender: &mut dyn FnMut(CanFdMessage)) {
        self.sdo_server = Some(SdoServer::new());
        sender(
            Heartbeat {
                node: self.node_id,
                toggle: false,
                state: self.node_state,
            }
            .into(),
        );
    }

    pub fn enter_preop(&mut self, sender: &mut dyn FnMut(CanFdMessage)) {
        self.handle_nmt_command(NmtCommandCmd::EnterPreOp, sender);
    }
}

// pub struct PdoServer<const N_RX {

// }
