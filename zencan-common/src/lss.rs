use crate::{messages::MessageError, traits::CanId};

#[derive(Debug, Clone, Copy)]
pub enum LssCommandSpecifier {
    SwitchModeGlobal = 0x04,
    ConfigureNodeId = 0x11,
    ConfigureBitTiming = 0x13,
    ActivateBitTiming = 0x15,
    StoreConfiguration = 0x17,
    SwitchStateVendor = 0x40,
    SwitchStateProduct = 0x41,
    SwitchStateRevr = 0x42,
    SwitchStateSerial = 0x43,
    SwitchStateResponse = 0x44,
    IdentifyVendor = 0x46,
    IdentifyProduct = 0x47,
    IdentifyRevLow = 0x48,
    IdentifyRevHigh = 0x49,
    IdentifySerialNumberLow = 0x4A,
    IdentifySlave = 0x4F,
    FastScan = 0x51,
    InquireVendor = 0x5A,
    InquireProduct = 0x5B,
    InquireRev = 0x5C,
    InquireSerial = 0x5D,
    InquireNodeId = 0x5E,
}

/// Special value for fastscan bit_check field
pub const LSS_FASTSCAN_CONFIRM: u8 = 0x80;

impl LssCommandSpecifier {
    pub fn from_byte(b: u8) -> Result<Self, MessageError> {
        match b {
            0x04 => Ok(Self::SwitchModeGlobal),
            0x11 => Ok(Self::ConfigureNodeId),
            0x13 => Ok(Self::ConfigureBitTiming),
            0x15 => Ok(Self::ActivateBitTiming),
            0x17 => Ok(Self::StoreConfiguration),
            0x40 => Ok(Self::SwitchStateVendor),
            0x41 => Ok(Self::SwitchStateProduct),
            0x42 => Ok(Self::SwitchStateRevr),
            0x43 => Ok(Self::SwitchStateSerial),
            0x44 => Ok(Self::SwitchStateResponse),
            0x5A => Ok(Self::InquireVendor),
            0x5B => Ok(Self::InquireProduct),
            0x5C => Ok(Self::InquireRev),
            0x5D => Ok(Self::InquireSerial),
            0x5E => Ok(Self::InquireNodeId),
            _ => Err(MessageError::UnexpectedLssCommand(b)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LssRequest {
    SwitchModeGlobal {
        mode: u8,
    },
    ConfigureNodeId {
        /// The new node ID to set
        node_id: u8,
    },
    ConfigureBitTiming {
        /// Defines what baudrate table is used to lookup the bit timing
        /// 0 means use the default table
        /// 1..127 are reserved
        /// 128..255 are user definable
        ///
        /// The default table is:
        /// - 0: 1MBit/s
        /// - 1: 800kBit/s
        /// - 2: 500kBit/s
        /// - 3: 250kBit/s
        /// - 4: 125kBit/s
        /// - 5: 100kBit/s
        /// - 6: 50kBit/sCS_IDENTIFY_REMOTE_SLAVE_SERIAL_NUMBER_LOW
        /// - 7: 20kBit/s
        /// - 8: 10kBit/s
        table: u8,
        /// The index into the baudrate table for the baudrate to select
        index: u8,
    },
    ActivateBitTiming {
        /// Duration in ms to delay before activating the new baudrate
        delay: u16,
    },
    SwitchStateVendor {
        /// The vendor ID to match against (32-bit value)
        vendor_id: u32,
    },
    SwitchStateProduct {
        /// The product code to match against (32-bit value)
        product_code: u32,
    },
    SwitchStateRevision {
        /// The revision number to match against (32-bit value)
        revision: u32,
    },
    SwitchStateSerial {
        /// The serial number to match against (32-bit value)
        serial: u32,
    },

    FastScan {
        id: u32,
        bit_check: u8,
        /// The sub index of the identity to check
        /// 0 - Vendor ID
        /// 1 - Product Code
        /// 2 - Revision
        /// 3 - Serial Number
        sub: u8,
        /// The sub index of the identity to check on the next FastScan request
        next: u8,
    },
}

impl TryFrom<&[u8]> for LssRequest {
    type Error = MessageError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 1 {
            return Err(MessageError::MessageTooShort);
        }
        let cs = LssCommandSpecifier::from_byte(value[0])?;
        match cs {
            LssCommandSpecifier::SwitchModeGlobal => {
                if value.len() < 2 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::SwitchModeGlobal { mode: value[1] })
            }
            LssCommandSpecifier::ConfigureNodeId => todo!(),
            LssCommandSpecifier::ConfigureBitTiming => todo!(),
            LssCommandSpecifier::ActivateBitTiming => todo!(),
            LssCommandSpecifier::StoreConfiguration => todo!(),
            LssCommandSpecifier::SwitchStateVendor => todo!(),
            LssCommandSpecifier::SwitchStateProduct => todo!(),
            LssCommandSpecifier::SwitchStateRevr => todo!(),
            LssCommandSpecifier::SwitchStateSerial => todo!(),
            LssCommandSpecifier::SwitchStateResponse => todo!(),
            LssCommandSpecifier::IdentifyVendor => todo!(),
            LssCommandSpecifier::IdentifyProduct => todo!(),
            LssCommandSpecifier::IdentifyRevLow => todo!(),
            LssCommandSpecifier::IdentifyRevHigh => todo!(),
            LssCommandSpecifier::IdentifySerialNumberLow => todo!(),
            LssCommandSpecifier::IdentifySlave => todo!(),
            LssCommandSpecifier::FastScan => {
                if value.len() < 8 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::FastScan {
                    id: u32::from_le_bytes([value[1], value[2], value[3], value[4]]),
                    bit_check: value[5],
                    sub: value[6],
                    next: value[7],
                })
            }
            LssCommandSpecifier::InquireVendor => todo!(),
            LssCommandSpecifier::InquireProduct => todo!(),
            LssCommandSpecifier::InquireRev => todo!(),
            LssCommandSpecifier::InquireSerial => todo!(),
            LssCommandSpecifier::InquireNodeId => todo!(),

        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LssResponse {
    IdentifySlave,
}

impl TryFrom<&[u8]> for LssResponse {
    type Error = MessageError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 1 {
            return Err(MessageError::MessageTooShort);
        }
        let cs = LssCommandSpecifier::from_byte(value[0])?;
        match cs {
            LssCommandSpecifier::IdentifySlave => Ok(Self::IdentifySlave {}),
            _ => Err(MessageError::UnexpectedLssCommand(value[0])),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum LssState {
    Waiting = 0,
    Configuring = 1,
}

impl LssState {
    pub fn from_byte(b: u8) -> Result<Self, MessageError> {
        match b {
            0x00 => Ok(Self::Waiting),
            0x01 => Ok(Self::Configuring),
            _ => Err(MessageError::InvalidField),
        }
    }
}

pub struct LssIdentity {
    pub vendor_id: u32,
    pub product_code: u32,
    pub revision: u32,
    pub serial_number: u32,
}

impl LssIdentity {
    pub fn new(vendor_id: u32, product_code: u32, revision: u32, serial_number: u32) -> Self {
        Self {
            vendor_id,
            product_code,
            revision,
            serial_number,
        }
    }

    pub fn by_addr(&self, addr: u8) -> u32 {
        match addr {
            0 => self.vendor_id,
            1 => self.product_code,
            2 => self.revision,
            3 => self.serial_number,
            _ => panic!("Invalid LSS identity address"),
        }
    }
}

