mod manual_node;
pub use manual_node::*;

#[cfg(test)]
mod tests {
    use crossbeam::atomic::AtomicCell;
    use zencan_common::{
        objects::{
            find_object, AccessType, Context, DataType, ODEntry, ObjectData, ObjectRawAccess,
            SubInfo,
        },
        sdo::AbortCode,
    };

    use super::*;

    /// The OD_TABLE must be `Sync` and `Send` because it is used in a
    /// multi-threaded context. This is a compile-time check to ensure that
    #[test]
    fn test_od_sync_send() {
        fn is_sync_send<T: Sync + Send>() {}

        is_sync_send::<ODEntry>();
    }

    #[test]
    fn test_callbacks() {
        fn write_handler(
            context: &Option<&dyn Context>,
            sub: u8,
            offset: usize,
            data: &[u8],
        ) -> Result<(), AbortCode> {
            let context = context
                .unwrap()
                .as_any()
                .downcast_ref::<AtomicCell<u32>>()
                .expect("Context should be of type AtomicCell<u32>");
            match sub {
                0 => Err(AbortCode::ReadOnly),
                1 => {
                    if offset != 0 {
                        return Err(AbortCode::DataTypeMismatch);
                    }
                    let value = u32::from_le_bytes(data.try_into().map_err(|_| {
                        if data.len() < std::mem::size_of::<u32>() {
                            AbortCode::DataTypeMismatchLengthLow
                        } else {
                            AbortCode::DataTypeMismatchLengthHigh
                        }
                    })?);
                    context.store(value);
                    Ok(())
                }
                _ => Err(AbortCode::NoSuchSubIndex),
            }
        }
        fn read_handler(
            context: &Option<&dyn Context>,
            _sub: u8,
            offset: usize,
            data: &mut [u8],
        ) -> Result<(), AbortCode> {
            // don't support partial access to scalar value
            if offset != 0 {
                return Err(AbortCode::DataTypeMismatch);
            }

            let context = context
                .unwrap()
                .as_any()
                .downcast_ref::<AtomicCell<u32>>()
                .expect("Context should be of type AtomicCell<u32>");

            if data.len() < size_of::<u32>() {
                return Err(AbortCode::DataTypeMismatchLengthLow);
            }
            if data.len() > size_of::<u32>() {
                return Err(AbortCode::DataTypeMismatchLengthHigh);
            }
            data.copy_from_slice(&context.load().to_le_bytes());
            Ok(())
        }

        fn info_handler(
            _context: &Option<&dyn Context>,
            sub: u8,
        ) -> Result<SubInfo, AbortCode> {
            match sub {
                0 => Ok(SubInfo {
                    size: 1,
                    data_type: DataType::UInt8,
                    access_type: AccessType::Const,
                }),
                1 => Ok(SubInfo {
                    data_type: DataType::UInt32,
                    size: 4,
                    access_type: AccessType::Rw,
                }),
                _ => Err(AbortCode::NoSuchSubIndex),
            }
        }

        let entry = find_object(&OD_TABLE, 4000).expect("Failed to find object 0x4000");
        if let ObjectData::Callback(callback) = entry {
            let context = Box::leak(Box::new(AtomicCell::new(0u32)));
            callback.register(
                Some(write_handler),
                Some(read_handler),
                Some(info_handler),
                Some(context),
            );

            callback.write(1, 0, &12u32.to_le_bytes()).unwrap();
            let mut readbuf = [0u8; 4];
            callback.read(1, 0, &mut readbuf).unwrap();
            assert_eq!(u32::from_le_bytes(readbuf), 12u32);
        } else {
            panic!("Expected a CallbackObject");
        }
    }

    #[test]
    fn test_direct_object_access() {
        OBJECT1000.set_sub1(400);
        let object = find_object(&OD_TABLE, 1000).expect("Failed to find object 0x1000");
        let mut readbuf = [0u8; 4];
        object.read(1, 0, &mut readbuf).unwrap();
        assert_eq!(400, u32::from_le_bytes(readbuf));
    }
}
