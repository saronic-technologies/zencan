//! Implements mailbox for receiving CAN messages
use defmt_or_log::warn;
use zencan_common::{
    messages::{CanId, CanMessage},
    sdo::SdoRequest,
    AtomicCell,
};

use crate::{lss_slave::LssReceiver, node_state::Pdo};

/// A data structure to be shared between a receiving thread (e.g. a CAN controller IRQ) and the
/// [`Node`](crate::Node) object.
///
/// Incoming messages should be passed to [NodeMbox::store_message].
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
    /// Create a new NodeMbox
    ///
    /// # Args
    ///
    /// - `rx_pdos`: A slice of Pdo objects for all of the receive PDOs
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

    pub(crate) fn num_rx_pdos(&self) -> usize {
        self.rx_pdos.len()
    }

    pub(crate) fn set_sdo_cob_id(&self, cob_id: Option<CanId>) {
        self.sdo_cob_id.store(cob_id);
    }

    pub(crate) fn read_sdo_mbox(&self) -> Option<SdoRequest> {
        self.sdo_mbox.take()
    }

    pub(crate) fn read_nmt_mbox(&self) -> Option<CanMessage> {
        self.nmt_mbox.take()
    }

    pub(crate) fn lss_receiver(&self) -> &LssReceiver {
        &self.lss_receiver
    }

    pub(crate) fn read_sync_flag(&self) -> bool {
        self.sync_flag.take()
    }

    /// Store a received CAN message
    pub fn store_message(&self, msg: CanMessage) -> Result<(), CanMessage> {
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
