//! Object Dictionary Implementation
//!
//! The object dictionary is typically generated using `zencan-build`, using the types provided
//! here.
//!
//!
//! ## Topics
//!
//! ### PDO event triggering
//!

use critical_section::Mutex;

use crate::sdo::AbortCode;
use crate::AtomicCell;
use core::any::Any;
use core::cell::UnsafeCell;

/// A container for the address of a subobject
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectId {
    /// Object index
    pub index: u16,
    /// Sub index
    pub sub: u8,
}

/// A struct used for synchronizing the A/B event flags of all objects, which are used for
/// triggering PDO events
#[derive(Debug)]
pub struct ObjectFlagSync {
    inner: Mutex<UnsafeCell<ObjectFlagsInner>>,
}

impl Default for ObjectFlagSync {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
struct ObjectFlagsInner {
    /// Indicates which "bank" of flags should be active for setting
    toggle: bool,
    /// A global flag that should be set by any object which has set a flag
    global_flag: bool,
}

impl ObjectFlagSync {
    /// Create a new ObjectFlagSync
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(UnsafeCell::new(ObjectFlagsInner {
                toggle: false,
                global_flag: false,
            })),
        }
    }

    /// Toggle the flag and return the global flag
    pub fn toggle(&self) -> bool {
        critical_section::with(|cs| {
            let inner = self.inner.borrow(cs).get();
            // Safety: This is the only place inner is accessed, and it is in a critical section
            unsafe {
                let global = (*inner).global_flag;
                (*inner).global_flag = false;
                (*inner).toggle = !(*inner).toggle;
                global
            }
        })
    }

    /// Get the current value of the flag
    ///
    /// `setting` should be true to set the global flag
    pub fn get_flag(&self, setting: bool) -> bool {
        critical_section::with(|cs| {
            let inner = unsafe { &mut (*self.inner.borrow(cs).get()) };
            inner.global_flag |= setting;
            inner.toggle
        })
    }
}

/// Stores an event flag for each sub object in an object
///
/// PDO transmission can be triggered by events, but PDOs are runtime configurable. An application
/// needs to be able to signal that an object has changed, and if that object is mapped to a TPDO,
/// that PDO should be scheduled for transmission.
///
/// In order to achieve this in a synchronized way without long critical sections, each object
/// holds two sets of flags, and they are swapped atomically using a global `ObjectFlagSync` shared by
/// all `ObjectFlags` instances.
#[derive(Debug)]
pub struct ObjectFlags<const N: usize> {
    sync: &'static ObjectFlagSync,
    flags0: AtomicCell<[u8; N]>,
    flags1: AtomicCell<[u8; N]>,
}

impl<const N: usize> ObjectFlags<N> {
    /// Create a new ObjectFlags
    pub const fn new(sync: &'static ObjectFlagSync) -> Self {
        Self {
            sync,
            flags0: AtomicCell::new([0; N]),
            flags1: AtomicCell::new([0; N]),
        }
    }

    /// Set the flag for the specified sub object
    ///
    /// The flag is set on the currently active flag set
    pub fn set_flag(&self, sub: u8) {
        if sub as usize >= N * 8 {
            return;
        }
        let flags = if self.sync.get_flag(true) {
            &self.flags0
        } else {
            &self.flags1
        };
        flags
            .fetch_update(|mut f| {
                f[sub as usize / 8] |= 1 << (sub & 7);
                Some(f)
            })
            .unwrap();
    }

    /// Read the flag for the specified object
    ///
    /// The flag is read from the currently inactive flag set, i.e. the flag value from before the
    /// last sync toggle is returned
    pub fn get_flag(&self, sub: u8) -> bool {
        if sub as usize >= N * 8 {
            return false;
        }
        let flags = if self.sync.get_flag(false) {
            &self.flags1.load()
        } else {
            &self.flags0.load()
        };
        flags[(sub / 8) as usize] & (1 << (sub & 7)) != 0
    }

    /// Clear all flags in the currently inactive set
    pub fn clear(&self) {
        if self.sync.get_flag(false) {
            self.flags1.store([0; N]);
        } else {
            self.flags0.store([0; N]);
        }
    }
}

/// Object Code value
///
/// Defines the type of an object or sub object
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
#[allow(missing_docs)]
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

/// Access type enum
#[derive(Copy, Clone, Debug, Default, PartialEq)]
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

impl AccessType {
    /// Returns true if an object with this access type can be read
    pub fn is_readable(&self) -> bool {
        matches!(self, AccessType::Ro | AccessType::Rw | AccessType::Const)
    }

    /// Returns true if an object with this access type can be written
    pub fn is_writable(&self) -> bool {
        matches!(self, AccessType::Rw | AccessType::Wo)
    }
}

/// Possible PDO mapping values for an object
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum PdoMapping {
    /// Object cannot be mapped to PDOs
    #[default]
    None,
    /// Object can be mapped to RPDOs only
    Rpdo,
    /// Object can be mapped to TPDOs only
    Tpdo,
    /// Object can be mapped to both RPDOs and TPDOs
    Both,
}

/// Indicate the type of data stored in an object
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[repr(u16)]
#[allow(missing_docs)]
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

/// A trait for accessing objects
///
/// Any struct which implements an object in the object dictionary must implement this trait
pub trait ObjectRawAccess: Sync + Send {
    /// Read raw bytes from a subobject
    ///
    /// If the specified read goes out of range (i.e. offset + buf.len() > current_size) an error is
    /// returned. All implementers are required to allow reading a subset of the object bytes, i.e.
    /// offset may be non-zero, and/or the buf length may be shorter than the object data
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode>;
    /// Write raw bytes to a subobject
    ///
    /// Implementers MAY require all bytes of the object to be written, in which case, if offset is
    /// non-zero or `offset + data.len()` is less than the size of the object, it may return an
    /// error with [AbortCode::DataTypeMismatchLengthLow]
    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode>;

    /// Get the type of this object
    fn object_code(&self) -> ObjectCode;

    /// Get metadata about a sub object
    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode>;

    /// Get the highest sub index available in this object
    fn max_sub_number(&self) -> u8 {
        match self.object_code() {
            ObjectCode::Null => 0,
            ObjectCode::Domain => 0,
            ObjectCode::DefType => 0,
            ObjectCode::DefStruct => 0,
            ObjectCode::Var => 0,
            ObjectCode::Array => self.read_u8(0).unwrap(),
            ObjectCode::Record => self.read_u8(0).unwrap(),
        }
    }

    /// Set an event flag for the specified sub object on this object
    ///
    /// Event flags are used for triggering PDOs. This is optional, as not all objects support PDOs
    /// or PDO triggering.
    fn set_event_flag(&self, _sub: u8) -> Result<(), AbortCode> {
        Err(AbortCode::UnsupportedAccess)
    }

    /// Read an event flag for the specified sub object
    ///
    /// This is optional as not all objects support events
    fn read_event_flag(&self, _sub: u8) -> bool {
        false
    }

    /// Clear event flags for all sub objects
    ///
    /// This is optional as not all objects support events
    fn clear_events(&self) {}

    /// Get the access type of a specific sub object
    fn access_type(&self, sub: u8) -> Result<AccessType, AbortCode> {
        Ok(self.sub_info(sub)?.access_type)
    }

    /// Get the data type of a specific sub object
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

    /// Read a sub object as a u32
    fn read_u32(&self, sub: u8) -> Result<u32, AbortCode> {
        let mut buf = [0; 4];
        self.read(sub, 0, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Read a sub object as a u16
    fn read_u16(&self, sub: u8) -> Result<u16, AbortCode> {
        let mut buf = [0; 2];
        self.read(sub, 0, &mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    /// Read a sub object as a u8
    fn read_u8(&self, sub: u8) -> Result<u8, AbortCode> {
        let mut buf = [0; 1];
        self.read(sub, 0, &mut buf)?;
        Ok(buf[0])
    }

    /// Read a sub object as an i32
    fn read_i32(&self, sub: u8) -> Result<i32, AbortCode> {
        let mut buf = [0; 4];
        self.read(sub, 0, &mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    /// Read a sub object as an i16
    fn read_i16(&self, sub: u8) -> Result<i16, AbortCode> {
        let mut buf = [0; 2];
        self.read(sub, 0, &mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    /// Read a sub object as an i8
    fn read_i8(&self, sub: u8) -> Result<i8, AbortCode> {
        let mut buf = [0; 1];
        self.read(sub, 0, &mut buf)?;
        Ok(buf[0] as i8)
    }
}

/// Implements an object which relies on provided callback functions for implementing access
///
/// This is used for objects which have extra logic in their access, such as PDOs. Some callbacks
/// are implemented by `zencan-node`, but they can also be implemented by the application.
pub struct CallbackObject {
    write_cb: AtomicCell<Option<WriteHookFn>>,
    read_cb: AtomicCell<Option<ReadHookFn>>,
    info_cb: AtomicCell<Option<InfoHookFn>>,
    object_code: ObjectCode,

    od: &'static [ODEntry<'static>],
    context: AtomicCell<Option<&'static dyn Context>>,
}

impl CallbackObject {
    /// Create a new CallbackObject
    ///
    /// # Arguments
    /// - `od`: The object dictionary. This is required so it can be passed to callbacks.
    /// - `object_code`: The object code for this record. Although the sub object types are all
    ///   defined by the info callback, the object code must be determined at build time.
    pub const fn new(od: &'static [ODEntry<'static>], object_code: ObjectCode) -> Self {
        Self {
            write_cb: AtomicCell::new(None),
            read_cb: AtomicCell::new(None),
            info_cb: AtomicCell::new(None),
            object_code,
            od,
            context: AtomicCell::new(None),
        }
    }

    /// Register callbacks on this object
    pub fn register(
        &self,
        write: Option<WriteHookFn>,
        read: Option<ReadHookFn>,
        info: Option<InfoHookFn>,
        context: Option<&'static dyn Context>,
    ) {
        // TODO: Think about which of these should actually be optional. e.g. I don't think you can
        // not implement info.
        self.write_cb.store(write);
        self.read_cb.store(read);
        self.info_cb.store(info);
        self.context.store(context);
    }
}

impl ObjectRawAccess for CallbackObject {
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        if let Some(read) = self.read_cb.load() {
            let caller_ctx = self.context.load();
            let ctx = ODCallbackContext {
                ctx: &caller_ctx,
                od: self.od,
            };
            (read)(&ctx, sub, offset, buf)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
        if let Some(write) = self.write_cb.load() {
            let caller_ctx = self.context.load();
            let ctx = ODCallbackContext {
                ctx: &caller_ctx,
                od: self.od,
            };
            (write)(&ctx, sub, offset, data)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        if let Some(info) = self.info_cb.load() {
            let caller_ctx = self.context.load();
            let ctx = ODCallbackContext {
                ctx: &caller_ctx,
                od: self.od,
            };
            (info)(&ctx, sub)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn object_code(&self) -> ObjectCode {
        self.object_code
    }
}

/// Trait to define requirements for opaque callback context references
pub trait Context: Any + Sync + Send + 'static {
    /// Get context as an Any ref
    fn as_any<'a, 'b: 'a>(&'b self) -> &'a dyn Any;
}

impl<T: Any + Sync + Send + 'static> Context for T {
    fn as_any<'a, 'b: 'a>(&'b self) -> &'a dyn Any {
        self
    }
}

/// Data to be passed to object callbacks
pub struct ODCallbackContext<'a> {
    /// The context object provided when registering the callback
    ///
    /// This can be any object which implement Any + Sync + Send
    pub ctx: &'a Option<&'a dyn Context>,

    /// The object dictionary
    pub od: &'static [ODEntry<'static>],
}

/// Object read/write callback function signature
type ReadHookFn =
    fn(ctx: &ODCallbackContext, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode>;

type WriteHookFn =
    fn(ctx: &ODCallbackContext, sub: u8, offset: usize, buf: &[u8]) -> Result<(), AbortCode>;

type InfoHookFn = fn(ctx: &ODCallbackContext, sub: u8) -> Result<SubInfo, AbortCode>;

/// Object enum to be stored in the dictionary
pub enum ObjectData<'a> {
    /// This is a normal object with allocated storage, e.g. as created by `zencan-build`
    Storage(&'a dyn ObjectRawAccess),
    /// This is a callback object which must have callback functions registered at runtime for
    /// access
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

    fn object_code(&self) -> ObjectCode {
        match self {
            ObjectData::Storage(obj) => obj.object_code(),
            ObjectData::Callback(obj) => obj.object_code(),
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
    /// The object implementation
    pub data: ObjectData<'a>,
}

/// Information about a sub object
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SubInfo {
    /// The size (or max size) of this sub object, in bytes
    pub size: usize,
    /// The data type of this sub object
    pub data_type: DataType,
    /// Indicates what accesses (i.e. read/write) are allowed on this sub object
    pub access_type: AccessType,
    /// Indicates whether this sub may be mapped to PDOs
    pub pdo_mapping: PdoMapping,
    /// Indicates whether this sub should be persisted when data is saved
    pub persist: bool,
}

impl SubInfo {
    /// A shorthand value for sub0 on record and array objects
    pub const MAX_SUB_NUMBER: SubInfo = SubInfo {
        size: 1,
        data_type: DataType::UInt8,
        access_type: AccessType::Const,
        pdo_mapping: PdoMapping::None,
        persist: false,
    };

    /// Convenience function for creating a new sub-info by type
    pub const fn new_u32() -> Self {
        Self {
            size: 4,
            data_type: DataType::UInt32,
            access_type: AccessType::Ro,
            pdo_mapping: PdoMapping::None,
            persist: false,
        }
    }

    /// Convenience function for creating a new sub-info by type
    pub const fn new_u16() -> Self {
        Self {
            size: 2,
            data_type: DataType::UInt16,
            access_type: AccessType::Ro,
            pdo_mapping: PdoMapping::None,
            persist: false,
        }
    }

    /// Convenience function for creating a new sub-info by type
    pub const fn new_u8() -> Self {
        Self {
            size: 1,
            data_type: DataType::UInt8,
            access_type: AccessType::Ro,
            pdo_mapping: PdoMapping::None,
            persist: false,
        }
    }

    /// Convenience function to set the access_type to read-only
    pub const fn ro_access(mut self) -> Self {
        self.access_type = AccessType::Ro;
        self
    }

    /// Convenience function to set the access_type to read-write
    pub const fn rw_access(mut self) -> Self {
        self.access_type = AccessType::Rw;
        self
    }

    /// Convenience function to set the access_type to const
    pub const fn const_access(mut self) -> Self {
        self.access_type = AccessType::Const;
        self
    }

    /// Convenience function to set the access_type to write-only
    pub const fn wo_access(mut self) -> Self {
        self.access_type = AccessType::Wo;
        self
    }

    /// Convenience function to set the persist value
    pub const fn persist(mut self, value: bool) -> Self {
        self.persist = value;
        self
    }
}

/// Lookup an object from the Object dictionary table
///
/// Note: `table` must be sorted by index
pub fn find_object<'a, 'b>(table: &'b [ODEntry<'a>], index: u16) -> Option<&'b ObjectData<'a>> {
    find_object_entry(table, index).map(|entry| &entry.data)
}

/// Lookup an entry from the object dictionary table
///
/// The same as [find_object], except that it returned the `&ODEntry` instead of the `&ObjectData`
/// it owns
///
/// Note: `table` must be sorted by index
pub fn find_object_entry<'a, 'b>(table: &'b [ODEntry<'a>], index: u16) -> Option<&'b ODEntry<'a>> {
    table
        .binary_search_by_key(&index, |e| e.index)
        .ok()
        .map(|i| &table[i])
}
