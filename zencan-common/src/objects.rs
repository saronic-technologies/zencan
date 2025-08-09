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

/// Trait for accessing object flags
pub trait ObjectFlagAccess {
    /// Set the flag for the specified sub object
    ///
    /// The flag is set on the currently active flag set
    fn set_flag(&self, sub: u8);
    /// Read the flag for the specified object
    ///
    /// The flag is read from the currently inactive flag set, i.e. the flag value from before the
    /// last sync toggle is returned
    fn get_flag(&self, sub: u8) -> bool;
    /// Clear all flags in the currently active flag set
    fn clear(&self);
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
}

impl<const N: usize> ObjectFlagAccess for ObjectFlags<N> {
    fn set_flag(&self, sub: u8) {
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

    fn get_flag(&self, sub: u8) -> bool {
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

    fn clear(&self) {
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
pub enum ObjectCode {
    /// An empty object
    ///
    /// Zencan does not support Null objects
    Null = 0,
    /// A large chunk of data
    ///
    /// Zencan does not support Domain Object; it only supports domain sub-objects.
    Domain = 2,
    /// Unused
    DefType = 5,
    /// Unused
    DefStruct = 6,
    /// An object which has a single sub object
    #[default]
    Var = 7,
    /// An array of sub-objects all with the same data type
    Array = 8,
    /// A collection of sub-objects with varying types
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
    /// The length of `data` must match the size of the object, or else it will fail with either
    /// [`AbortCode::DataTypeMismatchLengthLow`] or [`AbortCode::DataTypeMismatchLengthHigh`].
    ///
    /// If the sub is does not exist, it shall fail with [`AbortCode::NoSuchSubIndex`].
    ///
    /// If the sub exists but is not writeable, it shall fail with [`AbortCode::ReadOnly`].
    fn write(&self, sub: u8, data: &[u8]) -> Result<(), AbortCode>;

    /// Initialize a new partial write
    ///
    /// This must be called before performing calls to `partial_write`.
    ///
    /// A default implementation is provided which returns an appropriate error:
    /// - [`AbortCode::NoSuchSubIndex`] if the sub object does not exist
    /// - [`AbortCode::ReadOnly`] if the sub object is read-only
    /// - [`AbortCode::UnsupportedAccess`] if the sub object does not support partial writes
    ///
    /// Objects which support partial writing must override the default implementation.
    fn begin_partial(&self, sub: u8) -> Result<(), AbortCode> {
        if let Ok(sub_info) = self.sub_info(sub) {
            if sub_info.access_type.is_writable() {
                Err(AbortCode::UnsupportedAccess)
            } else {
                Err(AbortCode::ReadOnly)
            }
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    /// Perform a partial write of bytes to a subobject
    ///
    /// Most objects do not support partial writes. But in some cases, such as DOMAINs, or very
    /// large string objects, it is unavoidable and these must support it.
    ///
    /// Partial writes MUST be done sequentially, and implementers may assume that this is the case.
    /// Executing multiple concurrent partial writes to the same sub object is not supported. It is
    /// up to the application to ensure that this is not done.
    fn write_partial(&self, _sub: u8, _buf: &[u8]) -> Result<(), AbortCode> {
        // All callers should have failed at begin_partial, so this should never be called
        Err(AbortCode::GeneralError)
    }

    /// Finalize a previous partial write
    ///
    /// This must always be called after using partial_write, after all partial_write calls have
    /// been completed.
    fn end_partial(&self, _sub: u8) -> Result<(), AbortCode> {
        Err(AbortCode::GeneralError)
    }

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

/// OD placeholder for an object which will have a handler registered at runtime
pub struct CallbackObject<'a> {
    obj: AtomicCell<Option<&'a dyn ObjectRawAccess>>,
    object_code: ObjectCode,
}

impl CallbackObject<'_> {
    /// Create a new callback
    pub fn new(object_code: ObjectCode) -> Self {
        Self {
            obj: AtomicCell::new(None),
            object_code,
        }
    }
}

impl ObjectRawAccess for CallbackObject<'_> {
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        if let Some(obj) = self.obj.load() {
            obj.read(sub, offset, buf)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn write(&self, sub: u8, data: &[u8]) -> Result<(), AbortCode> {
        if let Some(obj) = self.obj.load() {
            obj.write(sub, data)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn write_partial(&self, sub: u8, buf: &[u8]) -> Result<(), AbortCode> {
        if let Some(obj) = self.obj.load() {
            obj.write_partial(sub, buf)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn end_partial(&self, sub: u8) -> Result<(), AbortCode> {
        if let Some(obj) = self.obj.load() {
            obj.end_partial(sub)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn object_code(&self) -> ObjectCode {
        self.object_code
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        if let Some(obj) = self.obj.load() {
            obj.sub_info(sub)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }
}
/// Object enum to be stored in the dictionary
pub enum ObjectData<'a> {
    /// This is a normal object with allocated storage, e.g. as created by `zencan-build`
    Storage(&'a dyn ObjectRawAccess),
    /// This is a callback object which must have callback functions registered at runtime for
    /// access
    Callback(&'a CallbackObject<'a>),
}

impl ObjectRawAccess for ObjectData<'_> {
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        match self {
            ObjectData::Storage(obj) => obj.read(sub, offset, buf),
            ObjectData::Callback(obj) => obj.read(sub, offset, buf),
        }
    }

    fn write(&self, sub: u8, data: &[u8]) -> Result<(), AbortCode> {
        match self {
            ObjectData::Storage(obj) => obj.write(sub, data),
            ObjectData::Callback(obj) => obj.write(sub, data),
        }
    }

    fn begin_partial(&self, sub: u8) -> Result<(), AbortCode> {
        match self {
            ObjectData::Storage(obj) => obj.begin_partial(sub),
            ObjectData::Callback(obj) => obj.begin_partial(sub),
        }
    }

    fn end_partial(&self, sub: u8) -> Result<(), AbortCode> {
        match self {
            ObjectData::Storage(obj) => obj.end_partial(sub),
            ObjectData::Callback(obj) => obj.end_partial(sub),
        }
    }

    fn write_partial(&self, sub: u8, buf: &[u8]) -> Result<(), AbortCode> {
        match self {
            ObjectData::Storage(obj) => obj.write_partial(sub, buf),
            ObjectData::Callback(obj) => obj.write_partial(sub, buf),
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

    /// Convenience function for creating a new sub-info by type
    pub const fn new_visibile_str(size: usize) -> Self {
        Self {
            size,
            data_type: DataType::VisibleString,
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

/// Allow transparent byte level access to a sub object
pub trait SubObjectAccess: Sync + Send + core::fmt::Debug {
    /// Read data from the sub object
    ///
    /// Read `buf.len()` bytes, starting at offset
    ///
    /// All sub objects are required to support partial read
    ///
    /// # Errors
    ///
    /// - [`AbortCode::DataTypeMismatchLengthHigh`] if `offset` + `buf.len()` exceeds the object
    ///   size
    /// - [`AbortCode::WriteOnly`] if the sub object does not support reading
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode>;
    /// Write data to the sub object
    ///
    /// For most objects, the length of data must match the size of the object exactly. However, for
    /// some objects, such as Domain, VisibleString, or UnicodeString, or objects with custom
    /// callback implementations it may be possible to write shorter values.
    ///
    /// # Errors
    ///
    /// - [`AbortCode::DataTypeMismatchLengthHigh`] if `data.len()` exceeds the object size
    /// - [`AbortCode::DataTypeMismatchLengthLow`] if `data.len()` is smaller than the object size
    ///   and the object does not support this
    /// - [`AbortCode::ReadOnly`] if the object does not support writing
    /// - [`AbortCode::InvalidValue`] if the value is not allowed
    /// - [`AbortCode::ValueTooHigh`] if the value is higher than the allowed range of this object
    /// - [`AbortCode::ValueTooLow`] if the value is lower than the allowed range of this object
    /// - [`AbortCode::ResourceNotAvailable`] if the object cannot be written because of the
    ///   application state. For example, this is returned if a required callback has not been
    ///   registered on the object.
    ///
    /// Other error types may be returned by special purpose objects implemented via custom
    /// callback.
    fn write(&self, data: &[u8]) -> Result<(), AbortCode>;

    /// Begin a multi-part write to the object
    ///
    /// Not all objects support partial writes. Primarily it is large objects which support it in
    /// order to allow transfer of the data in multiple blocks. It is up to the application to
    /// ensure that no other writes occur while a partial write is in progress, or else the object
    /// data may be corrupted and/or a call to `write_partial` may return an abort code on a
    /// subsequent call.
    ///
    /// Partial writes should always include the following, in this order:
    /// - One call to `begin_partial`
    /// - N calls to `write_partial`
    /// - One call to `end_partial`
    ///
    /// # Errors
    ///
    /// - [`AbortCode::UnsupportedAccess`] when the object does not support partial writes.
    fn begin_partial(&self) -> Result<(), AbortCode> {
        Err(AbortCode::UnsupportedAccess)
    }

    /// Write part of multi-part data to the object
    fn write_partial(&self, _buf: &[u8]) -> Result<(), AbortCode> {
        Err(AbortCode::UnsupportedAccess)
    }

    /// Finish a multi-part write
    fn end_partial(&self) -> Result<(), AbortCode> {
        Err(AbortCode::UnsupportedAccess)
    }
}

/// A sub object which contains a single scalar value of type T, which is a standard rust type
#[derive(Debug)]
pub struct ScalarField<T: Copy> {
    value: AtomicCell<T>,
}

impl<T: Send + Copy + PartialEq> ScalarField<T> {
    /// Atomically read the value of the field
    pub fn load(&self) -> T {
        self.value.load()
    }

    /// Atomically store a new value into the field
    pub fn store(&self, value: T) {
        self.value.store(value);
    }
}

impl<T: Copy + Default> Default for ScalarField<T> {
    fn default() -> Self {
        Self {
            value: AtomicCell::default(),
        }
    }
}

macro_rules! impl_scalar_field {
    ($rust_type: ty, $data_type: ty) => {
        impl ScalarField<$rust_type> {
            /// Create a new ScalarField with the given value
            pub const fn new(value: $rust_type) -> Self {
                Self {
                    value: AtomicCell::new(value),
                }
            }
        }
        impl SubObjectAccess for ScalarField<$rust_type> {
            fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
                let bytes = self.value.load().to_le_bytes();
                if offset + buf.len() > bytes.len() {
                    return Err(AbortCode::DataTypeMismatchLengthHigh);
                }
                buf.copy_from_slice(&bytes[offset..offset + buf.len()]);
                Ok(())
            }

            fn write(&self, data: &[u8]) -> Result<(), AbortCode> {
                let value = <$rust_type>::from_le_bytes(data.try_into().map_err(|_| {
                    if data.len() < size_of::<$rust_type>() {
                        AbortCode::DataTypeMismatchLengthLow
                    } else {
                        AbortCode::DataTypeMismatchLengthHigh
                    }
                })?);
                self.value.store(value);
                Ok(())
            }
        }
    };
}

impl_scalar_field!(u8, DataType::UInt8);
impl_scalar_field!(u16, DataType::UInt16);
impl_scalar_field!(u32, DataType::UInt32);
impl_scalar_field!(i8, DataType::Int8);
impl_scalar_field!(i16, DataType::Int16);
impl_scalar_field!(i32, DataType::Int32);
impl_scalar_field!(f32, DataType::Float);

// bool doesn't support from_le_bytes so it needs a special implementation
impl SubObjectAccess for ScalarField<bool> {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        let value = self.value.load();
        if offset != 0 || buf.len() > 1 {
            return Err(AbortCode::DataTypeMismatchLengthHigh);
        }
        buf[0] = if value { 1 } else { 0 };
        Ok(())
    }

    fn write(&self, data: &[u8]) -> Result<(), AbortCode> {
        if data.len() != 1 {
            return Err(AbortCode::DataTypeMismatchLengthHigh);
        }
        let value = data[0] != 0;
        self.value.store(value);
        Ok(())
    }
}

/// A byte field which supports storing short values using null termination to indicate size
///
/// This is here to support VisibleString and UnicodeString types.
#[allow(clippy::len_without_is_empty)]
#[derive(Debug)]
pub struct NullTermByteField<const N: usize>(ByteField<N>);

impl<const N: usize> NullTermByteField<N> {
    /// Create a new NullTermByteField with the provided value
    pub const fn new(value: [u8; N]) -> Self {
        Self(ByteField::new(value))
    }

    /// Return the size of the sub object
    pub fn len(&self) -> usize {
        N
    }

    /// Atomically load the value stored in the object
    ///
    /// Note that this will return the entire array, including any invalid bytes after the null
    /// terminator.
    pub fn load(&self) -> [u8; N] {
        self.0.load()
    }

    /// Atomically store a new value to the object
    pub fn store(&self, value: [u8; N]) {
        self.0.store(value);
    }

    /// Store a str to the object
    ///
    /// If the string is shorter than the object size, it will be stored with a null terminator
    /// If longer, an error will be returned.
    pub fn set_str(&self, value: &[u8]) -> Result<(), AbortCode> {
        self.0.begin_partial()?;
        self.0.write_partial(value)?;
        if value.len() < N {
            self.0.write_partial(&[0])?;
        }
        self.end_partial()?;
        Ok(())
    }
}

impl<const N: usize> Default for NullTermByteField<N> {
    fn default() -> Self {
        Self(ByteField::default())
    }
}

impl<const N: usize> SubObjectAccess for NullTermByteField<N> {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        self.0.read(offset, buf)
    }

    fn write(&self, data: &[u8]) -> Result<(), AbortCode> {
        self.0.begin_partial()?;
        self.0.write_partial(data)?;
        if data.len() < N {
            self.0.write_partial(&[0])?;
        }
        self.0.end_partial()?;
        Ok(())
    }

    fn begin_partial(&self) -> Result<(), AbortCode> {
        self.0.begin_partial()
    }

    fn write_partial(&self, data: &[u8]) -> Result<(), AbortCode> {
        self.0.write_partial(data)
    }

    fn end_partial(&self) -> Result<(), AbortCode> {
        // Null terminate if the length of data written is less than the sub object size
        if self.0.write_offset.load().unwrap_or(0) < N {
            self.0.write_partial(&[0])?;
        }
        self.0.end_partial()
    }
}

/// A sub object which contains a fixed-size byte array
///
/// This is the data storage backing for all string types
#[allow(clippy::len_without_is_empty)]
#[derive(Debug)]
pub struct ByteField<const N: usize> {
    value: UnsafeCell<[u8; N]>,
    write_offset: AtomicCell<Option<usize>>,
}

unsafe impl<const N: usize> Sync for ByteField<N> {}

impl<const N: usize> ByteField<N> {
    /// Create a new ByteField with the provided value
    pub const fn new(value: [u8; N]) -> Self {
        Self {
            value: UnsafeCell::new(value),
            write_offset: AtomicCell::new(None),
        }
    }

    /// Get the size of the ByteField
    pub fn len(&self) -> usize {
        N
    }

    /// Atomically store a new value to the sub object
    pub fn store(&self, value: [u8; N]) {
        // Any ongoing partial write will be cancelled
        self.write_offset.store(None);
        critical_section::with(|_| {
            let bytes = unsafe { &mut *self.value.get() };
            bytes.copy_from_slice(&value);
        });
    }

    /// Atomically read the value of the sub object
    pub fn load(&self) -> [u8; N] {
        critical_section::with(|_| unsafe { *self.value.get() })
    }
}

impl<const N: usize> Default for ByteField<N> {
    fn default() -> Self {
        Self {
            value: UnsafeCell::new([0; N]),
            write_offset: AtomicCell::new(None),
        }
    }
}

impl<const N: usize> SubObjectAccess for ByteField<N> {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        critical_section::with(|_| {
            let bytes = unsafe { &*self.value.get() };
            if offset + buf.len() > bytes.len() {
                return Err(AbortCode::DataTypeMismatchLengthHigh);
            }
            buf.copy_from_slice(&bytes[offset..offset + buf.len()]);
            Ok(())
        })
    }

    fn write(&self, data: &[u8]) -> Result<(), AbortCode> {
        critical_section::with(|_| {
            let bytes = unsafe { &mut *self.value.get() };
            if data.len() > bytes.len() {
                return Err(AbortCode::DataTypeMismatchLengthHigh);
            }
            bytes[..data.len()].copy_from_slice(data);
            Ok(())
        })
    }

    fn begin_partial(&self) -> Result<(), AbortCode> {
        self.write_offset.store(Some(0));
        Ok(())
    }

    fn write_partial(&self, buf: &[u8]) -> Result<(), AbortCode> {
        // Unwrap: fetch_update can only fail if the closure returns None
        let offset = self
            .write_offset
            .fetch_update(|old| Some(old.map(|x| x + buf.len())))
            .unwrap();
        if offset.is_none() {
            return Err(AbortCode::GeneralError);
        }
        let offset = offset.unwrap();
        if offset + buf.len() > N {
            return Err(AbortCode::DataTypeMismatchLengthHigh);
        }
        critical_section::with(|_| {
            let bytes = unsafe { &mut *self.value.get() };
            bytes[offset..offset + buf.len()].copy_from_slice(buf);
        });
        Ok(())
    }

    fn end_partial(&self) -> Result<(), AbortCode> {
        // No finalization action needed for byte fields
        self.write_offset.store(None);
        Ok(())
    }
}

#[derive(Debug)]
/// A struct for a constant sub object whose value never changes
///
/// For simplicity, the value is stored directly as bytes, so use `to_le_bytes` when creating the
/// const object.
pub struct ConstField<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> ConstField<N> {
    /// Create a const field
    pub const fn new(bytes: [u8; N]) -> Self {
        Self { bytes }
    }
}

impl<const N: usize> SubObjectAccess for ConstField<N> {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        if offset + buf.len() > self.bytes.len() {
            return Err(AbortCode::DataTypeMismatchLengthHigh);
        }
        buf.copy_from_slice(&self.bytes[offset..offset + buf.len()]);
        Ok(())
    }

    fn write(&self, _data: &[u8]) -> Result<(), AbortCode> {
        Err(AbortCode::ReadOnly)
    }
}

/// A subobject which is a place holder for a handler to be registered at runtime
#[derive(Debug)]
pub struct CallbackSubObject {
    handler: AtomicCell<Option<&'static dyn SubObjectAccess>>,
}

impl CallbackSubObject {
    /// Create a new object
    pub const fn new() -> Self {
        Self {
            handler: AtomicCell::new(None),
        }
    }

    /// Register a handler for this sub object
    pub fn register_handler(&self, handler: &'static dyn SubObjectAccess) {
        self.handler.store(Some(handler));
    }
}

impl SubObjectAccess for CallbackSubObject {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        if let Some(handler) = self.handler.load() {
            handler.read(offset, buf)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn write(&self, data: &[u8]) -> Result<(), AbortCode> {
        if let Some(handler) = self.handler.load() {
            handler.write(data)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn begin_partial(&self) -> Result<(), AbortCode> {
        if let Some(handler) = self.handler.load() {
            handler.begin_partial()
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn write_partial(&self, buf: &[u8]) -> Result<(), AbortCode> {
        if let Some(handler) = self.handler.load() {
            handler.write_partial(buf)
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }

    fn end_partial(&self) -> Result<(), AbortCode> {
        if let Some(handler) = self.handler.load() {
            handler.end_partial()
        } else {
            Err(AbortCode::ResourceNotAvailable)
        }
    }
}

/// A trait for structs which represent Objects to implement
///
/// Implementing this type allows a type sub object which implements [`SubObjectAccess`] to
/// implement [`ObjectRawAccess`] simply by implementing this trait to provide a sub object for each
/// sub index.
pub trait ProvidesSubObjects {
    /// Get a slice of sub objects
    ///
    /// This is used for objects which have a fixed number of sub objects, such as arrays or records.
    /// The slice must be at least as long as the maximum sub index of the object.
    ///
    /// It should return None if the sub object does not exist, and when it does exist it returns a
    /// tuple containing a [`SubInfo`] with metadata about the sub object, and [`dyn
    /// SubObjectAccess`] which provides read/write access to the sub object data.
    fn get_sub_object(&self, sub: u8) -> Option<(SubInfo, &dyn SubObjectAccess)>;

    /// Get the object flags for this object
    ///
    /// Notification flags are supported by some objects to indicate changes made in the object by
    /// the application -- for example, to trigger the transmission of a mapped PDO.
    ///
    /// If the object supports flags, it should override this method to return a reference to them
    fn flags(&self) -> Option<&dyn ObjectFlagAccess> {
        None
    }

    /// What type of object is this
    fn object_code(&self) -> ObjectCode;
}

impl<T: ProvidesSubObjects + Sync + Send> ObjectRawAccess for T {
    fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), AbortCode> {
        if let Some((info, access)) = self.get_sub_object(sub) {
            if info.access_type.is_readable() {
                access.read(offset, buf)
            } else {
                Err(AbortCode::WriteOnly)
            }
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    fn write(&self, sub: u8, data: &[u8]) -> Result<(), AbortCode> {
        if let Some((info, access)) = self.get_sub_object(sub) {
            if info.access_type.is_writable() {
                access.write(data)
            } else {
                Err(AbortCode::ReadOnly)
            }
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    fn begin_partial(&self, sub: u8) -> Result<(), AbortCode> {
        if let Some((info, access)) = self.get_sub_object(sub) {
            if info.access_type.is_writable() {
                access.begin_partial()
            } else {
                Err(AbortCode::ReadOnly)
            }
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    fn write_partial(&self, sub: u8, buf: &[u8]) -> Result<(), AbortCode> {
        if let Some((_, access)) = self.get_sub_object(sub) {
            access.write_partial(buf)
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    fn end_partial(&self, sub: u8) -> Result<(), AbortCode> {
        if let Some((_, access)) = self.get_sub_object(sub) {
            access.end_partial()
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }

    fn set_event_flag(&self, sub: u8) -> Result<(), AbortCode> {
        if let Some(flags) = self.flags() {
            flags.set_flag(sub);
            Ok(())
        } else {
            Err(AbortCode::UnsupportedAccess)
        }
    }

    fn read_event_flag(&self, sub: u8) -> bool {
        if let Some(flags) = self.flags() {
            flags.get_flag(sub)
        } else {
            false
        }
    }

    fn object_code(&self) -> ObjectCode {
        self.object_code()
    }

    fn sub_info(&self, sub: u8) -> Result<SubInfo, AbortCode> {
        if let Some((info, _access)) = self.get_sub_object(sub) {
            Ok(info)
        } else {
            Err(AbortCode::NoSuchSubIndex)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct ExampleRecord {
        val1: ScalarField<u32>,
        val2: ScalarField<bool>,
        val3: NullTermByteField<10>,
    }

    impl ProvidesSubObjects for ExampleRecord {
        fn get_sub_object(&self, sub: u8) -> Option<(SubInfo, &dyn SubObjectAccess)> {
            match sub {
                0 => Some((
                    SubInfo::MAX_SUB_NUMBER,
                    const { &ConstField::new(3u8.to_le_bytes()) },
                )),
                1 => Some((SubInfo::new_u32().rw_access(), &self.val1)),
                2 => Some((SubInfo::new_u8().rw_access(), &self.val2)),
                3 => Some((
                    SubInfo::new_visibile_str(self.val3.len()).rw_access(),
                    &self.val3,
                )),
                _ => None,
            }
        }

        fn object_code(&self) -> ObjectCode {
            ObjectCode::Record
        }
    }

    #[test]
    fn test_record_with_provides_sub_objects() {
        let record = ExampleRecord::default();

        assert_eq!(3, record.read_u8(0).unwrap());
        record.write(1, &42u32.to_le_bytes()).unwrap();
        assert_eq!(42, record.read_u32(1).unwrap());

        record.begin_partial(3).unwrap();
        // Do a write of the full length of the byte field
        record
            .write_partial(3, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
            .unwrap();
        let mut buf = [0; 10];
        record.read(3, 0, &mut buf).unwrap();
        assert_eq!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9], buf);
        // Do a write smaller than the size, and make sure it gets null terminated
        record.begin_partial(3).unwrap();
        record.write_partial(3, &[0, 1, 2, 3]).unwrap();
        record.write_partial(3, &[4, 5, 6, 7]).unwrap();
        record.end_partial(3).unwrap();
        let mut buf = [0; 9];
        record.read(3, 0, &mut buf).unwrap();
        assert_eq!([0u8, 1, 2, 3, 4, 5, 6, 7, 0], buf)
    }
}
