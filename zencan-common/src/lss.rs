use crate::{messages::MessageError, traits::{CanFdMessage, CanId}};

#[derive(Debug, Clone, Copy)]
pub enum LssCommandSpecifier {
    SwitchModeGlobal = 0x04,
    ConfigureNodeId = 0x11,
    ConfigureBitTiming = 0x13,
    ActivateBitTiming = 0x15,
    StoreConfiguration = 0x17,
    SwitchStateVendor = 0x40,
    SwitchStateProduct = 0x41,
    SwitchStateRev = 0x42,
    SwitchStateSerial = 0x43,
    SwitchStateResponse = 0x44,
    IdentifySlave = 0x4F,
    FastScan = 0x51,
    InquireVendor = 0x5A,
    InquireProduct = 0x5B,
    InquireRev = 0x5C,
    InquireSerial = 0x5D,
    InquireNodeId = 0x5E,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum LssConfigureError {
    Ok = 0,
    NodeIdOutOfRange = 1,
    Manufacturer = 0xff,
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
            0x42 => Ok(Self::SwitchStateRev),
            0x43 => Ok(Self::SwitchStateSerial),
            0x44 => Ok(Self::SwitchStateResponse),
            0x4F => Ok(Self::IdentifySlave),
            0x51 => Ok(Self::FastScan),
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
        /// - 6: 50kBit/s
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
    /// Request the vendor ID from a node in LSS Configuring state
    InquireVendor,
    /// Request the product code from a node in LSS Configuring state
    InquireProduct,
    /// Request the revision from a node in LSS Configuring state
    InquireRev,
    /// Request the serial number from a node in LSS Configuring state
    InquireSerial,
    /// Request the node ID from a node in LSS Configuring state
    InquireNodeId,

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
            LssCommandSpecifier::ConfigureNodeId => {
                if value.len() < 2 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::ConfigureNodeId {
                    node_id: value[1],
                })
            },
            LssCommandSpecifier::ConfigureBitTiming => {
                if value.len() < 3 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::ConfigureBitTiming {
                    table: value[1],
                    index: value[2],
                })
            },
            LssCommandSpecifier::ActivateBitTiming => {
                if value.len() < 3 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::ActivateBitTiming {
                    delay: u16::from_le_bytes([value[1], value[2]]),
                })
            },
            LssCommandSpecifier::StoreConfiguration => todo!(),
            LssCommandSpecifier::SwitchStateVendor => {
                if value.len() < 5 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::SwitchStateVendor {
                    vendor_id: u32::from_le_bytes(value[1..5].try_into().unwrap()),
                })
            },
            LssCommandSpecifier::SwitchStateProduct => {
                if value.len() < 5 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::SwitchStateProduct {
                    product_code: u32::from_le_bytes(value[1..5].try_into().unwrap()),
                })
            },
            LssCommandSpecifier::SwitchStateRev => {
                if value.len() < 5 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::SwitchStateRevision {
                    revision: u32::from_le_bytes(value[1..5].try_into().unwrap()),
                })
            },
            LssCommandSpecifier::SwitchStateSerial => {
                if value.len() < 5 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::SwitchStateSerial {
                    serial: u32::from_le_bytes(value[1..5].try_into().unwrap()),
                })
            },
            LssCommandSpecifier::SwitchStateResponse => {
                if value.len() < 5 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::SwitchStateVendor {
                    vendor_id: u32::from_le_bytes(value[1..5].try_into().unwrap()),
                })
            },
            // IdentifySlave is only used in a response
            LssCommandSpecifier::IdentifySlave => Err(MessageError::UnexpectedLssCommand(value[0])),
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
            LssCommandSpecifier::InquireVendor => Ok(LssRequest::InquireVendor),
            LssCommandSpecifier::InquireProduct => Ok(LssRequest::InquireProduct),
            LssCommandSpecifier::InquireRev => Ok(LssRequest::InquireRev),
            LssCommandSpecifier::InquireSerial => Ok(LssRequest::InquireSerial),
            LssCommandSpecifier::InquireNodeId => Ok(LssRequest::InquireNodeId),

        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LssResponse {
    IdentifySlave,
    SwitchStateResponse,
    ConfigureNodeIdAck {
        error: u8,
        spec_error: u8,
    },
    ConfigureBitTimingAck {
        error: u8,
        spec_error: u8,
    },
    InquireVendorAck {
        vendor_id: u32,
    },
    InquireProductAck {
        product_code: u32,
    },
    InquireRevAck {
        revision: u32,
    },
    InquireSerialAck {
        serial_number: u32,
    },
    InquireNodeIdAck {
        node_id: u8,
    },

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
            LssCommandSpecifier::SwitchStateResponse => Ok(Self::SwitchStateResponse {}),
            LssCommandSpecifier::ConfigureNodeId => {
                if value.len() < 3 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::ConfigureNodeIdAck {
                    error: value[1],
                    spec_error: value[2],
                })
            }
            LssCommandSpecifier::ConfigureBitTiming => {
                if value.len() < 3 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::ConfigureBitTimingAck {
                    error: value[1],
                    spec_error: value[2],
                })
            }
            LssCommandSpecifier::InquireVendor => {
                if value.len() < 5 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::InquireVendorAck {
                    vendor_id: u32::from_le_bytes([value[1], value[2], value[3], value[4]]),
                })
            }
            LssCommandSpecifier::InquireProduct => {
                if value.len() < 5 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::InquireProductAck {
                    product_code: u32::from_le_bytes([value[1], value[2], value[3], value[4]]),
                })
            }
            LssCommandSpecifier::InquireRev => {
                if value.len() < 5 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::InquireRevAck {
                    revision: u32::from_le_bytes([value[1], value[2], value[3], value[4]]),
                })
            }
            LssCommandSpecifier::InquireSerial => {
                if value.len() < 5 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::InquireSerialAck {
                    serial_number: u32::from_le_bytes([value[1], value[2], value[3], value[4]]),
                })
            }
            LssCommandSpecifier::InquireNodeId => {
                if value.len() < 2 {
                    return Err(MessageError::MessageTooShort);
                }
                Ok(Self::InquireNodeIdAck {
                    node_id: value[1],
                })
            }
            _ => Err(MessageError::UnexpectedLssCommand(value[0])),
        }
    }
}

impl LssResponse {
    pub fn to_can_message(self: &LssResponse, id: CanId) -> CanFdMessage {
        // LSS messages are required to always be 8 bytes long. For...some reason.
        let mut msg = CanFdMessage::new(id, &[0; 8]);
        match self {
            LssResponse::IdentifySlave => {
                msg.data[0] = LssCommandSpecifier::IdentifySlave as u8;
            }
            LssResponse::SwitchStateResponse => {
                msg.data[0] = LssCommandSpecifier::SwitchStateResponse as u8;
            }
            LssResponse::ConfigureNodeIdAck { error, spec_error } => {
                msg.data[0] = LssCommandSpecifier::ConfigureNodeId as u8;
                msg.data[1] = *error;
                msg.data[2] = *spec_error;
            }
            LssResponse::ConfigureBitTimingAck { error, spec_error } => {
                msg.data[0] = LssCommandSpecifier::ConfigureBitTiming as u8;
                msg.data[1] = *error;
                msg.data[2] = *spec_error;
            }
            LssResponse::InquireVendorAck { vendor_id } => {
                msg.data[0] = LssCommandSpecifier::InquireVendor as u8;
                msg.data[1..5].copy_from_slice(&vendor_id.to_le_bytes());
            }
            LssResponse::InquireProductAck { product_code } => {
                msg.data[0] = LssCommandSpecifier::InquireProduct as u8;
                msg.data[1..5].copy_from_slice(&product_code.to_le_bytes());
            }
            LssResponse::InquireRevAck { revision } => {
                msg.data[0] = LssCommandSpecifier::InquireRev as u8;
                msg.data[1..5].copy_from_slice(&revision.to_le_bytes());
            }
            LssResponse::InquireSerialAck { serial_number } => {
                msg.data[0] = LssCommandSpecifier::InquireSerial as u8;
                msg.data[1..5].copy_from_slice(&serial_number.to_le_bytes());
            }
            LssResponse::InquireNodeIdAck { node_id } => {
                msg.data[0] = LssCommandSpecifier::InquireNodeId as u8;
                msg.data[1] = *node_id;
            }
        }
        msg
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

#[derive(Debug, Clone, Copy, PartialEq)]
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

