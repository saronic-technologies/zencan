//! Bootloader objects
//!
//!

use core::sync::atomic::{AtomicBool, Ordering};

use zencan_common::{
    constants::values::BOOTLOADER_ERASE_CMD,
    objects::{ODEntry, ObjectCode, ObjectRawAccess, SubInfo},
    sdo::AbortCode,
    AtomicCell,
};

/// Implements a Bootloader info (0x5500) object
pub struct BootloaderInfo<'a, const APP: bool, const NUM_SECTIONS: u8> {
    reset_flag: AtomicBool,
    od: &'a [ODEntry<'a>],
}

impl<'a, const APP: bool, const NUM_SECTIONS: u8> BootloaderInfo<'a, APP, NUM_SECTIONS> {
    pub const fn new(od: &'a [ODEntry<'a>]) -> Self {
        Self {
            reset_flag: AtomicBool::new(false),
            od,
        }
    }

    pub fn reset_flag(&self) -> bool {
        return self.reset_flag.load(Ordering::Relaxed);
    }
}

impl<const APP: bool, const NUM_SECTIONS: u8> ObjectRawAccess
    for BootloaderInfo<'_, APP, NUM_SECTIONS>
{
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        if offset != 0 {
            return Err(AbortCode::UnsupportedAccess);
        }
        match sub {
            0 => {
                if buf.len() > 1 {
                    return Err(AbortCode::DataTypeMismatchLengthHigh);
                }
                buf[0] = 3;
                Ok(())
            }
            1 => {
                if buf.len() > 4 {
                    return Err(AbortCode::DataTypeMismatchLengthHigh);
                } else if buf.len() < 4 {
                    return Err(AbortCode::DataTypeMismatchLengthLow);
                }

                let mut config = 1u32;
                if APP {
                    config |= 1 << 1;
                }
                buf[0..4].copy_from_slice(&config.to_le_bytes());
                Ok(())
            }
            2 => {
                if buf.len() > 1 {
                    return Err(AbortCode::DataTypeMismatchLengthHigh);
                }
                buf[0] = NUM_SECTIONS;
                Ok(())
            }
            3 => Err(AbortCode::WriteOnly),
            _ => Err(AbortCode::NoSuchSubIndex),
        }
    }

    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
        match sub {
            0 | 1 | 2 => Err(AbortCode::ReadOnly),
            3 => {
                if !APP {
                    return Err(AbortCode::UnsupportedAccess);
                } else if data.len() == 4 && offset == 0 && data == &[0x42, 0x4F, 0x4F, 0x54] {
                    Ok(self.reset_flag.store(true, Ordering::Relaxed))
                } else {
                    Err(AbortCode::InvalidValue)
                }
            }
            _ => Err(AbortCode::NoSuchSubIndex),
        }
    }

    fn object_code(&self) -> ObjectCode {
        ObjectCode::Record
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        match sub {
            0 => Ok(SubInfo::MAX_SUB_NUMBER),
            1 => Ok(SubInfo::new_u32().ro_access()),
            2 => Ok(SubInfo::new_u8().ro_access()),
            3 => Ok(SubInfo::new_u32().wo_access()),
            _ => Err(AbortCode::NoSuchSubIndex),
        }
    }
}

pub trait BootloaderSectionCallbacks: Sync {
    /// Called to erase the section
    ///
    /// Returns true if section is successfully erased and ready for programming
    fn erase(&self) -> bool;

    /// Write a chunk of data
    ///
    /// Write will be called 1 or more times after an erase with a sequence of new data to write to
    /// the section
    fn write(&self, data: &[u8]);

    /// Finalize writing a section
    ///
    /// Will be called once after all data has been written to allow the storage driver to finalize
    /// writing the data and return any errors.
    ///
    /// Returns true on successful write
    fn finalize(&self) -> bool;
}

pub struct BootloaderSection {
    name: &'static str,
    size: u32,
    callbacks: AtomicCell<Option<&'static dyn BootloaderSectionCallbacks>>,
}

impl BootloaderSection {
    pub const fn new(name: &'static str, size: u32) -> Self {
        Self {
            name,
            size,
            callbacks: AtomicCell::new(None),
        }
    }

    pub fn register_callbacks(&self, callbacks: &'static dyn BootloaderSectionCallbacks) {
        self.callbacks.store(Some(callbacks));
    }
}

fn read_u8(value: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
    if offset != 0 {
        return Err(AbortCode::UnsupportedAccess);
    }
    if buf.len() != 1 {
        return Err(AbortCode::DataTypeMismatchLengthHigh);
    }
    buf[0] = value;
    Ok(())
}

fn read_str(value: &str, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
    let read_len = buf.len().min(value.len() - offset);
    buf[0..read_len].copy_from_slice(&value.as_bytes()[offset..]);
    Ok(())
}

impl ObjectRawAccess for BootloaderSection {
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        match sub {
            0 => read_u8(4, offset, buf),
            1 => read_u8(1, offset, buf),
            2 => read_str(&self.name, offset, buf),
            3 => Err(AbortCode::WriteOnly),
            4 => Err(AbortCode::WriteOnly),
            _ => Err(AbortCode::NoSuchSubIndex),
        }
    }

    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
        match sub {
            0 => Err(AbortCode::ReadOnly),
            1 => Err(AbortCode::ReadOnly),
            2 => Err(AbortCode::ReadOnly),
            3 => {
                if data == BOOTLOADER_ERASE_CMD.to_le_bytes() {
                    if let Some(cb) = self.callbacks.load() {
                        if cb.erase() {
                            Ok(())
                        } else {
                            Err(AbortCode::GeneralError)
                        }
                    } else {
                        Err(AbortCode::ResourceNotAvailable)
                    }
                } else {
                    Err(AbortCode::InvalidValue)
                }
            }
            _ => Err(AbortCode::NoSuchSubIndex),
        }
    }

    fn object_code(&self) -> ObjectCode {
        todo!()
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        todo!()
    }
}
