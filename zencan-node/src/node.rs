use zencan_common::{
    messages::{Heartbeat, NmtCommandCmd, NmtState, ZencanMessage}, objects::{find_object, AccessType, Context, DataType, ODEntry, ObjectRawAccess, SubInfo}, sdo::AbortCode, traits::{CanFdMessage, CanId}
};

use crate::{node_mbox::NodeMboxRead, node_state::Pdo};
use crate::{node_state::NodeStateAccess, sdo_server::SdoServer};

use defmt_or_log::warn;

fn pdo_comm_write_callback(
    ctx: &Option<&dyn Context>,
    sub: u8,
    offset: usize,
    buf: &[u8]
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx.unwrap().as_any()
        .downcast_ref().expect("invalid context type in pdo_write_callback");

    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess)
    }

    match sub {
        0 => {
            Err(AbortCode::ReadOnly)
        },
        1 => {
            if buf.len() != 4 {
                return Err(AbortCode::DataTypeMismatch)
            }
            let value = u32::from_le_bytes(buf.try_into().unwrap());
            let valid = (value & (1<<31)) != 0;
            let no_rtr = (value & (1<<30)) != 0;
            let extended_id = (value & (1<<29)) != 0;

            let can_id = if extended_id {
                CanId::Extended(value & 0x1FFFFFFF)
            } else {
                CanId::Std((value & 0x7FF) as u16)
            };
            pdo.cob_id.store(can_id);
            pdo.valid.store(valid);
            pdo.rtr_disabled.store(no_rtr);
            Ok(())
        },
        2 => {
            if buf.len() != 1 {
                return Err(AbortCode::DataTypeMismatch)
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
    sub: u8,
    offset: usize,
    buf: &mut [u8]
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx.unwrap().as_any()
        .downcast_ref().expect("invalid context type in pdo_comm_read_callback");
    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess)
    }
    match sub {
        0 => {
            if buf.len() != 1 {
                return Err(AbortCode::DataTypeMismatch)
            }
            buf[0] = 2;
            Ok(())
        },
        1 => {
            if buf.len() != 4 {
                return Err(AbortCode::DataTypeMismatch)
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
                return Err(AbortCode::DataTypeMismatch)
            }
            let value = pdo.transmission_type.load();
            buf[0] = value;
            Ok(())
        }
        _ => Err(AbortCode::NoSuchSubIndex),
    }

}

fn pdo_comm_info_callback(
    _ctx: &Option<&dyn Context>,
    sub: u8,
) -> Result<SubInfo, AbortCode> {
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
    sub: u8,
    offset: usize,
    buf: &[u8]
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx.unwrap().as_any()
        .downcast_ref().expect("invalid context type in pdo_comm_read_callback");

    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess)
    }

    if sub == 0 {
        Err(AbortCode::ReadOnly)
    } else if sub <= pdo.mapping_params.len() as u8 {
        if buf.len() != 4 {
            return Err(AbortCode::DataTypeMismatch)
        }
        let value = u32::from_le_bytes(buf.try_into().unwrap());
        pdo.mapping_params[(sub - 1) as usize].store(value);
        Ok(())
    } else {
        Err(AbortCode::NoSuchSubIndex)
    }
}
fn pdo_mapping_read_callback(
    ctx: &Option<&dyn Context>,
    sub: u8,
    offset: usize,
    buf: &mut [u8]
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx.unwrap().as_any()
        .downcast_ref().expect("invalid context type in pdo_comm_read_callback");

    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess)
    }

    if sub == 0 {
        if buf.len() != 1 {
            return Err(AbortCode::DataTypeMismatch)
        }
        buf[0] = pdo.mapping_params.len() as u8;
        Ok(())
    } else if sub <= pdo.mapping_params.len() as u8 {
        if buf.len() != 4 {
            return Err(AbortCode::DataTypeMismatch)
        }
        let value = pdo.mapping_params[(sub - 1) as usize].load();
        buf.copy_from_slice(&value.to_le_bytes());
        Ok(())
    } else {
        Err(AbortCode::NoSuchSubIndex)
    }
}

fn pdo_mapping_info_callback(
    ctx: &Option<&dyn Context>,
    sub: u8,
) -> Result<SubInfo, AbortCode> {
    let pdo: &Pdo = ctx.unwrap().as_any()
        .downcast_ref().expect("invalid context type in pdo_comm_read_callback");
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

fn store_pdo_data(
    data: &[u8],
    pdo: &Pdo,
    od: &[ODEntry]
) {
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

pub struct Node<'table> {
    node_id: Option<u8>,
    node_state: NmtState,
    sdo_server: SdoServer,
    message_count: u32,
    od: &'table [ODEntry<'table>],
    mbox: &'static dyn NodeMboxRead,
    state: &'static dyn NodeStateAccess,
}

impl<'table> Node<'table> {
    pub fn new(
        mbox: &'static dyn NodeMboxRead,
        state: &'static dyn NodeStateAccess,
        od: &'table [ODEntry<'table>],
    ) -> Self {
        let message_count = 0;
        let sdo_server = SdoServer::new();
        let node_state = NmtState::Bootup;
        let node_id = None;

        // register PDO handlers
        for i in 0..mbox.num_rx_pdos() {
            let comm_id = 0x1400 + i as u16;
            let mapping_id = 0x1600 + i as u16;
            let comm = find_object(od, comm_id).expect("Missing PDO comm object");
            match comm {
                zencan_common::objects::ObjectData::Storage(_) => panic!("PDO comm object is not a callback"),
                zencan_common::objects::ObjectData::Callback(callback_object) => {
                    callback_object.register(Some(pdo_comm_write_callback), Some(pdo_comm_read_callback), Some(pdo_comm_info_callback), Some(&state.get_rpdos()[i]));
                },
            }
            let mapping = find_object(od, mapping_id).expect("Missing PDO mapping object");
            match mapping {
                zencan_common::objects::ObjectData::Storage(_) => panic!("PDO mapping object is not a callback"),
                zencan_common::objects::ObjectData::Callback(callback_object) => {
                    callback_object.register(Some(pdo_mapping_write_callback), Some(pdo_mapping_read_callback), Some(pdo_mapping_info_callback), Some(&state.get_rpdos()[i]));
                },
            }
        }

        Self {
            node_id,
            node_state,
            sdo_server,
            message_count,
            od,
            mbox,
            state
        }
    }



    pub fn set_node_id(&mut self, node_id: u8) {
        self.node_id = Some(node_id);
        self.mbox.set_sdo_cob_id(Some(self.sdo_rx_cob_id()));
    }

    pub fn process(&mut self, send_cb: &mut dyn FnMut(CanFdMessage)) {
        // Some messages can only be handled after we have a node id
        if let Some(msg) = self.mbox.read_sdo_mbox() {
            self.message_count += 1;
            if let Ok(req) = msg.data().try_into() {
                if let Some(resp) = self.sdo_server.handle_request(&req, self.od) {
                    send_cb(resp.to_can_message(self.sdo_tx_cob_id()));
                }
            } else {
                warn!("Failed to parse an SDO request message");
            }
        }

        if let Some(msg) = self.mbox.read_nmt_mbox() {
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

        for rpdo in self.state.get_rpdos() {
            if let Some(new_data) = rpdo.buffered_value.take() {
                store_pdo_data(&new_data, rpdo, self.od);
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
