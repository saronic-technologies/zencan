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

pub trait VarObject : ObjectRawAccess + Sync {
}

pub trait RecordObject : ObjectRawAccess + Sync {
}

pub trait ArrayObject : ObjectRawAccess + Sync {
}

#[derive(Default)]
pub struct CallbackObject {
    write_cb: AtomicCell<Option<WriteHookFn>>,
    read_cb: AtomicCell<Option<ReadHookFn>>,
    info_cb: AtomicCell<Option<InfoHookFn>>,
    context: AtomicCell<Option<&'static dyn Context>>,
    object_code: ObjectCode,
}

impl CallbackObject {
    pub const fn new(
        object_code: ObjectCode,
    ) -> Self {
        Self {
            write_cb: AtomicCell::new(None),
            read_cb: AtomicCell::new(None),
            info_cb: AtomicCell::new(None),
            context: AtomicCell::new(None),
            object_code,
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

// pub trait Context: Any {}

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

impl<'a> ODEntry<'a> {
    // pub fn obj_code(&self) -> ObjectCode {
    //     match &self.data {
    //         ObjectData::Var(_) => ObjectCode::Var,
    //         ObjectData::Array(_) => ObjectCode::Array,
    //         ObjectData::Record(_) => ObjectCode::Record,
    //         ObjectData::Callback(obj) => obj.object_code,
    //     }
    // }

    // pub fn sub_count(&self) -> u8 {
    //     match &self.data {
    //         ObjectData::Var(_) => 1,
    //         ObjectData::Array(array) => (array.size() + 1) as u8,
    //         ObjectData::Record(_) => todo!(),
    //         ObjectData::Callback(_) => todo!(),
    //     }
    // }
}

unsafe impl<'a> Sync for ODEntry<'a> {}

// pub struct SubObject<'a, 'b> {
//     // NOTE: Maybe ObjectStorage here should actually be an Arc<Mutex<>>; we will need some locking
//     // at some point for threaded access, and maybe it should be at the sub object level rather than
//     // a single mutex on the whole dict...
//     storage: &'b Mutex<RefCell<ObjectStorage<'a>>>,
//     /// The offset into the storage at which this sub element is stored (e.g. for arrays and
//     /// records)
//     offset: usize,
//     pub size: usize,
//     pub data_type: DataType,
//     pub access_type: AccessType,
// }

// impl<'a, 'b> SubObject<'a, 'b> {
//     fn write(&self, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
//         let offset = offset + self.offset;
//         critical_section::with(|cs| match *self.storage.borrow_ref_mut(cs) {
//             ObjectStorage::Ram(ptr, size) => {
//                 let slice = unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, size) };
//                 if offset + data.len() > size {
//                     return Err(AbortCode::DataTypeMismatchLengthHigh);
//                 }
//                 slice[offset..offset + data.len()].copy_from_slice(data);
//                 Ok(())
//             }
//             ObjectStorage::Const(_, _) => Err(AbortCode::ReadOnly),
//             ObjectStorage::App(ref mut cb) => {
//                 if let Some(write) = cb.write.as_mut() {
//                     write(offset, data);
//                 }
//                 Ok(())
//             }
//         })

//     }

//     /// Read raw bytes from the object
//     ///
//     /// This function will panic on invalid read. It is up to the caller to ensure that the size
//     /// and offset fall within the range of the object
//     fn read(&self, offset: usize, buf: &mut [u8]) {
//         let offset = offset + self.offset;
//         critical_section::with(|cs| match *self.storage.borrow(cs).borrow() {
//             ObjectStorage::Ram(ptr, size) => {
//                 let slice = unsafe { core::slice::from_raw_parts(ptr, size) };
//                 buf.copy_from_slice(&slice[offset..offset + buf.len()])
//             }
//             ObjectStorage::Const(ptr, size) => {
//                 let slice = unsafe { core::slice::from_raw_parts(ptr, size) };
//                 buf.copy_from_slice(&slice[offset..offset + buf.len()])
//             }
//             ObjectStorage::App(ref callbacks) => {
//                 if let Some(read) = callbacks.read {
//                     read(offset, buf);
//                 } else {
//                     panic!("Attempted to read from DOMAIN with no registered callback");
//                 }
//             }
//         })
//     }

//     /// Get the size of the current value in the sub object
//     ///
//     /// For string types, this can be shorter than the allocated size.
//     pub fn current_size(&self) -> usize {
//         const CHUNK_SIZE: usize = 8;
//         if self.data_type.is_str() {
//             // Look for first 0
//             let mut chunk = 0;
//             let mut buf = [0; CHUNK_SIZE];

//             while chunk < self.size / CHUNK_SIZE + 1 {
//                 let offset = chunk * CHUNK_SIZE;
//                 let bytes_to_read = (self.size - offset).min(CHUNK_SIZE);
//                 self.read(offset, &mut buf[0..bytes_to_read]);

//                 if let Some(zero_pos) = buf[0..bytes_to_read].iter().position(|b| *b == 0) {
//                     return zero_pos + chunk * CHUNK_SIZE;
//                 }
//                 chunk += 1;
//             }
//             self.size
//         } else {
//             self.size
//         }
//     }
// }

// pub trait ObjectDictTrait<'table, 'cb> {
//     fn find(&self, index: u16) -> Option<&Object<'table, 'cb>>;
// }

// pub struct Object<'table> {
//     entry: &'table ODEntry<'table>,
// }

pub struct SubInfo {
    /// The size (or max size) of this sub object, in bytes
    pub size: usize,
    /// The data type of this sub object
    pub data_type: DataType,
    /// Indicates what accesses (i.e. read/write) are allowed on this sub object
    pub access_type: AccessType,
}

// impl<'table> Object<'table> {

//     /// Write data into a subobject
//     pub fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), AbortCode> {
//         if let Some(hook) = self.callbacks.write {
//             (hook)(&self.callbacks.context, self, sub, offset, data)
//         } else {
//             let sub = self.get_sub(sub).ok_or(AbortCode::NoSuchSubIndex)?;
//             sub.write(offset, data)?;
//             Ok(())
//         }
//     }

//     /// Read data from a subobj
//     ///
//     /// This function will panic if the read size is not valid. It is the responsibility of the
//     /// caller to validate the size before reading.
//     pub fn read(&self, sub: u8, offset: usize, data: &mut [u8]) -> Result<(), AbortCode> {
//         if let Some(hook) = self.callbacks.read {
//             (hook)(&self.callbacks.context, self, sub, offset, data)
//         } else {
//             let sub = self.get_sub(sub).ok_or(AbortCode::NoSuchSubIndex)?;
//             sub.read(offset, data);
//             Ok(())
//         }
//     }


//     /// Get metadata about one of the sub objects in this object
//     ///
//     /// Returns None if the provide sub index is invalid
//     ///
//     /// # Arguments
//     /// - `sub`: The sub index to request
//     pub fn sub_info(&self, sub: u8) -> Option<SubInfo> {
//         let sub = self.get_sub(sub)?;
//         Some(SubInfo {
//             size: sub.size,
//             current_size: sub.current_size(),
//             data_type: sub.data_type,
//             access_type: sub.access_type,
//         })
//     }

//     fn get_sub<'c>(&'c self, sub: u8) -> Option<SubObject<'table, 'c>> {
//         match &self.entry.data {
//             ObjectData::Var(var) => {
//                 if sub == 0 {
//                     Some(SubObject {
//                         offset: 0,
//                         data_type: var.data_type,
//                         access_type: var.access_type,
//                         size: var.size,
//                         storage: &var.storage,
//                     })
//                 } else {
//                     None
//                 }
//             }
//             ObjectData::Array(arr) => {
//                 if sub == 0 {
//                     Some(SubObject {
//                         offset: 0,
//                         data_type: DataType::UInt8,
//                         access_type: AccessType::Ro,
//                         size: 1,
//                         storage: &arr.storage_sub0,
//                     })
//                 } else if sub <= arr.size as u8 {
//                     Some(SubObject {
//                         offset: (sub as usize - 1) * element_storage_size(arr.data_type),
//                         data_type: arr.data_type,
//                         access_type: arr.access_type,
//                         size: element_storage_size(arr.data_type),
//                         storage: &arr.storage,
//                     })
//                 } else {
//                     None
//                 }
//             }
//             ObjectData::Record(rec) => {
//                 if sub == 0 {
//                     Some(SubObject {
//                         offset: 0,
//                         data_type: DataType::UInt8,
//                         access_type: AccessType::Const,
//                         size: 1,
//                         storage: &rec.storage_sub0,
//                     })
//                 } else if sub as usize <= rec.storage.len() {
//                     let storage = rec.storage.get(sub as usize - 1)?;
//                     if let Some(storage) = storage {
//                         Some(SubObject {
//                             offset: 0,
//                             // unwrap safety: if storage is not None, data type must be not None
//                             data_type: rec.data_types[sub as usize - 1].unwrap(),
//                             access_type: rec.access_types[sub as usize - 1].unwrap(),
//                             size: rec.sizes[sub as usize - 1],
//                             storage,
//                         })
//                     } else {
//                         None
//                     }
//                 } else {
//                     None
//                 }
//             }
//         }
//     }

// }
/// Builder to create an object dict from a table of ODEntry objects
///
/// The main function of the builder is to enforce the registration of callback hooks is done
/// up front so that they can be immutable during operation of the dictionary.
// pub struct ObjectDictBuilder<'a, 'b, const N: usize> {
//     pub table: [InternalEntry<'a, 'b>; N],
// }

// impl<'a, 'b, const N: usize> ObjectDictBuilder<'a, 'b, N> {
//     fn new(table: &[ODEntry<'a>; N]) -> Self {
//         let table_int = core::array::from_fn(|i|
//             InternalEntry {
//                 entry: &table[i],
//                 callbacks: Default::default()
//             }
//         );
//         Self {
//             table: table_int,
//         }
//     }

//     fn get_entry_mut(&mut self, index: u16) -> Option<&mut InternalEntry> {
//         for item in &mut self.table {
//             if item.entry.index == index {
//                 return Some(item)
//             }
//         }
//         return None
//     }

//     pub fn register_hook(
//         &mut self,
//         index: u16,
//         ctx: Option<&'a mut dyn Context>,
//         read_callback: Option<ReadHookFn>,
//         write_callback: Option<WriteHookFn>,

//     ) {
//         let entry = self.get_entry_mut(index).expect("Invalid index used in register hook");
//         entry.callbacks.context = ctx;
//         entry.callbacks.read = read_callback;
//         entry.callbacks.write = write_callback;
//     }

//     pub fn build(self) -> ObjectDict<'a> {
//         todo!()
//     }
// }


    // pub fn find_entry<'c>(&self, index: u16) -> Option<&'c ODEntry<'a>> {
    //     for i in 0..self.table.len() {
    //         if self.table[i].index == index {
    //             return Some(&self.table[i]);
    //         }
    //     }
    //     None
    // }

    // pub fn find_sub<'c>(&'b self, index: u16, sub: u8) -> Option<SubObject<'c, 'a>> {
    //     let entry = self.find_entry(index)?;
    //     entry.data.get_sub(sub)
    // }

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
    use super::*;


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
