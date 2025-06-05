use zencan_common::{
    objects::{
        find_object, AccessType, DataType, ODCallbackContext, ODEntry, ObjectRawAccess as _,
        PdoMapping, SubInfo,
    },
    sdo::AbortCode,
    AtomicCell, CanId,
};

/// Specifies the number of mapping parameters supported per PDO
///
/// Since we do not yet support CAN-FD, or sub-byte mapping, it's not possible to map more than 8
/// objects to a single PDO
const N_MAPPING_PARAMS: usize = 8;

/// Represents a single PDO state
#[derive(Debug)]
pub struct Pdo {
    /// The COB-ID used to send or receive this PDO
    pub cob_id: AtomicCell<CanId>,
    /// Indicates if the PDO is enabled
    pub valid: AtomicCell<bool>,
    /// If set, this PDO cannot be requested via RTR
    pub rtr_disabled: AtomicCell<bool>,
    /// Transmission type field (subindex 0x2)
    /// Determines when the PDO is sent/received
    ///
    /// 0 (unused): PDO is sent on receipt of SYNC, but only if the event has been triggered
    /// 1 - 240: PDO is sent on receipt of every Nth SYNC message
    /// 254: PDO is sent asynchronously on application request
    pub transmission_type: AtomicCell<u8>,
    /// Tracks the number of sync signals since this was last sent or received
    pub sync_counter: AtomicCell<u8>,
    /// Inhibit time for this PDO in us
    pub inhibit_time: AtomicCell<u16>,
    /// The last received data value for an RPDO
    pub buffered_value: AtomicCell<Option<[u8; 8]>>,
    /// Indicates how many of the values in mapping_params are valid
    ///
    /// This represents sub0 for the mapping object
    pub valid_maps: AtomicCell<u8>,
    /// The mapping parameters
    ///
    /// These specify which objects are
    pub mapping_params: [AtomicCell<u32>; N_MAPPING_PARAMS],
}

impl Default for Pdo {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdo {
    /// Create a new PDO object
    pub const fn new() -> Self {
        let cob_id = AtomicCell::new(CanId::Std(0));
        let valid = AtomicCell::new(false);
        let rtr_disabled = AtomicCell::new(false);
        let transmission_type = AtomicCell::new(0);
        let sync_counter = AtomicCell::new(0);
        let inhibit_time = AtomicCell::new(0);
        let buffered_value = AtomicCell::new(None);
        let valid_maps = AtomicCell::new(0);
        let mapping_params = [const { AtomicCell::new(0) }; N_MAPPING_PARAMS];
        Self {
            cob_id,
            valid,
            rtr_disabled,
            transmission_type,
            sync_counter,
            inhibit_time,
            buffered_value,
            valid_maps,
            mapping_params,
        }
    }

    /// This function should be called when a SYNC event occurs
    ///
    /// It will return true if the PDO should be sent in response to the SYNC event
    pub fn sync_update(&self) -> bool {
        if !self.valid.load() {
            return false;
        }

        let transmission_type = self.transmission_type.load();
        if transmission_type == 0 {
            // TODO: Figure out how to determine application "event" which triggers the PDO
            // For now, send every sync
            true
        } else if transmission_type <= 240 {
            let cnt = self.sync_counter.fetch_add(1) + 1;
            cnt == transmission_type
        } else {
            false
        }
    }

    /// Check mapped objects for TPDO event flag
    pub fn read_events(&self, od: &[ODEntry]) -> bool {
        // TODO: Should maybe cache pointers or something. This is searching the whole OD for every
        // mapped object
        if !self.valid.load() {
            return false;
        }

        for i in 0..self.mapping_params.len() {
            let param = self.mapping_params[i].load();
            if param == 0 {
                break;
            }
            let object_id = (param >> 16) as u16;
            let sub_index = ((param & 0xFF00) >> 8) as u8;
            // Unwrap safety: Object is validated to exist prior to setting mapping
            let entry = find_object(od, object_id).expect("invalid mapping parameter");
            if entry.read_event_flag(sub_index) {
                return true;
            }
        }
        false
    }

    pub(crate) fn clear_events(&self, od: &[ODEntry]) {
        for i in 0..self.mapping_params.len() {
            let param = self.mapping_params[i].load();
            if param == 0 {
                break;
            }
            let object_id = (param >> 16) as u16;
            // Unwrap safety: Object is validated to exist prior to setting mapping
            let entry = find_object(od, object_id).expect("invalid mapping parameter");
            entry.clear_events();
        }
    }
}

pub(crate) fn pdo_comm_write_callback(
    ctx: &ODCallbackContext,
    sub: u8,
    offset: usize,
    buf: &[u8],
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx
        .ctx
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

pub(crate) fn pdo_comm_read_callback(
    ctx: &ODCallbackContext,
    sub: u8,
    offset: usize,
    buf: &mut [u8],
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx
        .ctx
        .unwrap()
        .as_any()
        .downcast_ref()
        .expect("invalid context type in pdo_comm_read_callback");

    match sub {
        0 => {
            if offset != 0 {
                return Err(AbortCode::UnsupportedAccess);
            }
            if buf.len() != 1 {
                return Err(AbortCode::DataTypeMismatch);
            }
            buf[0] = 2;
            Ok(())
        }
        1 => {
            if offset + buf.len() > 4 {
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

            let bytes = value.to_le_bytes();
            buf.copy_from_slice(&bytes[offset..offset + buf.len()]);
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

pub(crate) fn pdo_comm_info_callback(
    _ctx: &ODCallbackContext,
    sub: u8,
) -> Result<SubInfo, AbortCode> {
    match sub {
        0 => Ok(SubInfo {
            data_type: DataType::UInt8,
            size: 1,
            access_type: AccessType::Ro,
            pdo_mapping: PdoMapping::None,
            persist: false,
        }),
        1 => Ok(SubInfo {
            data_type: DataType::UInt32,
            size: 4,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: true,
        }),
        2 => Ok(SubInfo {
            data_type: DataType::UInt8,
            size: 1,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: true,
        }),
        _ => Err(AbortCode::NoSuchSubIndex),
    }
}

pub(crate) fn pdo_mapping_write_callback(
    ctx: &ODCallbackContext,
    sub: u8,
    offset: usize,
    buf: &[u8],
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx
        .ctx
        .unwrap()
        .as_any()
        .downcast_ref()
        .expect("invalid context type in pdo_comm_read_callback");

    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess);
    }

    if sub == 0 {
        pdo.valid_maps.store(buf[0]);
        Ok(())
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
        let entry = find_object(ctx.od, object_id).ok_or(AbortCode::NoSuchObject)?;
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

pub(crate) fn pdo_mapping_read_callback(
    ctx: &ODCallbackContext,
    sub: u8,
    offset: usize,
    buf: &mut [u8],
) -> Result<(), AbortCode> {
    let pdo: &Pdo = ctx
        .ctx
        .unwrap()
        .as_any()
        .downcast_ref()
        .expect("invalid context type in pdo_comm_read_callback");

    if sub == 0 {
        if offset != 0 || buf.len() != 1 {
            return Err(AbortCode::DataTypeMismatch);
        }
        buf[0] = pdo.valid_maps.load();
        Ok(())
    } else if sub <= pdo.mapping_params.len() as u8 {
        if offset + buf.len() > 4 {
            return Err(AbortCode::DataTypeMismatchLengthHigh);
        }
        let value = pdo.mapping_params[(sub - 1) as usize].load();
        let bytes = value.to_le_bytes();
        buf.copy_from_slice(&bytes[offset..offset + buf.len()]);
        Ok(())
    } else {
        Err(AbortCode::NoSuchSubIndex)
    }
}

pub(crate) fn pdo_mapping_info_callback(
    ctx: &ODCallbackContext,
    sub: u8,
) -> Result<SubInfo, AbortCode> {
    let pdo: &Pdo = ctx
        .ctx
        .unwrap()
        .as_any()
        .downcast_ref()
        .expect("invalid context type in pdo_comm_read_callback");
    if sub == 0 {
        Ok(SubInfo {
            size: 1,
            data_type: DataType::UInt8,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: true,
        })
    } else if sub <= pdo.mapping_params.len() as u8 {
        Ok(SubInfo {
            size: 4,
            data_type: DataType::UInt32,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: true,
        })
    } else {
        Err(AbortCode::NoSuchSubIndex)
    }
}

pub(crate) fn store_pdo_data(data: &[u8], pdo: &Pdo, od: &[ODEntry]) {
    let mut offset = 0;
    for i in 0..pdo.mapping_params.len() {
        let param = pdo.mapping_params[i].load();
        if param == 0 {
            break;
        }
        let object_id = (param >> 16) as u16;
        let sub_index = ((param & 0xFF00) >> 8) as u8;
        // Rounding up to BYTES, because we do not currently support bit access
        let length = (param & 0xFF).div_ceil(8) as usize;
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

pub(crate) fn read_pdo_data(data: &mut [u8], pdo: &Pdo, od: &[ODEntry]) {
    let mut offset = 0;
    for i in 0..pdo.mapping_params.len() {
        let param = pdo.mapping_params[i].load();
        if param == 0 {
            break;
        }
        let object_id = (param >> 16) as u16;
        let sub_index = ((param & 0xFF00) >> 8) as u8;
        // Rounding up to BYTES, because we do not currently support bit access
        let length = (param & 0xFF).div_ceil(8) as usize;
        let entry = find_object(od, object_id).expect("Invalid mapping parameter");
        if offset + length > data.len() {
            break;
        }
        // There's no mechanism to report an error here, so we just ignore it if it fails. We can
        // check that the PDO mapping is valid when it is written to the object dictionary, to make
        // it impossible to fail.
        entry
            .read(sub_index, 0, &mut data[offset..offset + length])
            .ok();
        offset += length;
    }
}
