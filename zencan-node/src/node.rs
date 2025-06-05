//! Implements the core Node object
//!

use zencan_common::{
    lss::LssIdentity,
    messages::{CanId, CanMessage, Heartbeat, NmtCommandSpecifier, NmtState, ZencanMessage, LSS_RESP_ID},
    object_ids,
    objects::{find_object, ODEntry, ObjectData, ObjectRawAccess},
    NodeId,
};

use crate::{lss_slave::LssSlave, node_mbox::NodeMbox, storage::StoreObjectsCallback};
use crate::{node_state::NodeStateAccess, sdo_server::SdoServer};

use defmt_or_log::{debug, info};

type StoreNodeConfigCallback = dyn Fn(&NodeId) + Sync;

#[derive(Default)]
struct Callbacks {
    store_node_config: Option<&'static StoreNodeConfigCallback>,
}

/// The main object representing a node
pub struct Node {
    node_id: NodeId,
    nmt_state: NmtState,
    sdo_server: SdoServer,
    lss_slave: LssSlave,
    message_count: u32,
    od: &'static [ODEntry<'static>],
    mbox: &'static NodeMbox,
    state: &'static dyn NodeStateAccess,
    reassigned_node_id: Option<NodeId>,
    callbacks: Callbacks,
    next_heartbeat_time_us: u64,
    heartbeat_period_ms: u16,
}

fn read_identity(od: &[ODEntry]) -> Option<LssIdentity> {
    let obj = find_object(od, object_ids::IDENTITY)?;
    let vendor_id = obj.read_u32(1).ok()?;
    let product_code = obj.read_u32(2).ok()?;
    let revision = obj.read_u32(3).ok()?;
    let serial = obj.read_u32(4).ok()?;
    Some(LssIdentity {
        vendor_id,
        product_code,
        revision,
        serial,
    })
}

fn read_heartbeat_period(od: &[ODEntry]) -> Option<u16> {
    let obj = find_object(od, object_ids::HEARTBEAT_PRODUCER_TIME)?;
    obj.read_u16(0).ok()
}

impl Node {
    /// Create a new Node
    ///
    /// # Arguments
    ///
    /// - `node_id`: The initial ID for the node. It may be assigned an ID at boot time by the
    ///   application, or it may be left as [NodeId::Unconfigured].
    /// - `mbox`: The NodeMbox object created by code generator
    /// - `state`: The NodeState object created by code generator
    /// - `od`: The Object Dictionary, created by code generator
    pub fn new(
        node_id: NodeId,
        mbox: &'static NodeMbox,
        state: &'static dyn NodeStateAccess,
        od: &'static [ODEntry<'static>],
    ) -> Self {
        let message_count = 0;
        let sdo_server = SdoServer::new();

        let lss_slave = LssSlave::new(read_identity(od).unwrap(), node_id);
        let nmt_state = NmtState::Bootup;
        let reassigned_node_id = None;

        Self::register_pdo_callbacks(od, mbox, state);
        Self::register_storage_callbacks(od, state);

        let heartbeat_period_ms = read_heartbeat_period(od).expect("Heartbeat object must exist");
        let next_heartbeat_time_us = 0;
        Self {
            node_id,
            nmt_state,
            sdo_server,
            lss_slave,
            message_count,
            od,
            mbox,
            state,
            reassigned_node_id,
            next_heartbeat_time_us,
            heartbeat_period_ms,
            callbacks: Callbacks::default(),
        }
    }

    /// Manually set the node ID. Changing the node id will cause an NMT comm reset to occur,
    /// resetting communication parameter defaults and triggering a bootup heartbeat message if the
    /// ID is valid. Setting the node ID to 255 will put the node into unconfigured mode.
    pub fn set_node_id(&mut self, node_id: NodeId) {
        self.reassigned_node_id = Some(node_id);
    }

    /// Register a callback to store node configuration data persistently
    pub fn register_store_node_config_cb(&mut self, cb: &'static StoreNodeConfigCallback) {
        self.callbacks.store_node_config = Some(cb);
    }

    /// Register a callback to store object data persistently
    pub fn register_store_objects(&mut self, cb: &'static StoreObjectsCallback) {
        self.state.storage_context().store_callback.store(Some(cb));
    }

    /// Run periodic processing
    ///
    /// This should be called periodically by the application so that the node can update it's
    /// state, send periodic messages, process received messages, etc.
    ///
    /// It is sufficient to call this based on a timer, but the [NodeMbox] object also provides a
    /// notification callback, which can be used by an application to accelerate the call to process
    /// when an action is required
    ///
    /// # Arguments
    /// - `now_us`: A monotonic time in microseconds. This is used for measuring time and triggering
    ///   time-based actions such as heartbeat transmission
    /// - `send_cb`: A callback function for transmitting can messages
    pub fn process(&mut self, now_us: u64, send_cb: &mut dyn FnMut(CanMessage)) {
        if let Some(new_node_id) = self.reassigned_node_id.take() {
            self.node_id = new_node_id;
            self.nmt_state = NmtState::Bootup;
        }

        if self.nmt_state == NmtState::Bootup {
            // Set state before calling boot_up, so the heartbeat state is correct
            self.nmt_state = NmtState::PreOperational;
            self.boot_up(send_cb);
        }

        if let Some(req) = self.mbox.read_sdo_mbox() {
            self.message_count += 1;
            if let Some(resp) = self.sdo_server.handle_request(&req, self.od) {
                send_cb(resp.to_can_message(self.sdo_tx_cob_id()));
            }
        }

        if let Some(msg) = self.mbox.read_nmt_mbox() {
            if let Ok(ZencanMessage::NmtCommand(cmd)) = msg.try_into() {
                self.message_count += 1;
                // We cannot respond to NMT commands if we do not have a valid node ID

                if let NodeId::Configured(node_id) = self.node_id {
                    if cmd.node == 0 || cmd.node == node_id.raw() {
                        debug!("Received NMT command: {:?}", cmd.cs);
                        self.handle_nmt_command(cmd.cs);
                    }
                }
            }
        }

        if let Ok(Some(resp)) = self.lss_slave.process(self.mbox.lss_receiver()) {
            send_cb(resp.to_can_message(LSS_RESP_ID));
        }

        // check if a sync has been received
        let sync = self.mbox.read_sync_flag();

        // Swap the active TPDO flag set
        self.state.get_pdo_sync().toggle();

        for pdo in self.state.get_tpdos() {
            let transmission_type = pdo.transmission_type.load();
            if transmission_type >= 254 {
                if pdo.read_events(self.od) {
                    let mut data = [0u8; 8];
                    crate::pdo::read_pdo_data(&mut data, pdo, self.od);
                    let msg = CanMessage::new(pdo.cob_id.load(), &data);
                    send_cb(msg);
                }
            } else if sync && pdo.sync_update() {
                let mut data = [0u8; 8];
                crate::pdo::read_pdo_data(&mut data, pdo, self.od);
                let msg = CanMessage::new(pdo.cob_id.load(), &data);
                send_cb(msg);
            }
        }

        for pdo in self.state.get_tpdos() {
            pdo.clear_events(self.od);
        }

        for rpdo in self.state.get_rpdos() {
            if let Some(new_data) = rpdo.buffered_value.take() {
                crate::pdo::store_pdo_data(&new_data, rpdo, self.od);
            }
        }

        if self.heartbeat_period_ms != 0 && now_us >= self.next_heartbeat_time_us {
            self.send_heartbeat(send_cb);
        }

        if let Some(event) = self.lss_slave.pending_event() {
            info!("LSS Slave Event: {:?}", event);
            match event {
                crate::lss_slave::LssEvent::StoreConfiguration => {
                    if let Some(cb) = self.callbacks.store_node_config {
                        (cb)(&self.node_id)
                    }
                }
                crate::lss_slave::LssEvent::ActivateBitTiming {
                    table: _,
                    index: _,
                    delay: _,
                } => todo!(),
                crate::lss_slave::LssEvent::ConfigureNodeId { node_id } => {
                    self.set_node_id(node_id)
                }
            }
        }
    }

    fn handle_nmt_command(&mut self, cmd: NmtCommandSpecifier) {
        let prev_state = self.nmt_state;

        match cmd {
            NmtCommandSpecifier::Start => self.nmt_state = NmtState::Operational,
            NmtCommandSpecifier::Stop => self.nmt_state = NmtState::Stopped,
            NmtCommandSpecifier::EnterPreOp => self.nmt_state = NmtState::PreOperational,
            NmtCommandSpecifier::ResetApp => {
                // if let Some(cb) = self.app_reset_callback.as_mut() {
                //     cb();
                // }
                self.nmt_state = NmtState::Bootup;
            }
            NmtCommandSpecifier::ResetComm => self.nmt_state = NmtState::Bootup,
        }

        debug!(
            "NMT state changed from {:?} to {:?}",
            prev_state, self.nmt_state
        );
    }

    /// Get the current Node ID
    pub fn node_id(&self) -> u8 {
        self.node_id.into()
    }

    /// Get the current NMT state of the node
    pub fn nmt_state(&self) -> NmtState {
        self.nmt_state
    }

    /// Get the number of received messages
    pub fn rx_message_count(&self) -> u32 {
        self.message_count
    }

    fn sdo_tx_cob_id(&self) -> CanId {
        let node_id: u8 = self.node_id.into();
        CanId::Std(0x580 + node_id as u16)
    }

    fn sdo_rx_cob_id(&self) -> CanId {
        let node_id: u8 = self.node_id.into();
        CanId::Std(0x600 + node_id as u16)
    }

    fn boot_up(&mut self, sender: &mut dyn FnMut(CanMessage)) {
        //self.sdo_server = Some(SdoServer::new());
        let mut i = 0;
        if let NodeId::Configured(node_id) = self.node_id {
            info!("Booting node with ID {}", node_id.raw());
            for pdo in self.state.get_rpdos() {
                if i < 4 {
                    pdo.cob_id
                        .store(CanId::Std(0x200 + i * 0x100 + node_id.raw() as u16));
                } else {
                    pdo.cob_id.store(CanId::Std(0x0));
                }
                pdo.valid.store(false);
                pdo.rtr_disabled.store(false);
                pdo.transmission_type.store(0);
                pdo.inhibit_time.store(0);
                pdo.buffered_value.store(None);
                i += i;
            }

            for pdo in self.state.get_tpdos() {
                if i < 4 {
                    pdo.cob_id
                        .store(CanId::Std(0x180 + i * 0x100 + node_id.raw() as u16));
                } else {
                    pdo.cob_id.store(CanId::Std(0x0));
                }
                pdo.valid.store(false);
                pdo.rtr_disabled.store(false);
                pdo.transmission_type.store(0);
                pdo.inhibit_time.store(0);
                pdo.buffered_value.store(None);
                i += i;
            }

            self.mbox.set_sdo_cob_id(Some(self.sdo_rx_cob_id()));

            // Reset the LSS slave with the new ID
            self.lss_slave = LssSlave::new(read_identity(self.od).unwrap(), self.node_id);

            self.send_heartbeat(sender);
        }
    }

    fn send_heartbeat(&mut self, sender: &mut dyn FnMut(CanMessage)) {
        if let NodeId::Configured(node_id) = self.node_id {
            let heartbeat = Heartbeat {
                node: node_id.raw(),
                toggle: false,
                state: self.nmt_state,
            };
            sender(heartbeat.into());
            self.next_heartbeat_time_us += (self.heartbeat_period_ms as u64) * 1000;
        }
    }

    fn register_storage_callbacks(od: &'static [ODEntry], state: &'static dyn NodeStateAccess) {
        // If the 0x1010 object is present, hook it up
        if let Some(obj) = find_object(od, 0x1010) {
            if let ObjectData::Callback(obj) = obj {
                obj.register(
                    Some(crate::storage::handle_1010_write),
                    Some(crate::storage::handle_1010_read),
                    Some(crate::storage::handle_1010_subinfo),
                    Some(state.storage_context()),
                );
            } else {
                panic!("Object 1010 must be a callback object")
            }
        }
    }

    fn register_pdo_callbacks(
        od: &'static [ODEntry],
        mbox: &'static NodeMbox,
        state: &'static dyn NodeStateAccess,
    ) {
        // register RPDO handlers
        for i in 0..mbox.num_rx_pdos() {
            let comm_id = 0x1400 + i as u16;
            let mapping_id = 0x1600 + i as u16;
            let comm = find_object(od, comm_id).expect("Missing PDO comm object");
            match comm {
                zencan_common::objects::ObjectData::Storage(_) => {
                    panic!("PDO comm object is not a callback")
                }
                zencan_common::objects::ObjectData::Callback(callback_object) => {
                    callback_object.register(
                        Some(crate::pdo::pdo_comm_write_callback),
                        Some(crate::pdo::pdo_comm_read_callback),
                        Some(crate::pdo::pdo_comm_info_callback),
                        Some(&state.get_rpdos()[i]),
                    );
                }
            }
            let mapping = find_object(od, mapping_id).expect("Missing PDO mapping object");
            match mapping {
                zencan_common::objects::ObjectData::Storage(_) => {
                    panic!("PDO mapping object is not a callback")
                }
                zencan_common::objects::ObjectData::Callback(callback_object) => {
                    callback_object.register(
                        Some(crate::pdo::pdo_mapping_write_callback),
                        Some(crate::pdo::pdo_mapping_read_callback),
                        Some(crate::pdo::pdo_mapping_info_callback),
                        Some(&state.get_rpdos()[i]),
                    );
                }
            }
        }

        // Register TPDO handlers
        let tpdos = state.get_tpdos();
        for (i, tpdo) in tpdos.iter().enumerate() {
            let comm_id = 0x1800 + i as u16;
            let mapping_id = 0x1A00 + i as u16;
            let comm = find_object(od, comm_id).expect("Missing PDO comm object");
            match comm {
                zencan_common::objects::ObjectData::Storage(_) => {
                    panic!("PDO comm object is not a callback")
                }
                zencan_common::objects::ObjectData::Callback(callback_object) => {
                    callback_object.register(
                        Some(crate::pdo::pdo_comm_write_callback),
                        Some(crate::pdo::pdo_comm_read_callback),
                        Some(crate::pdo::pdo_comm_info_callback),
                        Some(tpdo),
                    );
                }
            }
            let mapping = find_object(od, mapping_id).expect("Missing PDO mapping object");
            match mapping {
                zencan_common::objects::ObjectData::Storage(_) => {
                    panic!("PDO mapping object is not a callback")
                }
                zencan_common::objects::ObjectData::Callback(callback_object) => {
                    callback_object.register(
                        Some(crate::pdo::pdo_mapping_write_callback),
                        Some(crate::pdo::pdo_mapping_read_callback),
                        Some(crate::pdo::pdo_mapping_info_callback),
                        Some(tpdo),
                    );
                }
            }
        }
    }
}
