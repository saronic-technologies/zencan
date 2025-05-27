//! Handling for persistent storage control objects
//!
//!

use core::convert::Infallible;

use zencan_common::{
    objects::{ODCallbackContext, SubInfo},
    sdo::AbortCode,
    AtomicCell,
};

pub type StoreObjectsCallback =
    dyn Fn(&mut dyn embedded_io::Read<Error = Infallible>, usize) + Sync;


#[derive(Default)]
pub struct StorageContext {
    pub(crate) store_callback: AtomicCell<Option<&'static StoreObjectsCallback>>,
}

impl StorageContext {
    pub const fn new() -> Self {
        Self {
            store_callback: AtomicCell::new(None),
        }
    }
}


/// Magic value used to trigger object storage
const SAVE_CMD: u32 = 0x73617665;

pub(crate) fn handle_1010_write(
    ctx: &ODCallbackContext,
    sub: u8,
    offset: usize,
    buf: &[u8],
) -> Result<(), AbortCode> {
    match sub {
        0 => {
            println!("1010 sub0 write");
            Err(AbortCode::ReadOnly)
        }
        1 => {
            if offset != 0 || buf.len() != 4 {
                Err(AbortCode::DataTypeMismatch)
            } else {
                let value = u32::from_le_bytes(buf[0..4].try_into().unwrap());
                // Magic value ('save') triggering a save
                if value == SAVE_CMD {
                    let storage: &StorageContext = ctx
                        .ctx
                        .unwrap()
                        .as_any()
                        .downcast_ref()
                        .expect("invalid context type in handle_1010_write");
                    if let Some(cb) = storage.store_callback.load() {
                        crate::persist::serialize(ctx.od, cb);
                        Ok(())
                    } else {
                        Err(AbortCode::CantStore)
                    }
                } else {
                    Err(AbortCode::CantStore)
                }
            }
        }
        _ => Err(AbortCode::NoSuchSubIndex),
    }
}

pub(crate) fn handle_1010_read(
    ctx: &ODCallbackContext,
    sub: u8,
    offset: usize,
    buf: &mut [u8],
) -> Result<(), AbortCode> {
    match sub {
        0 => {
            if offset != 0 || buf.len() != 1 {
                Err(AbortCode::DataTypeMismatch)
            } else {
                buf[0] = 1;
                Ok(())
            }
        }
        1 => {
            if offset != 0 || buf.len() != 4 {
                Err(AbortCode::DataTypeMismatch)
            } else {
                let storage: &StorageContext = ctx
                    .ctx
                    .unwrap()
                    .as_any()
                    .downcast_ref()
                    .expect("invalid context type in handle_1010_read");
                // Bit 0 indicates the node is capable of saving objects
                let mut value = 0u32;
                if storage.store_callback.load().is_some() {
                    value |= 1;
                }
                buf.copy_from_slice(&value.to_le_bytes());
                Ok(())
            }
        }
        _ => Err(AbortCode::NoSuchSubIndex),
    }
}

pub(crate) fn handle_1010_subinfo(_ctx: &ODCallbackContext, sub: u8) -> Result<SubInfo, AbortCode> {
    match sub {
        0 => Ok(SubInfo::MAX_SUB_NUMBER),
        1 => Ok(SubInfo::new_u32().rw_access()),
        _ => Err(AbortCode::NoSuchSubIndex),
    }
}
