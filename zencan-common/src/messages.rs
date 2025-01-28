use crate::traits::{CanId, CanFdMessage};

#[derive(Copy, Clone, Debug)]
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
            _ => Err(MessageError::InvalidField)
        }
    }
}

const NMT_CMD_ID: CanId = CanId::Std(0);
pub const HEARTBEAT_ID: u16 = 0x700;
/// The default base ID for sending SDO requests (server node ID is added)
const SDO_REQ_BASE: u16 = 0x600;
/// The default base ID for sending SDO responses (server node ID is added)
const SDO_RESP_BASE: u16 = 0x580;

pub struct NmtCommand {
    pub cmd: NmtCommandCmd,
    pub node: u8,
}

impl TryFrom<CanFdMessage> for NmtCommand {
    type Error = MessageError;

    fn try_from(msg: CanFdMessage) -> Result<Self, Self::Error> {
        let payload = msg.data();
        if msg.id() != NMT_CMD_ID {
            Err(MessageError::UnexpectedId(msg.id(), NMT_CMD_ID))
        } else if payload.len() >= 2 {
            let cmd = NmtCommandCmd::from_byte(payload[0])?;
            let node = payload[1];
            Ok(NmtCommand { cmd, node })
        } else {
            Err(MessageError::MessageTooShort)
        }
    }
}

impl From<NmtCommand> for CanFdMessage {
    fn from(cmd: NmtCommand) -> Self {
        let mut msg = CanFdMessage::default();
        msg.id = NMT_CMD_ID;
        msg.dlc = 2;
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

pub struct Heartbeat {
    pub node: u8,
    pub toggle: bool,
    pub state: NmtState,
}

impl From<Heartbeat> for CanFdMessage {
    fn from(value: Heartbeat) -> Self {
        let mut msg = CanFdMessage::default();
        msg.id = CanId::Std(HEARTBEAT_ID | value.node as u16);
        msg.dlc = 1;
        msg.data[0] = value.state as u8;
        if value.toggle {
            msg.data[0] |= 1<<7;
        }

        msg
    }
}

pub const SYNC_ID: CanId = CanId::std(0x80);

/// Represents a SYNC object/message
///
/// A single CAN node can serve as the SYNC provider, sending a periodic sync object to all other
/// nodes. The one byte count value starts at 1, and increments. On overflow, it should be reset to
/// 1.
#[derive(Debug, Clone, Copy)]
pub struct SyncObject {
    count: u8
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

impl From<SyncObject> for CanFdMessage {
    fn from(value: SyncObject) -> Self {
        let mut msg = CanFdMessage::default();
        msg.id = SYNC_ID;
        msg.dlc = 1;
        msg.data[0] = value.count;
        msg
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

impl TryFrom<CanFdMessage> for zencanMessage {
    type Error = MessageError;

    fn try_from(msg: CanFdMessage) -> Result<Self, Self::Error> {
        let id = msg.id();
        if id == NMT_CMD_ID {
            Ok(zencanMessage::NmtCommand(msg.try_into()?))
        } else if id.raw() & !0x7f == HEARTBEAT_ID as u32 {
            let node = (id.raw() & 0x7f) as u8;
            let toggle = (msg.data[0] & (1<<7)) != 0;
            let state: NmtState = (msg.data[0] & 0x7f)
                .try_into()
                .map_err(|e: InvalidNmtStateError| MessageError::InvalidNmtState(e.0))?;
            Ok(zencanMessage::Heartbeat(Heartbeat { node, toggle, state }))
        } else {
            Err(MessageError::UnrecognizedId(id))
        }
    }
}

pub enum zencanMessage {
    NmtCommand(NmtCommand),
    Sync(SyncObject),
    Heartbeat(Heartbeat),

}

pub enum MessageError {
    MessageTooShort,
    MalformedMsg(CanId),
    UnexpectedId(CanId, CanId),
    InvalidField,
    UnrecognizedId(CanId),
    InvalidNmtState(u8),
}
