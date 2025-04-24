use crossbeam::atomic::AtomicCell;
use zencan_common::traits::{CanFdMessage, CanId};

use crate::node_state::Pdo;

/// A trait for the (typically auto-generated) shared data struct to implement for reading data from mailboxes
pub trait NodeMboxRead : Sync + Send {
    /// Get the number of RX PDO mailboxes available
    fn num_rx_pdos(&self) -> usize;
    /// Set the receive COB ID for the SDO server
    /// Read a pending message for the main SDO mailbox. Will return None if there is no message.
    fn set_sdo_cob_id(&self, cob_id: Option<CanId>);
    fn read_sdo_mbox(&self) -> Option<CanFdMessage>;
    /// Read a pending NMT command
    fn read_nmt_mbox(&self) -> Option<CanFdMessage>;
}

/// A trait for the (typically auto-generated) shared data struct to implement for storing recieved messages
pub trait NodeMboxWrite : Sync + Send {
    /// Attempt to store a message to the node
    ///
    /// If the message ID is a valid CanOpen message handled by the node, returns `OK(())`,
    /// otherwise the unhandled message is returned wrapped in an Err.
    fn store_message(&self, msg: CanFdMessage) -> Result<(), CanFdMessage>;
}

/// A mailbox for communication of RX PDO messages between a message receiving thread and the main thread
#[derive(Debug, Default)]
pub struct RxPdoMbox {
    /// The current COB ID for this PDO
    pub cob_id: AtomicCell<Option<CanId>>,
    /// Holds any pending message for this PDO
    pub mbox: AtomicCell<Option<[u8; 8]>>,
}

impl RxPdoMbox {
    pub const fn new() -> Self {
        Self {
            cob_id: AtomicCell::new(None),
            mbox: AtomicCell::new(None),
        }
    }
}

pub struct NodeMbox {
    rx_pdos: &'static [Pdo],
    sdo_cob_id: AtomicCell<Option<CanId>>,
    sdo_mbox: AtomicCell<Option<CanFdMessage>>,
    nmt_mbox: AtomicCell<Option<CanFdMessage>>,
}

impl NodeMbox {
    pub const fn new(rx_pdos: &'static [Pdo]) -> Self {
        let sdo_cob_id = AtomicCell::new(None);
        let sdo_mbox = AtomicCell::new(None);
        let nmt_mbox = AtomicCell::new(None);
        Self {
            rx_pdos,
            sdo_cob_id,
            sdo_mbox,
            nmt_mbox,
        }
    }
}

impl NodeMboxRead for NodeMbox {
    fn num_rx_pdos(&self) -> usize {
        self.rx_pdos.len()
    }

    fn set_sdo_cob_id(&self, cob_id: Option<CanId>) {
        self.sdo_cob_id.store(cob_id);
    }
    /// Read a pending message for the main SDO mailbox. Will return None if there is no message.
    fn read_sdo_mbox(&self) -> Option<CanFdMessage> {
        self.sdo_mbox.take()
    }

    fn read_nmt_mbox(&self) -> Option<CanFdMessage> {
        self.nmt_mbox.take()
    }
}

impl NodeMboxWrite for NodeMbox {
    fn store_message(&self, msg: CanFdMessage) -> Result<(), CanFdMessage> {
        let id = msg.id();
        if id == zencan_common::messages::NMT_CMD_ID {
            self.nmt_mbox.store(Some(msg));
            return Ok(())
        }

        for rpdo in self.rx_pdos {
            if !rpdo.valid.load() {
                continue;
            }
            if id == rpdo.cob_id.load() {
                let mut data = [0u8; 8];
                data[0..msg.data().len()].copy_from_slice(msg.data());
                rpdo.buffered_value.store(Some(data));
                return Ok(());
            }
        }

        if let Some(cob_id) = self.sdo_cob_id.load() {
            if id == cob_id {
                self.sdo_mbox.store(Some(msg));
                return Ok(());
            }
        }

        Err(msg)

    }
}
