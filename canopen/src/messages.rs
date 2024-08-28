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
pub enum NmtState {
    Bootup = 0,
    Stopped = 4,
    Operational = 5,
    PreOperational = 127,
}



pub struct Heartbeat {
    pub state: NmtState,
}

const SYNC_ID: CanId = CanId::std(0x80);

pub struct Sync {

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
        if base == 0x580 && (msg_id == node_id as u16 || msg_id == 0) {
            return true;
        }
    }
    false
}


impl TryFrom<CanFdMessage> for CanOpenMessage {
    type Error = MessageError;

    fn try_from(msg: CanFdMessage) -> Result<Self, Self::Error> {
        let id = msg.id();
        match id {
            NMT_CMD_ID => Ok(CanOpenMessage::NmtCommand(msg.try_into()?)),
            _ => Err(MessageError::UnrecognizedId(id)),
        }
    }
}
pub enum CanOpenMessage {
    NmtCommand(NmtCommand),
    Sync(Sync),
    Heartbeat(Heartbeat),

}

pub enum MessageError {
    MessageTooShort,
    MalformedMsg(CanId),
    UnexpectedId(CanId, CanId),
    InvalidField,
    UnrecognizedId(CanId),
}
