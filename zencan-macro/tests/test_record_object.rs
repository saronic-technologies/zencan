use zencan_macro::record_object;
use zencan_node::common::{
    objects::{AccessType, DataType, ObjectRawAccess, PdoMapping, SubInfo},
    sdo::AbortCode,
};

#[derive(Default)]
#[record_object]
struct TestObject {
    #[record(pdo = "both")]
    val1: u32,
    val2: u16,
    val3: u8,
    val4: i32,
    val5: i16,
    val6: i8,
    val7: f32,
    #[record(persist)]
    val8: [u8; 15],
}

#[test]
fn test_record_object_sub_info() {
    let obj = TestObject::default();

    assert_eq!(
        obj.sub_info(0).unwrap(),
        SubInfo {
            size: 1,
            data_type: DataType::UInt8,
            access_type: AccessType::Const,
            pdo_mapping: PdoMapping::None,
            persist: false
        }
    );

    assert_eq!(
        obj.sub_info(1).unwrap(),
        SubInfo {
            size: 4,
            data_type: DataType::UInt32,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::Both,
            persist: false
        }
    );

    assert_eq!(
        obj.sub_info(2).unwrap(),
        SubInfo {
            size: 2,
            data_type: DataType::UInt16,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: false
        }
    );

    assert_eq!(
        obj.sub_info(3).unwrap(),
        SubInfo {
            size: 1,
            data_type: DataType::UInt8,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: false
        }
    );
    assert_eq!(
        obj.sub_info(4).unwrap(),
        SubInfo {
            size: 4,
            data_type: DataType::Int32,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: false
        }
    );
    assert_eq!(
        obj.sub_info(5).unwrap(),
        SubInfo {
            size: 2,
            data_type: DataType::Int16,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: false
        }
    );
    assert_eq!(
        obj.sub_info(6).unwrap(),
        SubInfo {
            size: 1,
            data_type: DataType::Int8,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: false
        }
    );
    assert_eq!(
        obj.sub_info(7).unwrap(),
        SubInfo {
            size: 4,
            data_type: DataType::Real32,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: false
        }
    );
    assert_eq!(
        obj.sub_info(8).unwrap(),
        SubInfo {
            size: 15,
            data_type: DataType::VisibleString,
            access_type: AccessType::Rw,
            pdo_mapping: PdoMapping::None,
            persist: true
        }
    );
    assert_eq!(obj.sub_info(9), Err(AbortCode::NoSuchSubIndex));
}

#[test]
fn test_record_object_access() {
    let obj = TestObject::default();

    // Sub0 should hold the highest sub index
    let mut buf = [0];
    obj.read(0, 0, &mut buf).unwrap();
    assert_eq!(buf[0], 8);
    // Sub0 should be read-only
    assert_eq!(obj.write(0, 0, &buf), Err(AbortCode::ReadOnly));

    obj.write(1, 0, &42u32.to_le_bytes())
        .expect("Failed writing");
    assert_eq!(obj.get_val1(), 42);
    let mut buf = [0; 4];
    obj.read(1, 0, &mut buf).unwrap();
    assert_eq!(u32::from_le_bytes(buf.try_into().unwrap()), 42);

    obj.write(7, 0, &42.0f32.to_le_bytes()).unwrap();
    assert_eq!(obj.get_val7(), 42.0);
    let mut buf = [0; 4];
    obj.read(7, 0, &mut buf).unwrap();
    assert_eq!(f32::from_le_bytes(buf.try_into().unwrap()), 42.0);

    let mut val8 = [0; 15];
    let s = "Hello World";
    val8[..s.len()].copy_from_slice(s.as_bytes());
    obj.write(8, 0, s.as_bytes()).unwrap();
    assert_eq!(obj.current_size(8).unwrap(), s.len());
    let val8 = obj.get_val8();
    assert_eq!(&val8[..s.len()], s.as_bytes());
}
