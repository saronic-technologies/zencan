use core::{
    cell::RefCell,
    future::Future,
    pin::{pin, Pin},
    task::Context,
};

use futures::{pending, task::noop_waker_ref};
use zencan_common::objects::{ODEntry, ObjectRawAccess};

/// Specifies the types of nodes which can be serialized to persistent storage
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum NodeType {
    /// A node containing the top-level node configuration
    NodeConfig = 0,
    /// A node containing a saved sub-object value
    ObjectValue = 1,
    /// An unrecognized node type
    Unknown,
}

impl NodeType {
    /// Create a `NodeType` from an ID byte
    pub fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::NodeConfig,
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

async fn serialize_sm(node_config: &NodeConfig, objects: &[ODEntry<'static>], reg: &RefCell<u8>) {
    write_bytes(
        &[
            4, // u16 length in little endian
            0,
            NodeType::NodeConfig as u8,
            node_config.node_id,
            node_config.baud_table,
            node_config.baud_index,
        ],
        reg,
    )
    .await;
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

                // Unwrap safety: This can only fail if the sub doesn't exist, and we already
                // checked for that above
                let data_size = obj.data.current_size(sub).unwrap() as u16;
                // Serialized node size is the variable length object data, plus node type (u8), index (u16), and sub index (u8)
                let node_size = data_size + 4;

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
        }
    }
}

/// A trait for reading data during persist serialization
pub trait PersistReader {
    /// Read the next chunk of serialized persist data into buf
    ///
    /// Returns the number of bytes written. If the return value is less than buf.len(), this
    /// indicates that the serialization is complete.
    fn read(&mut self, buf: &mut [u8]) -> usize;
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

impl<F: Future> PersistReader for PersistSerializer<'_, '_, F> {
    fn read(&mut self, buf: &mut [u8]) -> usize {
        let mut cx = Context::from_waker(noop_waker_ref());

        let mut pos = 0;
        loop {
            if pos >= buf.len() {
                return pos;
            }

            match self.f.as_mut().poll(&mut cx) {
                core::task::Poll::Ready(_) => return pos,
                core::task::Poll::Pending => {
                    buf[pos] = *self.reg.borrow();
                    pos += 1;
                }
            }
        }
    }
}

/// Serialize node data
pub fn serialize<F: FnMut(&mut dyn PersistReader)>(
    node_config: &NodeConfig,
    od: &'static [ODEntry],
    mut callback: F,
) {
    let reg = RefCell::new(0);
    let fut = pin!(serialize_sm(node_config, od, &reg));
    let mut serializer = PersistSerializer::new(fut, &reg);
    callback(&mut serializer)
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
    /// A node config object, holding basic node info like the node ID
    NodeConfig(NodeConfig),
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
            NodeType::NodeConfig => {
                if data.len() < 4 {
                    return Err(PersistReadError::NodeLengthShort);
                }
                Ok(Self::NodeConfig(NodeConfig {
                    node_id: data[1],
                    baud_table: data[2],
                    baud_index: data[3],
                }))
            }
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
            NodeType::Unknown => Ok(PersistNodeRef::Unknown(&data)),
        }
    }
}

/// Read serialized object data from a slice of bytes
///
/// PersistNodeReader provides an Iterator of PersistNodeRef objects, representing all of the nodes
/// stored in the slice
pub struct PersistNodeReader<'a> {
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
        if self.buf.len() - self.pos < 3 {
            return None;
        }
        let length = u16::from_le_bytes(self.buf[self.pos..self.pos + 2].try_into().unwrap());
        self.pos += 2;
        let node_slice = &self.buf[self.pos..self.pos + length as usize];
        self.pos += length as usize;

        PersistNodeRef::from_slice(node_slice).ok()
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

        let inst100 = Box::leak(Box::new(Object100::default()));
        let od = Box::leak(Box::new([ODEntry {
            index: 0x100,
            data: zencan_common::objects::ObjectData::Storage(inst100),
        }]));
        inst100.set_value1(42);
        let node_config = NodeConfig {
            node_id: 1,
            baud_table: 0,
            baud_index: 8,
        };

        let mut data = Vec::new();

        serialize(&node_config, od, |reader| {
            const CHUNK_SIZE: usize = 2;
            let mut buf = [0; CHUNK_SIZE];
            loop {
                let n = reader.read(&mut buf);
                data.extend_from_slice(&buf[..n]);
                if n < buf.len() {
                    break;
                }
            }
        });

        assert_eq!(6 + 10, data.len());

        let mut deser = PersistNodeReader::new(&data);
        assert_eq!(
            deser.next().unwrap(),
            PersistNodeRef::NodeConfig(node_config)
        );
        assert_eq!(
            deser.next().unwrap(),
            PersistNodeRef::ObjectValue(ObjectValue {
                index: 0x100,
                sub: 1,
                data: &42u32.to_le_bytes()
            })
        );
        assert_eq!(deser.next(), None);
    }
}
