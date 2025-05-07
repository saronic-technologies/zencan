use crossbeam::atomic::AtomicCell;
use defmt_or_log::warn;
use zencan_common::{
    messages::{CanId, CanMessage},
    sdo::SdoRequest,
};

use crate::{lss_slave::LssReceiver, node_state::Pdo};

/// A trait for the (typically auto-generated) shared data struct to implement for reading data from mailboxes
pub trait NodeMboxRead: Sync + Send {
    /// Get the number of RX PDO mailboxes available
    fn num_rx_pdos(&self) -> usize;
    /// Set the receive COB ID for the SDO server
    /// Read a pending message for the main SDO mailbox. Will return None if there is no message.
    fn set_sdo_cob_id(&self, cob_id: Option<CanId>);
    /// Read a pending message for the main SDO mailbox. Will return None if there is no message.
    fn read_sdo_mbox(&self) -> Option<SdoRequest>;
    /// Read a pending NMT command
    fn read_nmt_mbox(&self) -> Option<CanMessage>;
    /// Borrow the LSS receiver object
    fn lss_receiver(&self) -> &LssReceiver;
    /// Read the sync flag
    ///
    /// The flag is set when a SYNC message is received, and cleared when this function is called.
    fn read_sync_flag(&self) -> bool;
}

/// A trait for the (typically auto-generated) shared data struct to implement for storing recieved messages
pub trait NodeMboxWrite: Sync + Send {
    /// Attempt to store a message to the node
    ///
    /// If the message ID is a valid CanOpen message handled by the node, returns `OK(())`,
    /// otherwise the unhandled message is returned wrapped in an Err.
    fn store_message(&self, msg: CanMessage) -> Result<(), CanMessage>;
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
    sdo_mbox: AtomicCell<Option<SdoRequest>>,
    nmt_mbox: AtomicCell<Option<CanMessage>>,
    lss_receiver: LssReceiver,
    sync_flag: AtomicCell<bool>,
    notify_cb: AtomicCell<Option<&'static (dyn Fn() + Sync)>>,
}

impl NodeMbox {
    pub const fn new(rx_pdos: &'static [Pdo]) -> Self {
        let sdo_cob_id = AtomicCell::new(None);
        let sdo_mbox = AtomicCell::new(None);
        let nmt_mbox = AtomicCell::new(None);
        let lss_receiver = LssReceiver::new();
        let sync_flag = AtomicCell::new(false);
        let notify_cb = AtomicCell::new(None);
        Self {
            rx_pdos,
            sdo_cob_id,
            sdo_mbox,
            nmt_mbox,
            lss_receiver,
            sync_flag,
            notify_cb,
        }
    }

    /// Set a callback to be called when a message is received which should trigger a call to the
    /// node process method
    ///
    /// It must be static. Usually this will be a static fn, but in some circumstances, it may be
    /// desirable to use Box::leak to pass a heap allocated closure instead.
    pub fn set_process_notify_callback(&self, callback: &'static (dyn Fn() + Sync)) {
        self.notify_cb.store(Some(callback));
    }

    fn notify(&self) {
        if let Some(notify_cb) = self.notify_cb.load() {
            notify_cb();
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

    fn read_sdo_mbox(&self) -> Option<SdoRequest> {
        self.sdo_mbox.take()
    }

    fn read_nmt_mbox(&self) -> Option<CanMessage> {
        self.nmt_mbox.take()
    }

    fn lss_receiver(&self) -> &LssReceiver {
        &self.lss_receiver
    }

    fn read_sync_flag(&self) -> bool {
        self.sync_flag.take()
    }
}

impl NodeMboxWrite for NodeMbox {
    fn store_message(&self, msg: CanMessage) -> Result<(), CanMessage> {
        let id = msg.id();
        if id == zencan_common::messages::NMT_CMD_ID {
            self.nmt_mbox.store(Some(msg));
            self.notify();
            return Ok(());
        }

        if id == zencan_common::messages::SYNC_ID {
            self.sync_flag.store(true);
            self.notify();
            return Ok(());
        }

        if id == zencan_common::messages::LSS_REQ_ID {
            if let Ok(lss_req) = msg.data().try_into() {
                if self.lss_receiver.handle_req(lss_req) {
                    self.notify();
                }
            } else {
                warn!("Invalid LSS request");
                return Err(msg);
            }
            return Ok(());
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
                if let Ok(sdo_req) = msg.data().try_into() {
                    self.sdo_mbox.store(Some(sdo_req));
                    return Ok(());
                } else {
                    warn!("Invalid SDO request");
                    return Err(msg);
                }
            }
        }

        Err(msg)
    }
}
