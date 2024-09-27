// pub enum ObjectAccess {

// }

use core::{
    cell::RefCell,
    mem::{size_of, size_of_val},
    ptr::{self, addr_of, addr_of_mut}, str::FromStr,
};
use critical_section::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectCode {
    Null = 0,
    Domain = 2,
    DefType = 5,
    DefStruct = 6,
    Var = 7,
    Array = 8,
    Record = 9,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum AccessType {
    /// Read-only
    #[default]
    Ro,
    /// Write-only
    Wo,
    /// Read-write
    Rw,
    /// Read-only, and also will never be changed, even internally by the device
    Const,
}



#[derive(Copy, Clone, Debug, Default)]
#[repr(u16)]
pub enum DataType {
    Boolean = 1,
    #[default]
    Int8 = 2,
    Int16 = 3,
    Int32 = 4,
    UInt8 = 5,
    UInt16 = 6,
    UInt32 = 7,
    Real32 = 8,
    VisibleString = 9,
    OctetString = 0xa,
    UnicodeString = 0xb,
    TimeOfDay = 0xc,
    TimeDifference = 0xd,
    Domain = 0xf,
    Other(u16),
}

impl From<u16> for DataType {
    fn from(value: u16) -> Self {
        use DataType::*;
        match value {
            1 => Boolean,
            2 => Int8,
            3 => Int16,
            4 => Int32,
            5 => UInt8,
            6 => UInt16,
            7 => UInt32,
            8 => Real32,
            9 => VisibleString,
            0xa => OctetString,
            0xb => UnicodeString,
            0xf => Domain,
            _ => Other(value),
        }
    }
}


impl DataType {
    /// Returns true if data type is one of the string types
    pub fn is_str(&self) -> bool {
        match self {
            Self::VisibleString | Self::OctetString | Self::UnicodeString => true,
            _ => false,
        }
    }
}

fn element_storage_size(dt: DataType) -> usize {
    match dt {
        DataType::Boolean => 1,
        DataType::Int8 => 1,
        DataType::Int16 => 2,
        DataType::Int32 => 4,
        DataType::UInt8 => 1,
        DataType::UInt16 => 2,
        DataType::UInt32 => 4,
        DataType::Real32 => 5,
        _ => 0,
    }
}

pub struct AppCallback<'a> {
    read: Option<&'a dyn Fn(usize, &mut [u8])>,
    write: Option<&'a dyn Fn(usize, &[u8])>,
}

pub enum ObjectStorage<'a> {
    Ram(*const u8, usize),
    Const(*const u8, usize),
    App(AppCallback<'a>),
}
unsafe impl<'a> Send for ObjectStorage<'a> {}
unsafe impl<'a> Sync for ObjectStorage<'a> {}

pub struct Var<'a> {
    pub data_type: DataType,
    pub access_type: AccessType,
    pub size: usize,
    pub storage: Mutex<RefCell<ObjectStorage<'a>>>,
}

pub struct Array<'a> {
    pub data_type: DataType,
    pub access_type: AccessType,
    pub size: usize,
    pub storage_sub0: Mutex<RefCell<ObjectStorage<'a>>>,
    pub storage: Mutex<RefCell<ObjectStorage<'a>>>,
}

pub struct Record<'a> {
    pub data_types: &'a [Option<DataType>],
    pub access_types: &'a [Option<AccessType>],
    pub sizes: &'a [usize],
    pub storage_sub0: Mutex<RefCell<ObjectStorage<'a>>>,
    pub storage: &'a [Option<Mutex<RefCell<ObjectStorage<'a>>>>],
}

pub struct Object<'a> {
    pub data: ObjectData<'a>,
}

pub enum ObjectData<'a> {
    Var(Var<'a>),
    Array(Array<'a>),
    Record(Record<'a>),
}

impl<'a> Object<'a> {
    pub fn get_sub<'b: 'a>(&'b self, sub: u8) -> Option<SubObject<'b>> {
        match &self.data {
            ObjectData::Var(var) => {
                if sub == 0 {
                    Some(SubObject {
                        offset: 0,
                        data_type: var.data_type,
                        access_type: var.access_type,
                        size: var.size,
                        storage: &var.storage,
                    })
                } else {
                    None
                }
            }
            ObjectData::Array(arr) => {
                if sub == 0 {
                    Some(SubObject {
                        offset: 0,
                        data_type: DataType::UInt8,
                        access_type: AccessType::Ro,
                        size: 1,
                        storage: &arr.storage_sub0,
                    })
                } else if sub <= arr.size as u8 {
                    Some(SubObject {
                        offset: (sub as usize - 1) * element_storage_size(arr.data_type),
                        data_type: arr.data_type,
                        access_type: arr.access_type,
                        size: element_storage_size(arr.data_type),
                        storage: &arr.storage,
                    })
                } else {
                    None
                }
            }
            ObjectData::Record(rec) => {
                if sub == 0 {
                    Some(SubObject {
                        offset: 0,
                        data_type: DataType::UInt8,
                        access_type: AccessType::Const,
                        size: 1,
                        storage: &rec.storage_sub0,
                    })
                } else if sub as usize <= rec.storage.len() {
                    let storage = rec.storage.get(sub as usize - 1)?;
                    if let Some(storage) = storage {
                        Some(SubObject {
                            offset: 0,
                            // unwrap safety: if storage is not None, data type must be not None
                            data_type: rec.data_types[sub as usize - 1].unwrap(),
                            access_type: rec.access_types[sub as usize - 1].unwrap(),
                            size: rec.sizes[sub as usize - 1],
                            storage,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
}

/// Represents one item in the in-memory table of objects
pub struct ODEntry<'a> {
    /// The object index
    pub index: u16,
    pub object: &'a Object<'a>,
}

impl<'a> ODEntry<'a> {
    pub fn obj_code(&self) -> ObjectCode {
        match &self.object.data {
            ObjectData::Var(_) => ObjectCode::Var,
            ObjectData::Array(_) => ObjectCode::Array,
            ObjectData::Record(_) => ObjectCode::Record,
        }
    }

    pub fn sub_count(&self) -> u8 {
        match &self.object.data {
            ObjectData::Var(_) => 1,
            ObjectData::Array(array) => (array.size + 1) as u8,
            ObjectData::Record(_) => todo!(),
        }
    }
}

unsafe impl<'a> Sync for ODEntry<'a> {}

pub struct SubObject<'a> {
    // NOTE: Maybe ObjectStorage here should actually be an Arc<Mutex<>>; we will need some locking
    // at some point for threaded access, and maybe it should be at the sub object level rather than
    // a single mutex on the whole dict...
    storage: &'a Mutex<RefCell<ObjectStorage<'a>>>,
    /// The offset into the storage at which this sub element is stored (e.g. for arrays and
    /// records)
    offset: usize,
    pub size: usize,
    pub data_type: DataType,
    pub access_type: AccessType,

}

impl<'a> SubObject<'a> {
    pub fn write(&self, offset: usize, data: &[u8]) {
        let offset = offset + self.offset;
        critical_section::with(|cs| match *self.storage.borrow(cs).borrow_mut() {
            ObjectStorage::Ram(ptr, size) => {
                let slice = unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, size) };
                slice[offset..offset + data.len()].copy_from_slice(data);
            }
            ObjectStorage::Const(_, _) => panic!("Write to const object"),
            ObjectStorage::App(ref mut cb) => {
                if let Some(write) = cb.write.as_mut() {
                    write(offset, data);
                }
            }
        });
    }

    /// Read raw bytes from the object
    ///
    /// This function will panic on invalid read. It is up to the caller to ensure that the size
    /// and offset fall within the range of the object
    pub fn read(&self, offset: usize, buf: &mut [u8]) {
        let offset = offset + self.offset;
        critical_section::with(|cs| match *self.storage.borrow(cs).borrow() {
            ObjectStorage::Ram(ptr, size) => {
                let slice = unsafe { core::slice::from_raw_parts(ptr, size) };
                buf.copy_from_slice(&slice[offset..offset + buf.len()])
            }
            ObjectStorage::Const(ptr, size) => {
                let slice = unsafe { core::slice::from_raw_parts(ptr, size) };
                buf.copy_from_slice(&slice[offset..offset + buf.len()])
            }
            ObjectStorage::App(ref callbacks) => {
                if let Some(read) = callbacks.read {
                    read(offset, buf);
                } else {
                    panic!("Attempted to read from DOMAIN with no registered callback");
                }
            }
        })
    }

    /// Get the size of the current value in the sub object
    ///
    /// For string types, this can be shorter than the allocated size.
    pub fn current_size(&self) -> usize {
        const CHUNK_SIZE: usize = 8;
        if self.data_type.is_str() {
            // Look for first 0
            let mut chunk = 0;
            let mut buf = [0; CHUNK_SIZE];

            while chunk < self.size / CHUNK_SIZE + 1 {
                let offset = chunk * CHUNK_SIZE;
                let bytes_to_read = (self.size - offset).min(CHUNK_SIZE);
                self.read(offset, &mut buf[0..bytes_to_read]);

                if let Some(zero_pos) = buf[0..bytes_to_read].iter().position(|b| *b == 0) {
                    return zero_pos + chunk * CHUNK_SIZE;
                }
                chunk += 1;
            }
            self.size
        } else {
            self.size
        }
    }

}

pub struct ObjectDict<'a> {
    pub table: &'a [ODEntry<'a>],
}

impl<'a> ObjectDict<'a> {
    pub fn find<'b: 'a>(&self, index: u16) -> Option<&'b Object<'a>> {
        for i in 0..self.table.len() {
            if self.table[i].index == index {
                return Some(self.table[i].object);
            }
        }
        None
    }
    pub fn find_entry<'b>(&self, index: u16) -> Option<&'b ODEntry<'a>> {
        for i in 0..self.table.len() {
            if self.table[i].index == index {
                return Some(&self.table[i]);
            }
        }
        None
    }

    pub fn find_sub<'b>(&'b self, index: u16, sub: u8) -> Option<SubObject<'a>> {
        let entry = self.find_entry(index)?;
        entry.object.get_sub(sub)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_object_var() {
//         let dict = ObjectDict { table: &OD_TABLE };
//         let object = dict.find(0x1000).unwrap();
//         let node = object.get_sub(0).unwrap();
//         //let node = dict.find_sub(0x1000, 0).unwrap();
//         node.write(0, &10u32.to_le_bytes());
//         let mut buf = [0u8; 4];
//         node.read(0, &mut buf);
//         assert_eq!(10, u32::from_le_bytes(buf));
//     }

//     #[test]
//     fn test_object_array() {
//         let dict = ObjectDict { table: &OD_TABLE };
//         let object = dict.find(0x1001).unwrap();
//         let node0 = object.get_sub(0).unwrap();
//         let mut buf = [0u8];
//         node0.read(0, &mut buf);
//         assert_eq!(buf[0], ITEM2_SIZE);

//         let node1 = object.get_sub(1).unwrap();
//         let buf = [0; 2];
//         node1.write(0, &99u16.to_le_bytes());
//     }

//     #[test]
//     fn test_object_record() {
//         let dict = ObjectDict { table: &OD_TABLE };
//         let object = dict.find(0x1002).unwrap();
//         let node1 = object.get_sub(1).unwrap();
//         node1.write(0, &44u32.to_le_bytes());
//         let mut buf = [0; 4];
//         node1.read(0, &mut buf);
//         assert_eq!(44, u32::from_le_bytes(buf));

//         let node3 = object.get_sub(3).unwrap();
//         node3.write(0, &22u8.to_le_bytes());
//         let mut buf = [0; 1];
//         node3.read(0, &mut buf);
//         assert_eq!(22, buf[0]);
//     }
// }
