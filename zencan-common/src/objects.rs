// pub enum ObjectAccess {

// }

use crate::sdo::AbortCode;
use crate::AtomicCell;
use core::any::Any;
use core::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct ObjectFlagSync {
    toggle: AtomicBool,
}

impl ObjectFlagSync {
    pub const fn new() -> Self {
        Self { toggle: AtomicBool::new(false) }
    }

    pub fn toggle(&self) {
        self.toggle.fetch_xor(true, Ordering::SeqCst);
    }
}

/// Store an event flag for each sub object in an object
#[derive(Debug)]
pub struct ObjectFlags<const N: usize>
{
    sync: &'static ObjectFlagSync,
    flags0: AtomicCell<[u8; N]>,
    flags1: AtomicCell<[u8; N]>,
}

impl<const N: usize> ObjectFlags<N> {
    pub const fn new(sync: &'static ObjectFlagSync) -> Self {
        Self { sync, flags0: AtomicCell::new([0; N]), flags1: AtomicCell::new([0; N]) }
    }

    pub fn set_flag(&self, sub: u8) {
        if sub as usize >= N * 8 {
            return;
        }
        let flags = if self.sync.toggle.load(Ordering::Acquire) {
            &self.flags0
        } else {
            &self.flags1
        };
        flags.fetch_update(|mut f| {
            f[sub as usize / 8] |= 1 << (sub & 7);
            Some(f)
        }).unwrap();
    }

    pub fn get_flag(&self, sub: u8) -> bool {
        if sub as usize >= N*8 {
            return false;
        }
        let flags = if self.sync.toggle.load(Ordering::Acquire) {
            &self.flags1.load()
        } else {
            &self.flags0.load()
        };
        flags[(sub / 8) as usize] & (1 << (sub & 7)) != 0
    }

    pub fn clear(&self) {
        if self.sync.toggle.load(Ordering::Relaxed) {
            self.flags0.store([0; N]);
        } else {
            self.flags1.store([0; N]);
        }
    }
}

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

#[derive(Copy, Clone, Debug)]
pub enum PdoMapping {
    None,
    Rpdo,
    Tpdo,
    Both,
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
        matches!(
            self,
            Self::VisibleString | Self::OctetString | Self::UnicodeString
        )
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

    /// Get metadata about a sub object
    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode>;

    fn set_event_flag(&self, _sub: u8) -> Result<(), AbortCode> {
        return Err(AbortCode::UnsupportedAccess);
    }

    fn read_event_flag(&self, _sub: u8) -> bool {
        false
    }

    fn clear_events(&self) {}

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

    fn read_u32(&self, sub: u8) -> Result<u32, AbortCode> {
        let mut buf = [0; 4];
        self.read(sub, 0, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u16(&self, sub: u8) -> Result<u16, AbortCode> {
        let mut buf = [0; 2];
        self.read(sub, 0, &mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u8(&self, sub: u8) -> Result<u8, AbortCode> {
        let mut buf = [0; 1];
        self.read(sub, 0, &mut buf)?;
        Ok(buf[0])
    }

    fn read_i32(&self, sub: u8) -> Result<i32, AbortCode> {
        let mut buf = [0; 4];
        self.read(sub, 0, &mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    fn read_i16(&self, sub: u8) -> Result<i16, AbortCode> {
        let mut buf = [0; 2];
        self.read(sub, 0, &mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    fn read_i8(&self, sub: u8) -> Result<i8, AbortCode> {
        let mut buf = [0; 1];
        self.read(sub, 0, &mut buf)?;
        Ok(buf[0] as i8)
    }
}

pub struct CallbackObject {
    write_cb: AtomicCell<Option<WriteHookFn>>,
    read_cb: AtomicCell<Option<ReadHookFn>>,
    info_cb: AtomicCell<Option<InfoHookFn>>,

    od: &'static [ODEntry<'static>],
    context: AtomicCell<Option<&'static dyn Context>>,
}

impl CallbackObject {
    pub const fn new(od: &'static [ODEntry<'static>]) -> Self {
        Self {
            write_cb: AtomicCell::new(None),
            read_cb: AtomicCell::new(None),
            info_cb: AtomicCell::new(None),
            od,
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
            (read)(&self.context.load(), self.od, sub, offset, buf)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
        if let Some(write) = self.write_cb.load() {
            (write)(&self.context.load(), self.od, sub, offset, data)
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
    od: &[ODEntry],
    sub: u8,
    offset: usize,
    buf: &mut [u8],
) -> Result<(), AbortCode>;

type WriteHookFn = fn(
    ctx: &Option<&dyn Context>,
    od: &[ODEntry],
    sub: u8,
    offset: usize,
    buf: &[u8],
) -> Result<(), AbortCode>;

type InfoHookFn = fn(ctx: &Option<&dyn Context>, sub: u8) -> Result<SubInfo, AbortCode>;

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

    fn set_event_flag(&self, sub: u8) -> Result<(), AbortCode> {
        match self {
            ObjectData::Storage(obj) => obj.set_event_flag(sub),
            ObjectData::Callback(obj) => obj.set_event_flag(sub),
        }
    }

    fn read_event_flag(&self, sub: u8) -> bool {
        match self {
            ObjectData::Storage(obj) => obj.read_event_flag(sub),
            ObjectData::Callback(obj) => obj.read_event_flag(sub),
        }
    }

    fn clear_events(&self) {
        match self {
            ObjectData::Storage(obj) => obj.clear_events(),
            ObjectData::Callback(obj) => obj.clear_events(),
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
    pub pdo_mapping: PdoMapping,
}

pub fn find_object<'a, 'b>(table: &'b [ODEntry<'a>], index: u16) -> Option<&'b ObjectData<'a>> {
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
