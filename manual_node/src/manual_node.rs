//! A hand-coded example node instantiation
//!
//! Normally, this would be auto-generated from an EDS file using the
//! zencan-build crate. But this is here to provide an example of what the
//! generated code looks like and facilitate standalone tests w/o requiring
//! zencan-build.

use std::cell::RefCell;

use critical_section::Mutex;
use crossbeam::atomic::AtomicCell;
use zencan_common::{
    objects::{
        AccessType, ArrayObject, CallbackObject, DataType, ODEntry, ObjectCode, ObjectData, ObjectRawAccess, RecordObject, SubInfo, VarObject
    },
    sdo::AbortCode,
};

// a record type object
pub struct Object1000 {
    pub sub1: AtomicCell<u32>,
    // Sub 2 intentionally missing. It's not required for a record to implement every sub index
    pub sub3: AtomicCell<f32>,
}

// Goal is that this impl can be created by a macro
impl Object1000 {
    pub fn set_sub1(&self, value: u32) {
        self.sub1.store(value);
    }
    pub fn get_sub1(&self) -> u32 {
        self.sub1.load()
    }
    pub fn set_sub3(&self, value: f32) {
        self.sub3.store(value);
    }
    pub fn get_sub3(&self) -> f32 {
        self.sub3.load()
    }
}

impl ObjectRawAccess for Object1000 {
    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
        match sub {
            0 => Err(AbortCode::ReadOnly),
            1 => {
                let value = u32::from_le_bytes(data.try_into().map_err(|_| {
                    if data.len() < size_of::<u32>() {
                        AbortCode::DataTypeMismatchLengthLow
                    } else {
                        AbortCode::DataTypeMismatchLengthHigh
                    }
                })?);
                self.set_sub1(value);
                Ok(())
            }
            3 => {
                let value = f32::from_le_bytes(data.try_into().map_err(|_| {
                    if data.len() < size_of::<f32>() {
                        AbortCode::DataTypeMismatchLengthLow
                    } else {
                        AbortCode::DataTypeMismatchLengthHigh
                    }
                })?);
                self.set_sub3(value);
                Ok(())
            }
            _ => Err(AbortCode::NoSuchSubIndex),
        }
    }
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        match sub {
            0 => {
                buf[0] = 3;
                Ok(())
            }
            1 => {
                let value = self.get_sub1();
                if buf.len() < size_of::<u32>() {
                    return Err(AbortCode::DataTypeMismatchLengthLow);
                }
                if buf.len() > size_of::<u32>() {
                    return Err(AbortCode::DataTypeMismatchLengthHigh);
                }
                buf.copy_from_slice(&value.to_le_bytes());
                Ok(())
            }
            3 => {
                let value = self.get_sub3();
                if buf.len() < size_of::<f32>() {
                    return Err(AbortCode::DataTypeMismatchLengthLow);
                }
                if buf.len() > size_of::<f32>() {
                    return Err(AbortCode::DataTypeMismatchLengthHigh);
                }
                buf.copy_from_slice(&value.to_le_bytes());
                Ok(())
            }
            _ => Err(AbortCode::NoSuchSubIndex),
        }
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        match sub {
            0 => Ok(SubInfo {
                access_type: AccessType::Ro,
                data_type: DataType::UInt8,
                size: 1,
            }),
            1 => Ok(SubInfo {
                access_type: AccessType::Rw,
                data_type: DataType::UInt32,
                size: size_of::<u32>(),
            }),
            3 => Ok(SubInfo {
                access_type: AccessType::Rw,
                data_type: DataType::Real32,
                size: size_of::<f32>(),
            }),
            _ => Err(AbortCode::NoSuchSubIndex),
        }
    }
}

pub struct Object2000 {
    pub array: Mutex<RefCell<[u16; 10]>>,
}

impl Object2000 {
    pub fn set_idx(&self, idx: usize, value: u16) -> Result<(), AbortCode> {
        if idx < 10 {
            critical_section::with(|cs| {
                let mut array = self.array.borrow_ref_mut(cs);
                array[idx] = value;
            });
            Ok(())
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    pub fn get_idx(&self, idx: usize) -> Result<u16, AbortCode> {
        if idx < 10 {
            critical_section::with(|cs| {
                let array = self.array.borrow_ref(cs);
                Ok(array[idx])
            })
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }
}

impl ObjectRawAccess for Object2000 {
    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
        if sub < 10 {
            let idx = sub as usize;
            let value = u16::from_le_bytes(data.try_into().map_err(|_| {
                if data.len() < size_of::<u16>() {
                    AbortCode::DataTypeMismatchLengthLow
                } else {
                    AbortCode::DataTypeMismatchLengthHigh
                }
            })?);
            self.set_idx(idx, value)
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        if sub < 10 {
            let idx = sub as usize;
            let value = self.get_idx(idx)?;
            if buf.len() < size_of::<u16>() {
                return Err(AbortCode::DataTypeMismatchLengthLow);
            }
            if buf.len() > size_of::<u16>() {
                return Err(AbortCode::DataTypeMismatchLengthHigh);
            }
            buf.copy_from_slice(&value.to_le_bytes());
            Ok(())
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        if sub == 0 {
            Ok(SubInfo {
                access_type: AccessType::Ro,
                data_type: DataType::UInt8,
                size: 1,
            })
        } else if sub < 10 {
            Ok(SubInfo {
                access_type: AccessType::Rw,
                data_type: DataType::UInt16,
                size: size_of::<u16>(),
            })
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }
}


// An example variable object
pub struct Object3000 {
    pub value: AtomicCell<u8>,
}

impl Object3000 {
    pub fn set_value(&self, value: u8) {
        self.value.store(value);
    }
    pub fn get_value(&self) -> u8 {
        self.value.load()
    }
}

impl ObjectRawAccess for Object3000 {
    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
        if sub == 0 {
            let value = data[0];
            self.set_value(value);
            Ok(())
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        if sub == 0 {
            let value = self.get_value();
            buf[0] = value;
            Ok(())
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        if sub == 0 {
            Ok(SubInfo {
                access_type: AccessType::Rw,
                data_type: DataType::UInt8,
                size: 1,
            })
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }
}

pub static OBJECT1000: Object1000 = Object1000 {
    sub1: AtomicCell::new(0),
    sub3: AtomicCell::new(0.0),
};

pub static OBJECT2000: Object2000 = Object2000 {
    array: Mutex::new(RefCell::new([0; 10])),
};

pub static OBJECT3000: Object3000 = Object3000 {
    value: AtomicCell::new(0),
};

pub static OBJECT4000: CallbackObject = CallbackObject::new(ObjectCode::Var);

pub static OD_TABLE: [ODEntry; 4] = [
    ODEntry {
        index: 1000,
        data: ObjectData::Storage(&OBJECT1000),
    },
    ODEntry {
        index: 2000,
        data: ObjectData::Storage(&OBJECT2000),
    },
    ODEntry {
        index: 3000,
        data: ObjectData::Storage(&OBJECT3000),
    },
    ODEntry {
        index: 4000,
        data: ObjectData::Callback(&OBJECT4000),
    },
];

// pub struct NodeState {
//     rx_pdos: [RxPdo; 4],
//     sdos_cob_id: Option<CanId>,
//     sdo_mbox: AtomicCell<Option<CanFdMessage>>,
// }

// impl NodeState {
//     pub fn new() -> Self {
//         let rx_pdos = [RxPdo::default(), RxPdo::default(), RxPdo::default(), RxPdo::default()];
//         let sdos_cob_id = None;
//         let sdo_mbox = AtomicCell::new(None);
//         Self {
//             rx_pdos,
//             sdos_cob_id,
//             sdo_mbox,
//         }
//     }
// }

// impl NodeStateAccess for NodeState {
//     fn set_rx_pdo_cob_id(&self, idx: usize, cob_id: Option<CanId>) {
//         self.rx_pdos[idx].cob_id.store(cob_id);
//     }

//     fn num_rx_pdos(&self) -> usize {
//         self.rx_pdos.len()
//     }

//     fn read_rx_pdo(&self, idx: usize) -> Option<CanFdMessage> {
//         self.rx_pdos[idx].mbox.take()
//     }

//     /// Read a pending message for the main SDO mailbox. Will return None if there is no message.
//     fn read_sdo_mbox(&self) -> Option<CanFdMessage> {
//         self.sdo_mbox.take()
//     }
// }
