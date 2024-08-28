use futures::channel::mpsc::{Receiver, Sender};

use crate::{traits::{CanFdMessage, CanId, CanSender, CanReceiver, MessageHandler}, stack::Stack};

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum AbortCode {
    /// Toggle bit not alternated
    ToggleNotAlternated = 0x0503_0000,
    /// SDO protocol timed out
    SdoTimeout = 0x0504_0000,
    /// Client/server command specifier not valid or unknown
    InvalidCommandSpecifier = 0x0504_0001,
    /// Invalid block size (block mode only)
    InvalidBlockSize = 0x0504_0002,
    /// Invalid sequence number (block mode only)
    InvalidSequenceNumber = 0x0504_0003,
    /// CRC Error (block mode only )
    CrcError = 0x0504_0004,
    /// Out of memory
    OutOfMemory = 0x0504_0005,
    /// Unsupported access to an object
    UnsupportedAccess = 0x0601_0000,
    /// Attempt to read a write only object
    WriteOnly = 0x0601_0001,
    /// Attempt to write a read only object
    ReadOnly = 0x0601_0002,
    /// Object does not exist in the dictionary
    NoSuchObject = 0x0602_0000,
    /// Object cannot be mapped to the PDO
    UnnallowedPdo = 0x0604_0041,
    /// The number and length of objects would exceed PDO length
    PdoTooLong = 0x0604_0042,
    /// General parameter incompatibility
    IncompatibleParameter = 0x0604_0043,
    /// Access failed due to hardware error
    HardwareError = 0x0606_0000,
    /// Data type does not match, length of service parameter does not match
    DataTypeMismatch = 0x0607_0010,
    /// Data type does not match, length of service parameter too high
    DataTypeMismatchLengthHigh = 0x0607_0012,
    /// Data type does not match, length of service parameter too low
    DataTypeMismatchLengthLow = 0x0607_0013,
    /// Sub-index does not exist
    NoSuchSubIndex = 0x0609_0011,
    /// Invalid value for parameter (download only)
    InvalidValue = 0x0609_0030,
    /// Value of parameter too high (download only)
    ValueTooHigh = 0x0609_0031,
    /// Value of parameter too low (download only)
    ValueTooLow = 0x0609_0032,
    ResourceNotAvailable = 0x060A_0023,
    GeneralError = 0x0800_0000,
    /// Data cannot be transferred or stored to the application
    CantStore = 0x0800_0020,
    /// Data cannot be transferred or stored to the application because of local control
    CantStoreLocalControl = 0x0800_0021,
    /// Data cannot be transferred or stored to the application because of the device state
    CantStoreDeviceState = 0x0800_0022,
    /// No object dictionary is present
    NoObjectDict = 0x0800_0023,
    /// No data available
    NoData = 0x0800_0024,
    /// An unrecognized abort code
    Unknown,
}
const SDO_REQ_BASE: u16 = 0x600;
const SDO_RESP_BASE: u16 = 0x580;

pub enum ClientCommand {
    ReqDownloadSegment = 0,
    InitiateDownload = 1,
    InitiateUpload = 2,
    ReqUploadSegment = 3,
    Abort = 4,
    ReqBlockUpload = 5,
    ReqBlockDownload = 6,
}

impl TryFrom<u8> for ClientCommand {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use ClientCommand::*;
        match value {
            0 => Ok(ReqDownloadSegment),
            1 => Ok(InitiateDownload),
            2 => Ok(InitiateUpload),
            3 => Ok(ReqUploadSegment),
            4 => Ok(Abort),
            5 => Ok(ReqBlockUpload),
            6 => Ok(ReqBlockDownload),
            _ => Err(()),
        }
    }
}

pub enum ServerCommand {
    SeqmentUpload = 0,
    Upload = 2,
    Abort = 4,
}

pub enum SdoRequest {
    InitiateDownload {
        // Number of unused bytes in data
        n: u8,
        // Expedited
        e: bool,
        // size valid
        s: bool,
        // Object index
        index: u16,
        // Object sub-index
        sub: u8,
        // data (value on expedited, size when e=1 and s=1)
        data: [u8; 4],
    },
    DownloadSegment {
        // Toggle flag
        t: bool,
        // Number of unused bytes in data
        n: u8,
        // When set, indicates there are no more segments to be sent
        c: bool,
        // Segment data
        data: [u8; 7],
    },
    InitiateUpload {
        index: u16,
        sub: u8,
    },
    ReqUploadSegment {
        t: bool,
    },
    InitiateBlockDownload {
        // Client CRC supported flag
        cc: bool,
        // size flag
        s: bool,
        // client sub command
        cs: bool,
        index: u16,
        sub: u8,
        // If s=1, contains the number of bytes to be downloaded
        size: u32,
    },
    InitiateBlockUpload {},
}

pub enum SdoResponse {
    ConfirmUpload {
        // Number of unused bytes in data
        n: u8,
        // Expedited flag
        e: bool,
        // size flag
        s: bool,
        index: u16,
        sub: u8,
        // Value if e=1, or size if s=1
        data: [u8; 4],
    },
    UploadSegment {
        t: bool,
        n: u8,
        c: bool,
        data: [u8; 7],
    },
    ConfirmBlockDownload {
        sc: bool,
    },
}

impl SdoResponse {
    pub fn expedited_upload(index: u16, sub: u8, data: &[u8]) -> SdoResponse {
        if data.len() > 4 {
            panic!("Cannot create expedited upload with more than 4 bytes");
        }

        let mut msg_data = [0; 4];
        msg_data[0..data.len()].copy_from_slice(data);

        SdoResponse::ConfirmUpload {
            index,
            sub,
            e: true,
            s: true,
            n: 4 - data.len() as u8,
            data: msg_data,
        }
    }
}

impl SdoResponse {
    pub fn to_can_message(self, id: CanId) -> CanFdMessage {
        let mut payload = [0; 64];

        match self {
            SdoResponse::ConfirmUpload {
                n,
                e,
                s,
                index,
                sub,
                data,
            } => {
                payload[0] =
                    (ServerCommand::Upload as u8) << 5 | (n << 2) | ((e as u8) << 1) | s as u8;
                payload[1] = (index & 0xff) as u8;
                payload[2] = (index >> 8) as u8;
                payload[3] = sub;
                payload[4..8].copy_from_slice(&data);
                CanFdMessage {
                    data: payload,
                    dlc: 8,
                    id,
                }
            }
            SdoResponse::UploadSegment { t, n, c, data } => todo!(),
            SdoResponse::ConfirmBlockDownload { sc } => todo!(),
        }
    }
}

pub struct SdoServer {
    request_state: Option<bool>,
    tx_cob_id: CanId,
}

impl SdoServer {
    pub fn new(tx_cob_id: CanId) -> Self {
        let request_state = None;
        Self {
            request_state,
            tx_cob_id,
        }
    }

    pub fn handle_request(&mut self, data: &[u8], sender: &mut dyn CanSender) {
        let msg = if let Ok(msg) = data.try_into() {
            msg
        } else {
            return;
        };

        match msg {
            SdoRequest::InitiateUpload { index, sub } => {
                let data = [0, 1, 2, 3];
                self.request_state = Some(false);
                let resp = SdoResponse::expedited_upload(index, sub, &data);
                sender.send(resp.to_can_message(self.tx_cob_id)).unwrap();
            }
            SdoRequest::InitiateDownload {
                n,
                e,
                s,
                index,
                sub,
                data,
            } => todo!(),
            SdoRequest::DownloadSegment { t, n, c, data } => todo!(),

            SdoRequest::ReqUploadSegment { t } => todo!(),
            SdoRequest::InitiateBlockDownload {
                cc,
                s,
                cs,
                index,
                sub,
                size,
            } => todo!(),
            SdoRequest::InitiateBlockUpload {} => todo!(),
        }
    }
}

pub struct SdoClient {
    req_cob_id: CanId,
    resp_cob_id: CanId,
    receiver: Receiver<CanFdMessage>,
    sender: Sender<CanFdMessage>,
}

impl SdoClient {
    pub fn new_std(node_id: u8) -> Self {
        let (sender, receiver) = futures::channel::mpsc::channel(1);
        Self {
            req_cob_id: CanId::Std(0x600 + node_id as u16),
            resp_cob_id: CanId::Std(0x580 + node_id as u16),
            receiver,
            sender,
        }
    }

    pub fn message_handler(&mut self) -> MessageHandler {
        let sender = self.sender.clone();
        let id = self.resp_cob_id;
        MessageHandler { sender, id }
    }

}

pub enum SdoError {
    Abort(AbortCode),
}

impl TryFrom<&[u8]> for SdoRequest {
    type Error = SdoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 8 {
            return Err(SdoError::Abort(AbortCode::DataTypeMismatchLengthLow));
        }
        let ccs = value[0] >> 5;
        let ccs: ClientCommand = match ccs.try_into() {
            Ok(ccs) => ccs,
            Err(_) => return Err(SdoError::Abort(AbortCode::InvalidCommandSpecifier)),
        };

        match ccs {
            ClientCommand::ReqDownloadSegment => todo!(),
            ClientCommand::InitiateDownload => todo!(),
            ClientCommand::InitiateUpload => {
                let index = value[1] as u16 | ((value[2] as u16) << 8);
                let sub = value[3];
                Ok(SdoRequest::InitiateUpload { index, sub })
            }
            ClientCommand::ReqUploadSegment => todo!(),
            ClientCommand::Abort => todo!(),
            ClientCommand::ReqBlockUpload => todo!(),
            ClientCommand::ReqBlockDownload => todo!(),
        }
    }
}
