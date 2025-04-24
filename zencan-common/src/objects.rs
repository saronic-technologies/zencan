// pub enum ObjectAccess {

// }

use core::any::Any
;
use crossbeam::atomic::AtomicCell;

use crate::sdo::AbortCode;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectCode {
    Null = 0,
    Domain = 2,
    DefType = 5,
    DefStruct = 6,
    #[default]
    Var = 7,
    Array = 8,
    Record = 9,
}

impl TryFrom<u8> for ObjectCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ObjectCode::Null),
            2 => Ok(ObjectCode::Domain),
            5 => Ok(ObjectCode::DefType),
            6 => Ok(ObjectCode::DefStruct),
            7 => Ok(ObjectCode::Var),
            8 => Ok(ObjectCode::Array),
            9 => Ok(ObjectCode::Record),
            _ => Err(()),
        }
    }
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
        matches!(self, Self::VisibleString | Self::OctetString | Self::UnicodeString)
    }
}

pub trait ObjectRawAccess: Sync + Send {
    /// Read raw bytes from a subobject
    ///
    /// If the specified read goes out of range (i.e. offset + buf.len() > current_size) an error is
    /// returned
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode>;
    /// Write raw bytes to a subobject
    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode>;

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode>;

    fn access_type(&self, sub: u8) -> Result<AccessType, AbortCode> {
        Ok(self.sub_info(sub)?.access_type)
    }

    fn data_type(&self, sub: u8) -> Result<DataType, AbortCode> {
        Ok(self.sub_info(sub)?.data_type)
    }

    /// Get the maximum size of an sub object
    ///
    /// For most sub objects, this matches the current_size, but for strings the size of the
    /// currently stored value (returned by `current_size()`) may be smaller.
    fn size(&self, sub: u8) -> Result<usize, AbortCode> {
        Ok(self.sub_info(sub)?.size)
    }

    /// Get the current size of a sub object
    ///
    /// Note that this is not necessarily the allocated size of the object, as some objects (such as
    /// strings) may have values shorter than their maximum size. As such, this gives the maximum
    /// number of bytes which may be read, but not necessarily the number of bytes which may be
    /// written.
    fn current_size(&self, sub: u8) -> Result<usize, AbortCode> {
        const CHUNK_SIZE: usize = 8;

        let size = self.size(sub)?;
        if self.data_type(sub)?.is_str() {
            // Look for first 0
            let mut chunk = 0;
            let mut buf = [0; CHUNK_SIZE];
            while chunk < size / CHUNK_SIZE + 1 {
                let offset = chunk * CHUNK_SIZE;
                let bytes_to_read = (size - offset).min(CHUNK_SIZE);
                self.read(sub, offset, &mut buf[0..bytes_to_read])?;

                if let Some(zero_pos) = buf[0..bytes_to_read].iter().position(|b| *b == 0) {
                    return Ok(zero_pos + chunk * CHUNK_SIZE);
                }
                chunk += 1;
            }
        }
        // not a string type or no null-terminator was found
        Ok(size)
    }
}

#[derive(Default)]
pub struct CallbackObject {
    write_cb: AtomicCell<Option<WriteHookFn>>,
    read_cb: AtomicCell<Option<ReadHookFn>>,
    info_cb: AtomicCell<Option<InfoHookFn>>,
    context: AtomicCell<Option<&'static dyn Context>>,
}

impl CallbackObject {
    pub const fn new() -> Self {
        Self {
            write_cb: AtomicCell::new(None),
            read_cb: AtomicCell::new(None),
            info_cb: AtomicCell::new(None),
            context: AtomicCell::new(None),
        }
    }

    pub fn register(
        &self,
        write: Option<WriteHookFn>,
        read: Option<ReadHookFn>,
        info: Option<InfoHookFn>,
        context: Option<&'static dyn Context>,
    ) {
        self.write_cb.store(write);
        self.read_cb.store(read);
        self.info_cb.store(info);
        self.context.store(context);
    }

}

impl ObjectRawAccess for CallbackObject {
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        if let Some(read) = self.read_cb.load() {
            (read)(&self.context.load(), sub, offset, buf)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
        if let Some(write) = self.write_cb.load() {
            (write)(&self.context.load(), sub, offset, data)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        if let Some(info) = self.info_cb.load() {
            (info)(&self.context.load(), sub)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }
}

// Trait to define requirements for opaque callback context references
pub trait Context: Any + Sync + Send + 'static {
    fn as_any<'a, 'b: 'a>(&'b self) -> &'a dyn Any;
}

impl<T: Any + Sync + Send + 'static> Context for T {
    fn as_any<'a, 'b: 'a>(&'b self) -> &'a dyn Any {
        self
    }
}

/// Object read/write callback function signature
type ReadHookFn = fn(
    ctx: &Option<&dyn Context>,
    sub: u8,
    offset: usize,
    buf: &mut [u8]
) -> Result<(), AbortCode>;

type WriteHookFn = fn(
    ctx: &Option<&dyn Context>,
    sub: u8,
    offset: usize,
    buf: &[u8]
) -> Result<(), AbortCode>;

type InfoHookFn = fn(
    ctx: &Option<&dyn Context>,
    sub: u8,
) -> Result<SubInfo, AbortCode>;

pub enum ObjectData<'a> {
    Storage(&'a dyn ObjectRawAccess),
    Callback(&'a CallbackObject),
}

impl ObjectRawAccess for ObjectData<'_> {
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        match self {
            ObjectData::Storage(obj) => obj.read(sub, offset, buf),
            ObjectData::Callback(obj) => obj.read(sub, offset, buf),
        }
    }

    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
        match self {
            ObjectData::Storage(obj) => obj.write(sub, offset, data),
            ObjectData::Callback(obj) => obj.write(sub, offset, data),
        }
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        match self {
            ObjectData::Storage(obj) => obj.sub_info(sub),
            ObjectData::Callback(obj) => obj.sub_info(sub),
        }
    }
}

/// Represents one item in the in-memory table of objects
pub struct ODEntry<'a> {
    /// The object index
    pub index: u16,
    pub data: ObjectData<'a>,
}

pub struct SubInfo {
    /// The size (or max size) of this sub object, in bytes
    pub size: usize,
    /// The data type of this sub object
    pub data_type: DataType,
    /// Indicates what accesses (i.e. read/write) are allowed on this sub object
    pub access_type: AccessType,
}

pub fn find_object<'a, 'b>(table: &'b[ODEntry<'a>], index: u16) -> Option<&'b ObjectData<'a>> {
    // TODO: Table is sorted, so we could use binary search
    for entry in table {
        if entry.index == index {
            return Some(&entry.data);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn test_object_var() {
    //     let dict = ObjectDict::new(&OD_TABLE);
    //     let object = dict.find(0x1000).unwrap();
    //     let node = object.get_sub(0).unwrap();
    //     //let node = dict.find_sub(0x1000, 0).unwrap();
    //     node.write(0, &10u32.to_le_bytes()).unwrap();
    //     let mut buf = [0u8; 4];
    //     node.read(0, &mut buf);
    //     assert_eq!(10, u32::from_le_bytes(buf));
    // }

    // #[test]
    // fn test_object_array() {
    //     let dict = ObjectDict::new(&OD_TABLE);
    //     let object = dict.find(0x1001).unwrap();
    //     let node0 = object.get_sub(0).unwrap();
    //     let mut buf = [0u8];
    //     node0.read(0, &mut buf);
    //     assert_eq!(buf[0], ITEM2_SIZE);

    //     let node1 = object.get_sub(1).unwrap();
    //     let buf = [0; 2];
    //     node1.write(0, &99u16.to_le_bytes()).unwrap();
    // }

    // #[test]
    // fn test_object_record() {
    //     let dict = ObjectDict::new(&OD_TABLE);
    //     let object = dict.find(0x1002).unwrap();
    //     let node1 = object.get_sub(1).unwrap();
    //     node1.write(0, &44u32.to_le_bytes()).unwrap();
    //     let mut buf = [0; 4];
    //     node1.read(0, &mut buf);
    //     assert_eq!(44, u32::from_le_bytes(buf));

    //     let node3 = object.get_sub(3).unwrap();
    //     node3.write(0, &22u8.to_le_bytes()).unwrap();
    //     let mut buf = [0; 1];
    //     node3.read(0, &mut buf);
    //     assert_eq!(22, buf[0]);

    // }


    // #[test]
    // fn test_write_hook() {
    //     let mut dict = ObjectDict::new(&OD_TABLE);

    //     fn fail_hook(
    //         ctx: &Option<&dyn Context>,
    //         object: &Object,
    //         sub: u8,
    //         offset: usize,
    //         buf: &[u8]
    //     ) -> Result<(), AbortCode> {
    //         let binding2 = ctx.unwrap().as_any().downcast_ref::<Mutex<RefCell<u32>>>().unwrap();
    //         critical_section::with(|cs| {
    //             *binding2.borrow_ref_mut(cs) += 1;
    //         });

    //         return Err(AbortCode::CantStore)
    //     }

    //     let value = Mutex::new(RefCell::new(20u32));
    //     dict.register_hook(0x1000, Some(&value), None, Some(fail_hook));

    //     let object = dict.find(0x1000).unwrap();
    //     let result = object.write(0x1, 0, &[]);
    //     assert_eq!(Err(AbortCode::CantStore), result);
    //     let stored_value = critical_section::with(|cs| {
    //         *value.borrow_ref_mut(cs)
    //     });
    //     assert_eq!(21, stored_value);
    // }

    // fn test() {
    // }

}
