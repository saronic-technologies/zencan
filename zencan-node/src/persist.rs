use core::{
    cell::RefCell,
    convert::Infallible,
    future::Future,
    pin::{pin, Pin},
    task::Context,
};

use futures::{pending, task::noop_waker_ref};
use zencan_common::objects::{find_object, ODEntry, ObjectRawAccess};

use defmt_or_log::{info, warn};

/// Specifies the types of nodes which can be serialized to persistent storage
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum NodeType {
    /// A node containing a saved sub-object value
    ObjectValue = 1,
    /// An unrecognized node type
    Unknown,
}

impl NodeType {
    /// Create a `NodeType` from an ID byte
    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => Self::ObjectValue,
            _ => Self::Unknown,
        }
    }
}

/// Top-level node configuration which is persisted
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeConfig {
    /// The configured node ID
    pub node_id: u8,
    /// The configured baud table
    pub baud_table: u8,
    /// The index into the baud table of the configured baud rate
    pub baud_index: u8,
}

async fn write_bytes(bytes: &[u8], reg: &RefCell<u8>) {
    for b in bytes {
        *reg.borrow_mut() = *b;
        pending!()
    }
}

async fn serialize_object(obj: &ODEntry<'_>, sub: u8, reg: &RefCell<u8>) {
    // Unwrap safety: This can only fail if the sub doesn't exist, and we already
    // checked for that above
    let data_size = obj.data.current_size(sub).unwrap() as u16;
    // Serialized node size is the variable length object data, plus node type (u8), index (u16), and sub index (u8)
    let node_size = data_size + 4;

    println!(
        "serializing {} bytes for {:x}sub{}",
        node_size, obj.index, sub
    );
    write_bytes(&node_size.to_le_bytes(), reg).await;
    write_bytes(&[NodeType::ObjectValue as u8], reg).await;
    write_bytes(&obj.index.to_le_bytes(), reg).await;
    write_bytes(&[sub], reg).await;

    let mut buf = [0u8];
    for i in 0..data_size {
        obj.data.read(sub, i as usize, &mut buf).unwrap();
        write_bytes(&buf, reg).await;
    }
}

async fn serialize_sm(objects: &[ODEntry<'static>], reg: &RefCell<u8>) {
    for obj in objects {
        let max_sub = obj.data.max_sub_number();
        if max_sub > 0 {
            // This is an array or record. We don't store sub 0, which holds the max_sub_index, but
            // will store any remaining subs which are marked as stored
            for i in 0..max_sub {
                let sub = i + 1;
                let info = obj.data.sub_info(sub);
                // On a record, some subs may not be present. Just skip these.
                if info.is_err() {
                    continue;
                }
                let info = info.unwrap();
                if !info.persist {
                    continue;
                }
                serialize_object(obj, sub, reg).await;
            }
        } else {
            let info = obj.data.sub_info(0).expect("var object must have sub 0");
            if !info.persist {
                continue;
            }
            serialize_object(obj, 0, reg).await;
        }
    }
}

struct PersistSerializer<'a, 'b, F: Future> {
    f: Pin<&'a mut F>,
    reg: &'b RefCell<u8>,
}

impl<'a, 'b, F: Future> PersistSerializer<'a, 'b, F> {
    pub fn new(f: Pin<&'a mut F>, reg: &'b RefCell<u8>) -> Self {
        Self { f, reg }
    }
}

impl<F: Future> embedded_io::ErrorType for PersistSerializer<'_, '_, F> {
    type Error = Infallible;
}

impl<F: Future> embedded_io::Read for PersistSerializer<'_, '_, F> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Infallible> {
        let mut cx = Context::from_waker(noop_waker_ref());

        let mut pos = 0;
        loop {
            if pos >= buf.len() {
                return Ok(pos);
            }

            match self.f.as_mut().poll(&mut cx) {
                core::task::Poll::Ready(_) => return Ok(pos),
                core::task::Poll::Pending => {
                    buf[pos] = *self.reg.borrow();
                    pos += 1;
                }
            }
        }
    }
}

pub fn serialized_size(objects: &[ODEntry]) -> usize {
    let mut size = 0;
    for obj in objects {
        let max_sub = obj.data.max_sub_number();
        if max_sub > 0 {
            // This is an array or record. We don't store sub 0, which holds the max_sub_index, but
            // will store any remaining subs which are marked as persisted
            for i in 0..max_sub {
                let sub = i + 1;
                let info = obj.data.sub_info(sub);
                // On a record, some subs may not be present. Just skip these.
                if info.is_err() {
                    continue;
                }
                let info = info.unwrap();
                if !info.persist {
                    continue;
                }
                // Unwrap safety: This can only fail if the sub doesn't exist, and we already
                // checked for that above
                let data_size = obj.data.current_size(sub).unwrap();
                // Serialized node size is the variable length object data, plus node type (u8),
                // index (u16), and sub index (u8), plus a length header (u16)
                size += data_size + 6;
            }
        }
    }

    size
}

/// Serialize node data
pub fn serialize<F: Fn(&mut dyn embedded_io::Read<Error = Infallible>, usize)>(
    od: &'static [ODEntry],
    callback: F,
) {
    let reg = RefCell::new(0);
    let fut = pin!(serialize_sm(od, &reg));
    let mut serializer = PersistSerializer::new(fut, &reg);
    let size = serialized_size(od);
    callback(&mut serializer, size)
}

/// Error which can be returned while reading persisted data
pub enum PersistReadError {
    /// Not enough bytes were present to construct the node
    NodeLengthShort,
}

/// The data for an ObjectValue node
#[derive(Debug, PartialEq)]
pub struct ObjectValue<'a> {
    /// The object index this value belongs to
    pub index: u16,
    /// The sub-object index this value belongs to
    pub sub: u8,
    /// The raw bytes to be restored to the sub object
    pub data: &'a [u8],
}

/// A reference to a single node within a slice of serialized data
///
/// Returned by the PersistNodeReader iterator.
#[derive(Debug, PartialEq)]
pub enum PersistNodeRef<'a> {
    /// A saved value for a sub-object
    ObjectValue(ObjectValue<'a>),
    /// An unrecognized node type was encountered. Either the serialized data is malformed, or
    /// perhaps it was written with a future version of code that supports more node types
    ///
    /// The bytes of the node are stored in the contained slice, including the node type in the
    /// first byte
    Unknown(&'a [u8]),
}

impl<'a> PersistNodeRef<'a> {
    /// Create a PersistNodeRef from a slice of bytes
    pub fn from_slice(data: &'a [u8]) -> Result<Self, PersistReadError> {
        if data.is_empty() {
            return Err(PersistReadError::NodeLengthShort);
        }

        match NodeType::from_byte(data[0]) {
            NodeType::ObjectValue => {
                if data.len() < 5 {
                    return Err(PersistReadError::NodeLengthShort);
                }
                Ok(Self::ObjectValue(ObjectValue {
                    index: u16::from_le_bytes(data[1..3].try_into().unwrap()),
                    sub: data[3],
                    data: &data[4..],
                }))
            }
            NodeType::Unknown => Ok(PersistNodeRef::Unknown(data)),
        }
    }
}

/// Read serialized object data from a slice of bytes
///
/// PersistNodeReader provides an Iterator of PersistNodeRef objects, representing all of the nodes
/// stored in the slice
struct PersistNodeReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> PersistNodeReader<'a> {
    /// Instantiate a PersistNodeReader from a slice of serialized data
    pub fn new(data: &'a [u8]) -> Self {
        Self { buf: data, pos: 0 }
    }
}

impl<'a> Iterator for PersistNodeReader<'a> {
    type Item = PersistNodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf.len() - self.pos < 2 {
            return None;
        }
        let length = u16::from_le_bytes(self.buf[self.pos..self.pos + 2].try_into().unwrap());
        println!("Read node of len {length}");
        self.pos += 2;
        let node_slice = &self.buf[self.pos..self.pos + length as usize];
        self.pos += length as usize;

        PersistNodeRef::from_slice(node_slice).ok()
    }
}

/// Load values of objects previously persisted in serialized format
///
/// # Arguments
/// - `od`: The object dictionary where objects will be updated
/// - `stored_data`: A slice of bytes, as previously provided to the store_objects callback.
pub fn restore_stored_objects(od: &[ODEntry], stored_data: &[u8]) {
    let reader = PersistNodeReader::new(stored_data);
    for item in reader {
        match item {
            PersistNodeRef::ObjectValue(restore) => {
                if let Some(obj) = find_object(od, restore.index) {
                    if let Ok(sub_info) = obj.sub_info(restore.sub) {
                        info!(
                            "Restoring 0x{:x}sub{} with {:?}",
                            restore.index, restore.sub, restore.data
                        );
                        if let Err(abort_code) = obj.write(restore.sub, 0, restore.data) {
                            warn!(
                                "Error restoring object 0x{:x}sub{}: {:x}",
                                restore.index, restore.sub, abort_code as u32
                            );
                        }
                        // Null terminate short strings when restoring
                        if sub_info.data_type.is_str() && restore.data.len() < sub_info.size {
                            obj.write(restore.sub, restore.data.len(), &[0])
                                .expect("Error null terminated restored string");
                        }
                    } else {
                        warn!(
                            "Saved object 0x{:x}sub{} not found in OD",
                            restore.index, restore.sub
                        );
                    }
                } else {
                    warn!("Saved object 0x{:x} not found in OD", restore.index);
                }
            }
            PersistNodeRef::Unknown(id) => warn!("Unknown persisted object read: {}", id[0]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zencan_common::objects::ODEntry;
    use zencan_macro::record_object;

    // The `record_object` macro output references `zencan_node`, so in the context of the
    // zencan_node crate, we have to provide this name
    use crate::{self as zencan_node, persist::serialize};

    #[test]
    fn test_serialize_deserialize() {
        #[derive(Debug, Default)]
        #[record_object]
        struct Object100 {
            #[record(persist)]
            value1: u32,
            value2: u16,
        }

        #[derive(Debug, Default)]
        #[record_object]
        struct Object200 {
            #[record(persist)]
            string: [u8; 15],
        }

        let inst100 = Box::leak(Box::new(Object100::default()));
        let inst200 = Box::leak(Box::new(Object200::default()));

        let od = Box::leak(Box::new([
            ODEntry {
                index: 0x100,
                data: zencan_common::objects::ObjectData::Storage(inst100),
            },
            ODEntry {
                index: 0x200,
                data: zencan_common::objects::ObjectData::Storage(inst200),
            },
        ]));
        inst100.set_value1(42);
        inst200.set_string("test".as_bytes());

        let data = RefCell::new(Vec::new());
        serialize(od, |reader, _size| {
            const CHUNK_SIZE: usize = 2;
            let mut buf = [0; CHUNK_SIZE];
            loop {
                let n = reader.read(&mut buf).unwrap();
                data.borrow_mut().extend_from_slice(&buf[..n]);
                if n < buf.len() {
                    break;
                }
            }
        });

        let data = data.take();
        assert_eq!(20, data.len());
        assert_eq!(data.len(), serialized_size(od));

        let mut deser = PersistNodeReader::new(&data);
        assert_eq!(
            deser.next().unwrap(),
            PersistNodeRef::ObjectValue(ObjectValue {
                index: 0x100,
                sub: 1,
                data: &42u32.to_le_bytes()
            })
        );
        assert_eq!(
            deser.next().unwrap(),
            PersistNodeRef::ObjectValue(ObjectValue {
                index: 0x200,
                sub: 1,
                data: "test".as_bytes()
            })
        );
        assert_eq!(deser.next(), None);
    }
}
