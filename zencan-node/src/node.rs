use zencan_common::{
    lss::{self, LssIdentity}, messages::{Heartbeat, NmtCommandCmd, NmtState, ZencanMessage, LSS_RESP_ID}, objects::{find_object, AccessType, Context, DataType, ODEntry, ObjectRawAccess, SubInfo}, sdo::AbortCode, traits::{CanFdMessage, CanId}
};

use crate::{lss_slave::LssSlave, node_mbox::NodeMboxRead, node_state::Pdo};
use crate::{node_state::NodeStateAccess, sdo_server::SdoServer};

use defmt_or_log::{debug, info, warn};

fn pdo_comm_write_callback(
    ctx: &Option<&dyn Context>,
    _od: &[ODEntry],
    sub: u8,
    offset: usize,
    buf: &[u8],
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx
        .unwrap()
        .as_any()
        .downcast_ref()
        .expect("invalid context type in pdo_write_callback");

    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess);
    }

    match sub {
        0 => Err(AbortCode::ReadOnly),
        1 => {
            if buf.len() != 4 {
                return Err(AbortCode::DataTypeMismatch);
            }
            let value = u32::from_le_bytes(buf.try_into().unwrap());
            let valid = (value & (1 << 31)) != 0;
            let no_rtr = (value & (1 << 30)) != 0;
            let extended_id = (value & (1 << 29)) != 0;

            let can_id = if extended_id {
                CanId::Extended(value & 0x1FFFFFFF)
            } else {
                CanId::Std((value & 0x7FF) as u16)
            };
            pdo.cob_id.store(can_id);
            pdo.valid.store(valid);
            pdo.rtr_disabled.store(no_rtr);
            Ok(())
        }
        2 => {
            if buf.len() != 1 {
                return Err(AbortCode::DataTypeMismatch);
            }
            let value = buf[0];
            pdo.transmission_type.store(value);
            Ok(())
        }
        _ => Err(AbortCode::NoSuchSubIndex),
    }
}

fn pdo_comm_read_callback(
    ctx: &Option<&dyn Context>,
    _od: &[ODEntry],
    sub: u8,
    offset: usize,
    buf: &mut [u8],
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx
        .unwrap()
        .as_any()
        .downcast_ref()
        .expect("invalid context type in pdo_comm_read_callback");
    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess);
    }
    match sub {
        0 => {
            if buf.len() != 1 {
                return Err(AbortCode::DataTypeMismatch);
            }
            buf[0] = 2;
            Ok(())
        }
        1 => {
            if buf.len() != 4 {
                return Err(AbortCode::DataTypeMismatch);
            }

            let cob_id = pdo.cob_id.load();
            let mut value = cob_id.raw();
            if cob_id.is_extended() {
                value |= 1 << 29;
            }
            if pdo.rtr_disabled.load() {
                value |= 1 << 30;
            }
            if pdo.valid.load() {
                value |= 1 << 31;
            }

            buf.copy_from_slice(&value.to_le_bytes());
            Ok(())
        }
        2 => {
            if buf.len() != 1 {
                return Err(AbortCode::DataTypeMismatch);
            }
            let value = pdo.transmission_type.load();
            buf[0] = value;
            Ok(())
        }
        _ => Err(AbortCode::NoSuchSubIndex),
    }
}

fn pdo_comm_info_callback(_ctx: &Option<&dyn Context>, sub: u8) -> Result<SubInfo, AbortCode> {
    match sub {
        0 => Ok(SubInfo {
            data_type: DataType::UInt8,
            size: 1,
            access_type: AccessType::Ro,
        }),
        1 => Ok(SubInfo {
            data_type: DataType::UInt32,
            size: 4,
            access_type: AccessType::Rw,
        }),
        2 => Ok(SubInfo {
            data_type: DataType::UInt8,
            size: 1,
            access_type: AccessType::Rw,
        }),
        _ => Err(AbortCode::NoSuchSubIndex),
    }
}

fn pdo_mapping_write_callback(
    ctx: &Option<&dyn Context>,
    od: &[ODEntry],
    sub: u8,
    offset: usize,
    buf: &[u8],
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx
        .unwrap()
        .as_any()
        .downcast_ref()
        .expect("invalid context type in pdo_comm_read_callback");

    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess);
    }

    if sub == 0 {
        Err(AbortCode::ReadOnly)
    } else if sub <= pdo.mapping_params.len() as u8 {
        if buf.len() != 4 {
            return Err(AbortCode::DataTypeMismatch);
        }
        let value = u32::from_le_bytes(buf.try_into().unwrap());

        let object_id = (value >> 16) as u16;
        let sub_index = ((value & 0xFF00) >> 8) as u8;
        // Rounding up to BYTES, because we do not currently support bit access
        let length = (value & 0xFF) as usize;
        if (length % 8) != 0 {
            // only support byte level access for now
            return Err(AbortCode::IncompatibleParameter);
        }
        let entry = find_object(od, object_id).ok_or(AbortCode::NoSuchObject)?;
        let sub_info = entry.sub_info(sub_index)?;
        if sub_info.size < length / 8 {
            return Err(AbortCode::IncompatibleParameter);
        }
        pdo.mapping_params[(sub - 1) as usize].store(value);
        Ok(())
    } else {
        Err(AbortCode::NoSuchSubIndex)
    }
}
fn pdo_mapping_read_callback(
    ctx: &Option<&dyn Context>,
    _od: &[ODEntry],
    sub: u8,
    offset: usize,
    buf: &mut [u8],
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx
        .unwrap()
        .as_any()
        .downcast_ref()
        .expect("invalid context type in pdo_comm_read_callback");

    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess);
    }

    if sub == 0 {
        if buf.len() != 1 {
            return Err(AbortCode::DataTypeMismatch);
        }
        buf[0] = pdo.mapping_params.len() as u8;
        Ok(())
    } else if sub <= pdo.mapping_params.len() as u8 {
        if buf.len() != 4 {
            return Err(AbortCode::DataTypeMismatch);
        }
        let value = pdo.mapping_params[(sub - 1) as usize].load();
        buf.copy_from_slice(&value.to_le_bytes());
        Ok(())
    } else {
        Err(AbortCode::NoSuchSubIndex)
    }
}

fn pdo_mapping_info_callback(ctx: &Option<&dyn Context>, sub: u8) -> Result<SubInfo, AbortCode> {
    let pdo: &Pdo = ctx
        .unwrap()
        .as_any()
        .downcast_ref()
        .expect("invalid context type in pdo_comm_read_callback");
    if sub == 0 {
        Ok(SubInfo {
            size: 1,
            data_type: DataType::UInt8,
            access_type: AccessType::Ro,
        })
    } else if sub <= pdo.mapping_params.len() as u8 {
        Ok(SubInfo {
            size: 4,
            data_type: DataType::UInt32,
            access_type: AccessType::Rw,
        })
    } else {
        Err(AbortCode::NoSuchSubIndex)
    }
}

fn store_pdo_data(data: &[u8], pdo: &Pdo, od: &[ODEntry]) {
    let mut offset = 0;
    for i in 0..pdo.mapping_params.len() {
        let param = pdo.mapping_params[i].load();
        if param == 0 {
            break;
        }
        let object_id = (param >> 16) as u16;
        let sub_index = ((param & 0xFF00) >> 8) as u8;
        // Rounding up to BYTES, because we do not currently support bit access
        let length = (((param & 0xFF) + 7) / 8) as usize;
        let entry = find_object(od, object_id).expect("Invalid mapping parameter");
        if offset + length > data.len() {
            break;
        }
        let data_to_write = &data[offset..offset + length];
        // There's no mechanism to report an error here, so we just ignore it if it fails. We can
        // check that the PDO mapping is valid when it is written to the object dictionary, to make
        // it impossible to fail.
        entry.write(sub_index, 0, data_to_write).ok();
        offset += length;
    }
}

fn read_pdo_data(data: &mut[u8], pdo: &Pdo, od: &[ODEntry]) {
    let mut offset = 0;
    for i in 0..pdo.mapping_params.len() {
        let param = pdo.mapping_params[i].load();
        if param == 0 {
            break;
        }
        let object_id = (param >> 16) as u16;
        let sub_index = ((param & 0xFF00) >> 8) as u8;
        // Rounding up to BYTES, because we do not currently support bit access
        let length = (((param & 0xFF) + 7) / 8) as usize;
        let entry = find_object(od, object_id).expect("Invalid mapping parameter");
        if offset + length > data.len() {
            break;
        }
        // There's no mechanism to report an error here, so we just ignore it if it fails. We can
        // check that the PDO mapping is valid when it is written to the object dictionary, to make
        // it impossible to fail.
        entry.read(sub_index, 0, &mut data[offset..offset + length]).ok();
        offset += length;
    }
}

/// An enum representing the node ID of a CANopen node. The node ID must be between 1 and 127 for
/// configured devices, with the special value of 255 used to represent an unconfigured device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeId {
    Unconfigured,
    Configured(NodeIdNum),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeIdNum(u8);
impl NodeIdNum {
    pub fn new(value: u8) -> Result<Self, InvalidNodeIdError> {
        if (value > 0 && value < 128) || value == 255 {
            Ok(NodeIdNum(value))
        } else {
            Err(InvalidNodeIdError)
        }
    }
}

impl NodeId {
    pub fn new(value: u8) -> Result<Self, InvalidNodeIdError> {
        if value == 255 {
            Ok(NodeId::Unconfigured)
        } else {
            NodeIdNum::new(value).map(NodeId::Configured)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidNodeIdError;

impl core::fmt::Display for InvalidNodeIdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid node ID")
    }
}
impl core::error::Error for InvalidNodeIdError {}

impl TryFrom<u8> for NodeId {
    type Error = InvalidNodeIdError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 255 {
            Ok(NodeId::Unconfigured)
        } else {
            Ok(NodeId::Configured(NodeIdNum(value)))
        }
    }
}

impl From<NodeId> for u8 {
    fn from(value: NodeId) -> Self {
        match value {
            NodeId::Unconfigured => 255,
            NodeId::Configured(id) => id.0,
        }
    }
}

pub struct Node<'table> {
    node_id: NodeId,
    nmt_state: NmtState,
    sdo_server: SdoServer,
    lss_slave: LssSlave,
    message_count: u32,
    od: &'table [ODEntry<'table>],
    mbox: &'static dyn NodeMboxRead,
    state: &'static dyn NodeStateAccess,
    reassigned_node_id: Option<NodeId>,
}

impl<'table> Node<'table> {
    pub fn new(
        node_id: NodeId,
        mbox: &'static dyn NodeMboxRead,
        state: &'static dyn NodeStateAccess,
        od: &'table [ODEntry<'table>],
    ) -> Self {
        let message_count = 0;
        let sdo_server = SdoServer::new();
        // TODO: Lookup identity. Should node actually be created with a node id?
        let lss_slave = LssSlave::new(LssIdentity::new(10, 20, 30, 40), node_id);
        let nmt_state = NmtState::Bootup;
        let node_id = node_id.try_into().unwrap_or(NodeId::Unconfigured);
        let reassigned_node_id = None;

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
                        Some(pdo_comm_write_callback),
                        Some(pdo_comm_read_callback),
                        Some(pdo_comm_info_callback),
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
                        Some(pdo_mapping_write_callback),
                        Some(pdo_mapping_read_callback),
                        Some(pdo_mapping_info_callback),
                        Some(&state.get_rpdos()[i]),
                    );
                }
            }
        }

        // Register TPDO handlers
        let tpdos = state.get_tpdos();
        for i in 0..tpdos.len() {
            let comm_id = 0x1800 + i as u16;
            let mapping_id = 0x1A00 + i as u16;
            let comm = find_object(od, comm_id).expect("Missing PDO comm object");
            match comm {
                zencan_common::objects::ObjectData::Storage(_) => {
                    panic!("PDO comm object is not a callback")
                }
                zencan_common::objects::ObjectData::Callback(callback_object) => {
                    callback_object.register(
                        Some(pdo_comm_write_callback),
                        Some(pdo_comm_read_callback),
                        Some(pdo_comm_info_callback),
                        Some(&tpdos[i]),
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
                        Some(pdo_mapping_write_callback),
                        Some(pdo_mapping_read_callback),
                        Some(pdo_mapping_info_callback),
                        Some(&tpdos[i]),
                    );
                }
            }
        }

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
        }
    }

    /// Manually set the node ID. Changing the node id will cause an NMT comm reset to occur,
    /// resetting communication parameter defaults and triggering a bootup heartbeat message if the
    /// ID is valid. Setting the node ID to 255 will put the node into unconfigured mode.
    pub fn set_node_id(&mut self, node_id: NodeId) {
        self.reassigned_node_id = Some(node_id);
    }

    pub fn process(&mut self, send_cb: &mut dyn FnMut(CanFdMessage)) {
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
                    if cmd.node == 0 || cmd.node == node_id.0 {
                        debug!("Received NMT command: {:?}", cmd.cmd);
                        self.handle_nmt_command(cmd.cmd);
                    }
                }
            }
        }

        if let Ok(Some(resp)) = self.lss_slave.process(self.mbox.lss_receiver()) {
            send_cb(resp.to_can_message(LSS_RESP_ID));
        }

                // check if a sync has been received
        if self.mbox.read_sync_flag() {
            for pdo in self.state.get_tpdos() {
                if pdo.sync_update() {
                    let mut data = [0u8; 8];
                    read_pdo_data(&mut data, pdo, self.od);
                    let msg = CanFdMessage::new(pdo.cob_id.load(), &data);
                    send_cb(msg);
                }
            }
        }

        for rpdo in self.state.get_rpdos() {
            if let Some(new_data) = rpdo.buffered_value.take() {
                store_pdo_data(&new_data, rpdo, self.od);
            }
        }

        if let Some(event) = self.lss_slave.pending_event() {
            info!("LSS Slave Event: {:?}", event);
            match event {
                crate::lss_slave::LssEvent::StoreConfiguration => todo!(),
                crate::lss_slave::LssEvent::ActivateBitTiming { table: _, index: _, delay: _ } => todo!(),
                crate::lss_slave::LssEvent::ConfigureNodeId { node_id } => self.set_node_id(node_id),
            }
        }
    }

    fn handle_nmt_command(&mut self, cmd: NmtCommandCmd) {
        let prev_state = self.nmt_state;

        match cmd {
            NmtCommandCmd::Start => self.nmt_state = NmtState::Operational,
            NmtCommandCmd::Stop => self.nmt_state = NmtState::Stopped,
            NmtCommandCmd::EnterPreOp => self.nmt_state = NmtState::PreOperational,
            NmtCommandCmd::ResetApp => {
                // if let Some(cb) = self.app_reset_callback.as_mut() {
                //     cb();
                // }
                self.nmt_state = NmtState::Bootup;
            }
            NmtCommandCmd::ResetComm => self.nmt_state = NmtState::Bootup,
        }

        debug!(
            "NMT state changed from {:?} to {:?}",
            prev_state, self.nmt_state
        );
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
        self.node_id.into()
    }

    pub fn nmt_state(&self) -> NmtState {
        self.nmt_state
    }

    pub fn rx_message_count(&self) -> u32 {
        self.message_count
    }

    pub fn sdo_tx_cob_id(&self) -> CanId {
        let node_id: u8 = self.node_id.into();
        CanId::Std(0x580 + node_id as u16)
    }

    pub fn sdo_rx_cob_id(&self) -> CanId {
        let node_id: u8 = self.node_id.into();
        CanId::Std(0x600 + node_id as u16)
    }

    fn boot_up(&mut self, sender: &mut dyn FnMut(CanFdMessage)) {
        //self.sdo_server = Some(SdoServer::new());
        let mut i = 0;
        if let NodeId::Configured(node_id) = self.node_id {
            info!("Booting node with ID {}", node_id.0);
            for pdo in self.state.get_rpdos() {
                if i < 4 {
                    pdo.cob_id.store(CanId::Std(
                        0x200 + i * 0x100 + node_id.0 as u16,
                    ));
                } else {
                    pdo.cob_id.store(CanId::Std(0x0));
                }
                pdo.valid.store(false);
                pdo.rtr_disabled.store(false);
                pdo.transmission_type.store(0);
                pdo.inhibit_time.store(0);
                pdo.event_timer.store(0);
                pdo.sync_start.store(0);
                pdo.buffered_value.store(None);
                i += i;
            }

            for pdo in self.state.get_tpdos() {
                if i < 4 {
                    pdo.cob_id.store(CanId::Std(
                        0x180 + i * 0x100 + node_id.0 as u16,
                    ));
                } else {
                    pdo.cob_id.store(CanId::Std(0x0));
                }
                pdo.valid.store(false);
                pdo.rtr_disabled.store(false);
                pdo.transmission_type.store(0);
                pdo.inhibit_time.store(0);
                pdo.event_timer.store(0);
                pdo.sync_start.store(0);
                pdo.buffered_value.store(None);
                i += i;
            }

            self.mbox.set_sdo_cob_id(Some(self.sdo_rx_cob_id()));

            self.lss_slave = LssSlave::new(
                LssIdentity::new(10, 20, 30, 40),
                self.node_id,
            );

            sender(
                Heartbeat {
                    node: node_id.0,
                    toggle: false,
                    state: self.nmt_state,
                }
                .into(),
            );
        }

    }
}

// pub struct PdoServer<const N_RX {

// }
