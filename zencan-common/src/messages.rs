use snafu::Snafu;

use crate::{
    lss::{LssRequest, LssResponse},
    sdo::{SdoRequest, SdoResponse},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CanId {
    Extended(u32),
    Std(u16),
}

impl CanId {
    pub const fn extended(id: u32) -> CanId {
        CanId::Extended(id)
    }

    pub const fn std(id: u16) -> CanId {
        CanId::Std(id)
    }

    pub fn raw(&self) -> u32 {
        match self {
            CanId::Extended(id) => *id,
            CanId::Std(id) => *id as u32,
        }
    }

    pub fn is_extended(&self) -> bool {
        match self {
            CanId::Extended(_) => true,
            CanId::Std(_) => false,
        }
    }
}

const MAX_DATA_LENGTH: usize = 8;

#[derive(Clone, Copy, Debug)]
pub struct CanMessage {
    pub data: [u8; MAX_DATA_LENGTH],
    pub dlc: u8,
    pub id: CanId,
}

impl Default for CanMessage {
    fn default() -> Self {
        Self { data: [0; MAX_DATA_LENGTH], dlc: 0, id: CanId::Std(0) }
    }
}

impl CanMessage {
    pub fn new(id: CanId, data: &[u8]) -> Self {
        let dlc = data.len() as u8;
        if dlc > MAX_DATA_LENGTH as u8 {
            panic!("Data length exceeds maximum size of {} bytes", MAX_DATA_LENGTH);
        }
        let mut buf = [0u8; MAX_DATA_LENGTH];
        buf[0..dlc as usize].copy_from_slice(data);

        Self { id, dlc, data: buf }
    }

    pub fn id(&self) -> CanId {
        self.id
    }

    pub fn data(&self) -> &[u8] {
        &self.data[0..self.dlc as usize]
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum NmtCommandCmd {
    Start = 1,
    Stop = 2,
    EnterPreOp = 128,
    ResetApp = 129,
    ResetComm = 130,
}

impl NmtCommandCmd {
    pub fn from_byte(b: u8) -> Result<Self, MessageError> {
        match b {
            1 => Ok(Self::Start),
            2 => Ok(Self::Stop),
            128 => Ok(Self::EnterPreOp),
            129 => Ok(Self::ResetApp),
            130 => Ok(Self::ResetComm),
            _ => Err(MessageError::InvalidField),
        }
    }
}

pub const NMT_CMD_ID: CanId = CanId::Std(0);
pub const SYNC_ID: CanId = CanId::Std(0x80);
pub const LSS_RESP_ID: CanId = CanId::Std(0x7E4);
pub const LSS_REQ_ID: CanId = CanId::Std(0x7E5);
pub const HEARTBEAT_ID: u16 = 0x700;
/// The default base ID for sending SDO requests (server node ID is added)
pub const SDO_REQ_BASE: u16 = 0x600;
/// The default base ID for sending SDO responses (server node ID is added)
pub const SDO_RESP_BASE: u16 = 0x580;

#[derive(Debug, Clone, Copy)]
pub struct NmtCommand {
    pub cmd: NmtCommandCmd,
    pub node: u8,
}

impl TryFrom<CanMessage> for NmtCommand {
    type Error = MessageError;

    fn try_from(msg: CanMessage) -> Result<Self, Self::Error> {
        let payload = msg.data();
        if msg.id() != NMT_CMD_ID {
            Err(MessageError::UnexpectedId { cob_id: msg.id(), expected: NMT_CMD_ID })
        } else if payload.len() >= 2 {
            let cmd = NmtCommandCmd::from_byte(payload[0])?;
            let node = payload[1];
            Ok(NmtCommand { cmd, node })
        } else {
            Err(MessageError::MessageTooShort)
        }
    }
}

impl From<NmtCommand> for CanMessage {
    fn from(cmd: NmtCommand) -> Self {
        let mut msg = CanMessage {
            id: NMT_CMD_ID,
            dlc: 2,
            ..Default::default()
        };
        msg.data[0] = cmd.cmd as u8;
        msg.data[1] = cmd.node;
        msg
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum NmtState {
    Bootup = 0,
    Stopped = 4,
    Operational = 5,
    PreOperational = 127,
}

pub struct InvalidNmtStateError(u8);

impl TryFrom<u8> for NmtState {
    type Error = InvalidNmtStateError;

    /// Attempt to convert a u8 to an NmtState enum
    ///
    /// Fails with BadNmtStateError if value is not a valid state
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use NmtState::*;
        match value {
            x if x == Bootup as u8 => Ok(Bootup),
            x if x == Stopped as u8 => Ok(Stopped),
            x if x == Operational as u8 => Ok(Operational),
            x if x == PreOperational as u8 => Ok(PreOperational),
            _ => Err(InvalidNmtStateError(value)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Heartbeat {
    pub node: u8,
    pub toggle: bool,
    pub state: NmtState,
}

impl From<Heartbeat> for CanMessage {
    fn from(value: Heartbeat) -> Self {
        let mut msg = CanMessage {
            id: CanId::Std(HEARTBEAT_ID | value.node as u16),
            dlc: 1,
            ..Default::default()
        };
        msg.data[0] = value.state as u8;
        if value.toggle {
            msg.data[0] |= 1 << 7;
        }
        msg
    }
}
/// Represents a SYNC object/message
///
/// A single CAN node can serve as the SYNC provider, sending a periodic sync object to all other
/// nodes. The one byte count value starts at 1, and increments. On overflow, it should be reset to
/// 1.
#[derive(Debug, Clone, Copy)]
pub struct SyncObject {
    count: u8,
}

impl SyncObject {
    pub fn new(count: u8) -> Self {
        Self { count }
    }
}

impl Default for SyncObject {
    fn default() -> Self {
        Self { count: 1 }
    }
}

impl From<SyncObject> for CanMessage {
    fn from(value: SyncObject) -> Self {
        let mut msg = CanMessage {
            id: SYNC_ID,
            dlc: 1,
            data: [0; MAX_DATA_LENGTH],
        };
        msg.data[0] = value.count;
        msg
    }
}

impl From<CanMessage> for SyncObject {
    fn from(msg: CanMessage) -> Self {
        if msg.id() == SYNC_ID {
            let count = msg.data()[0];
            Self { count }
        } else {
            panic!("Invalid message ID for SyncObject");
        }
    }
}

// pub struct SdoRequest {
//     pub ccs: ClientCommand,
//     pub index: u16,
//     pub sub: u8,
//     pub data: [u8; 4],
//     pub len: u8,
// }

// pub struct SdoResponse {
//     pub scs: ServerCommand,
//     pub index: u16,
//     pub sub: u8,
//     /// Expedited flag
//     pub e: bool,
//     /// size indicator
//     pub s: bool,
//     /// If e=1 and s=1, indicates the number of bytes in data which do not contain valid data
//     pub n: u8,
//     pub data: [u8; 4],
// }

pub fn is_std_sdo_request(can_id: CanId, node_id: u8) -> bool {
    if let CanId::Std(id) = can_id {
        let base = id & 0xff80;
        let msg_id = id & 0x7f;
        if base == SDO_REQ_BASE && (msg_id == node_id as u16 || msg_id == 0) {
            return true;
        }
    }
    false
}

impl TryFrom<CanMessage> for ZencanMessage {
    type Error = MessageError;

    fn try_from(msg: CanMessage) -> Result<Self, Self::Error> {
        let cob_id = msg.id();
        if cob_id == NMT_CMD_ID {
            Ok(ZencanMessage::NmtCommand(msg.try_into()?))
        } else if cob_id.raw() & !0x7f == HEARTBEAT_ID as u32 {
            let node = (cob_id.raw() & 0x7f) as u8;
            let toggle = (msg.data[0] & (1 << 7)) != 0;
            let state: NmtState = (msg.data[0] & 0x7f)
                .try_into()
                .map_err(|e: InvalidNmtStateError| MessageError::InvalidNmtState { value: e.0 })?;
            Ok(ZencanMessage::Heartbeat(Heartbeat {
                node,
                toggle,
                state,
            }))
        } else if cob_id.raw() & 0xff80 == 0x580 {
            // SDO response
            let resp: SdoResponse = msg.try_into().map_err(|_| MessageError::MalformedMsg { cob_id })?;
            Ok(ZencanMessage::SdoResponse(resp))
        } else if cob_id.raw() >= 0x580 && cob_id.raw() <= 0x580 + 256 {
            // SDO request
            let req: SdoRequest = msg
                .data()
                .try_into()
                .map_err(|_| MessageError::MalformedMsg { cob_id })?;
            Ok(ZencanMessage::SdoRequest(req))
        } else if cob_id == SYNC_ID {
            Ok(ZencanMessage::Sync(msg.into()))
        } else if cob_id == LSS_REQ_ID {
            let req: LssRequest = msg
                .data()
                .try_into()
                .map_err(|_| MessageError::MalformedMsg { cob_id })?;
            Ok(ZencanMessage::LssRequest(req))
        } else if cob_id == LSS_RESP_ID {
            let resp: LssResponse = msg
                .data()
                .try_into()
                .map_err(|_| MessageError::MalformedMsg { cob_id })?;
            Ok(ZencanMessage::LssResponse(resp))
        } else {
            Err(MessageError::UnrecognizedId { cob_id })
        }
    }
}

#[derive(Debug)]
pub enum ZencanMessage {
    NmtCommand(NmtCommand),
    Sync(SyncObject),
    Heartbeat(Heartbeat),
    SdoRequest(SdoRequest),
    SdoResponse(SdoResponse),
    LssRequest(LssRequest),
    LssResponse(LssResponse),
}

#[derive(Debug, Clone, Copy, PartialEq, Snafu)]
pub enum MessageError {
    MessageTooShort,
    MalformedMsg{ cob_id: CanId },
    /// The message ID was not the expected value
    #[snafu(display("Unexpected message ID found: {cob_id:?}, expected: {expected:?}"))]
    UnexpectedId{ cob_id: CanId, expected: CanId },
    InvalidField,
    UnrecognizedId{ cob_id: CanId },
    /// The NMT state integer in the message is not a valid NMT state
    InvalidNmtState { value: u8 },
    /// An invalid LSS command specifier was found in the message
    #[snafu(display("Unexpected LSS command: {value}"))]
    UnexpectedLssCommand { value: u8 },
}
