use crate::traits::{CanFdMessage, CanId};

pub enum SdoError {
    Abort(AbortCode),
}

/// Specifies the possible server command specifier (SCS) values in SDO response packets
pub enum ServerCommand {
    SegmentUpload = 0,
    SegmentDownload = 1,
    Upload = 2,
    /// Acknowledge a download command
    Download = 3,
    Abort = 4,
}

impl TryFrom<u8> for ServerCommand {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use ServerCommand::*;
        match value {
            0 => Ok(SegmentUpload),
            1 => Ok(SegmentDownload),
            2 => Ok(Upload),
            3 => Ok(Download),
            4 => Ok(Abort),
            _ => Err(()),
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
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
}


pub enum ClientCommand {
    DownloadSegment = 0,
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
            0 => Ok(DownloadSegment),
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

#[derive(Debug, Clone, Copy)]
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
    Abort {
        index: u16,
        sub: u8,
        abort_code: u32,
    }
}

impl SdoRequest {

    pub fn abort(index: u16, sub: u8, abort_code: AbortCode) -> Self {
        SdoRequest::Abort { index, sub, abort_code: abort_code as u32 }
    }

    /// Create an initiate download message
    pub fn initiate_download(index: u16, sub: u8, size: Option<u32>) -> Self {
        let data = size.unwrap_or(0).to_le_bytes();

        SdoRequest::InitiateDownload {
            n: 0,
            e: false,
            s: size.is_some(),
            index,
            sub,
            data,
        }
    }

    pub fn download_segment(toggle: bool, last_segment: bool, segment_data: &[u8]) -> Self {
        let mut data = [0; 7];
        data[0..segment_data.len()].copy_from_slice(segment_data);
        SdoRequest::DownloadSegment {
            t: toggle,
            n: 7 - segment_data.len() as u8,
            c: last_segment,
            data
        }
    }

    /// Create an expedited download message
    pub fn expedited_download(index: u16, sub: u8, data: &[u8]) -> Self {
        let mut msg_data = [0; 4];
        msg_data[0..data.len()].copy_from_slice(data);

        SdoRequest::InitiateDownload {
            n: (4 - data.len()) as u8,
            e: true,
            s: true,
            index,
            sub,
            data: msg_data
        }
    }

    pub fn initiate_upload(index: u16, sub: u8) -> Self {
        SdoRequest::InitiateUpload { index: index, sub: sub }
    }

    pub fn upload_segment_request(toggle: bool) -> Self {
        SdoRequest::ReqUploadSegment { t: toggle }
    }

    pub fn to_can_message(self, id: CanId) -> CanFdMessage {
        let mut payload = [0; 64];

        match self {
            SdoRequest::InitiateDownload {
                n,
                e,
                s,
                index,
                sub,
                data,
            } => {
                payload[0] =
                    (ClientCommand::InitiateDownload as u8) << 5 |
                    (n << 2) |
                    ((e as u8) << 1) |
                    s as u8;
                payload[1] = (index & 0xff) as u8;
                payload[2] = (index >> 8) as u8;
                payload[3] = sub;
                payload[4..8].copy_from_slice(&data);
            }
            SdoRequest::DownloadSegment { t, n, c, data } => {
                payload[0] =
                    (ClientCommand::DownloadSegment as u8) << 5 |
                    (t as u8) << 4 |
                    (n & 7) << 1 |
                    (c as u8);

                payload[1..8].copy_from_slice(&data);
            },
            SdoRequest::InitiateUpload { index, sub } => {
                payload[0] = (ClientCommand::InitiateUpload as u8) << 5;
                payload[1] = (index & 0xff) as u8;
                payload[2] = (index >> 8) as u8;
                payload[3] = sub;
            },
            SdoRequest::ReqUploadSegment { t } => {
                payload[0] = (ClientCommand::ReqUploadSegment as u8) << 5| (t as u8) << 4;
            },
            SdoRequest::Abort { index, sub, abort_code } => {
                payload[0] = (ClientCommand::Abort as u8) << 5;
                payload[1] = (index & 0xff) as u8;
                payload[2] = (index >> 8) as u8;
                payload[3] = sub;
                payload[4..8].copy_from_slice(&abort_code.to_le_bytes());
            }
            SdoRequest::InitiateBlockDownload { cc: _, s: _, cs: _, index: _, sub: _, size: _ } => todo!(),
            SdoRequest::InitiateBlockUpload {  } => todo!(),
        }

        CanFdMessage {
            data: payload,
            dlc: 8,
            id,
        }
    }
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
            ClientCommand::DownloadSegment => {
                let t = (value[0] & (1<<4)) != 0;
                let n = (value[0] >> 1) & 0x7;
                let c = (value[0] & (1<<0)) != 0;
                let data = value[1..8].try_into().unwrap();
                Ok(SdoRequest::DownloadSegment { t, n, c, data })
            },
            ClientCommand::InitiateDownload => {
                let n = (value[0] >> 2) & 0x3;
                let e = (value[0] & (1<<1)) != 0;
                let s = (value[0] & (1<<0)) != 0;
                let index = value[1] as u16 | ((value[2] as u16) << 8);
                let sub = value[3];
                let data = value[4..8].try_into().unwrap();
                Ok(SdoRequest::InitiateDownload {
                    n,
                    e,
                    s,
                    index,
                    sub,
                    data,
                })
            },
            ClientCommand::InitiateUpload => {
                let index = value[1] as u16 | ((value[2] as u16) << 8);
                let sub = value[3];
                Ok(SdoRequest::InitiateUpload { index, sub })
            }
            ClientCommand::ReqUploadSegment => {
                let t = (((value[0]) >> 4) & 1) != 0;
                Ok(SdoRequest::ReqUploadSegment { t })
            },
            ClientCommand::Abort => {
                let index = value[1] as u16 | ((value[2] as u16) << 8);
                let sub = value[3];
                let abort_code = u32::from_le_bytes(value[4..8].try_into().unwrap());
                Ok(SdoRequest::Abort { index, sub, abort_code })
            },
            ClientCommand::ReqBlockUpload => todo!(),
            ClientCommand::ReqBlockDownload => todo!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
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
    ConfirmDownload {
        index: u16,
        sub: u8,
    },
    ConfirmDownloadSegment {
        t: bool,
    },
    ConfirmBlockDownload {
        sc: bool,
    },
    Abort {
        index: u16,
        sub: u8,
        abort_code: u32,
    }
}


impl TryFrom<CanFdMessage> for SdoResponse {
    type Error = ();
    fn try_from(msg: CanFdMessage) -> Result<Self, Self::Error> {

        let scs = msg.data[0] >> 5;
        let command: ServerCommand = scs.try_into()?;
        match command {
            ServerCommand::SegmentUpload => {
                let t = (msg.data[0] & (1<<4)) != 0;
                let n = (msg.data[0] >> 1) & 7;
                let c = (msg.data[0] & (1<<0)) != 0;
                let data: [u8; 7] = msg.data[1..8].try_into().unwrap();

                Ok(SdoResponse::UploadSegment { t, n, c, data })
            },
            ServerCommand::SegmentDownload => {
                let t = (msg.data[0] & (1<<4)) != 0;
                Ok(SdoResponse::ConfirmDownloadSegment { t })
            }
            ServerCommand::Upload => {
                let n = (msg.data[0] >> 2) & 0x3;
                let e = (msg.data[0] & (1<<1)) != 0;
                let s = (msg.data[0] & (1<<0)) != 0;
                let index = u16::from_le_bytes(msg.data[1..3].try_into().unwrap());
                let sub = msg.data[3];
                let data: [u8; 4] = msg.data[4..8].try_into().unwrap();
                Ok(SdoResponse::ConfirmUpload { n, e, s, index, sub, data })
            },
            ServerCommand::Download => {
                let index = u16::from_le_bytes(msg.data[1..3].try_into().unwrap());
                let sub = msg.data[3];
                Ok(SdoResponse::ConfirmDownload { index, sub })
            }
            ServerCommand::Abort => {
                let index = u16::from_le_bytes(msg.data[1..3].try_into().unwrap());
                let sub = msg.data[3];
                let abort_code = u32::from_le_bytes(msg.data[4..8].try_into().unwrap());
                Ok(SdoResponse::Abort { index, sub, abort_code })
            },
        }
    }
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

    pub fn upload_acknowledge(index: u16, sub: u8, size: u32) -> SdoResponse {
        SdoResponse::ConfirmUpload {
            n: 0,
            e: false,
            s: true,
            index,
            sub,
            data: size.to_le_bytes()
        }
    }

    pub fn upload_segment(t: bool, c: bool, data: &[u8]) -> SdoResponse {
        let n = (7 - data.len()) as u8;
        let mut buf = [0; 7];
        buf[0..data.len()].copy_from_slice(data);
        SdoResponse::UploadSegment { t, n, c, data: buf }
    }

    pub fn download_acknowledge(index: u16, sub: u8) -> SdoResponse {
        SdoResponse::ConfirmDownload { index, sub }
    }

    pub fn download_segment_acknowledge(t: bool) -> SdoResponse {
        SdoResponse::ConfirmDownloadSegment { t }
    }

    pub fn abort(index: u16, sub: u8, abort_code: AbortCode) -> SdoResponse {
        let abort_code = abort_code as u32;
        SdoResponse::Abort { index, sub, abort_code }
    }

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
                    (ServerCommand::Upload as u8) << 5 | ((n & 0x3) << 2) | ((e as u8) << 1) | (s as u8);
                payload[1] = (index & 0xff) as u8;
                payload[2] = (index >> 8) as u8;
                payload[3] = sub;
                payload[4..8].copy_from_slice(&data);
            }
            SdoResponse::ConfirmDownload { index, sub } => {
                payload[0] = (ServerCommand::Download as u8) << 5;
                payload[1] = (index & 0xff) as u8;
                payload[2] = (index >> 8) as u8;
                payload[3] = sub;
            },
            SdoResponse::UploadSegment { t, n, c, data } => {
                payload[0] =
                    (ServerCommand::SegmentUpload as u8) << 5 |
                    (t as u8) << 4 |
                    n << 1 |
                    c as u8;
                payload[1..8].copy_from_slice(&data);
            },
            SdoResponse::ConfirmBlockDownload { sc: _ } => todo!(),
            SdoResponse::Abort { index, sub, abort_code } => {
                payload[0] = (ServerCommand::Abort as u8) << 5;
                payload[1] = (index & 0xff) as u8;
                payload[2] = (index >> 8) as u8;
                payload[3] = sub;
                payload[4..8].copy_from_slice(&abort_code.to_le_bytes());

            },
            SdoResponse::ConfirmDownloadSegment { t } => {
                payload[0] = (ServerCommand::SegmentDownload as u8) << 5 | (t as u8) << 4;
            },

        }
        CanFdMessage {
            data: payload,
            dlc: 8,
            id,
        }
    }
}
