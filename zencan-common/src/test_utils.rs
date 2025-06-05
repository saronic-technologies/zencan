//! Test utils
//!
//! Only available with `#[cfg(test)]`
use crate::{objects::ObjectRawAccess, sdo::AbortCode};

/// Utility to test some basic properties of structs implementing the [`ObjectRawAccess`] trait
pub fn test_raw_object_access(obj: &dyn ObjectRawAccess, sub: u8, size: u16) {
    // Can read sub info
    let sub_info = obj.sub_info(sub).expect("Missing sub info for sub {sub}");
    // Has expected size in sub info
    assert_eq!(size as usize, sub_info.size);

    let mut write_data = vec![0; size as usize];
    for i in 0..size {
        write_data[i as usize] = i as u8;
    }

    let write_result = obj.write(sub, 0, &write_data);
    if sub_info.access_type.is_writable() {
        assert_eq!(Ok(()), write_result);
    } else {
        assert_eq!(Err(AbortCode::ReadOnly), write_result);
    }

    // Read the full object
    if sub_info.access_type.is_readable() {
        let mut read_buf = vec![0; size as usize];
        obj.read(sub, 0, &mut read_buf)
            .expect("Failed to read the full object");
        if sub_info.access_type.is_writable() {
            assert_eq!(read_buf, write_data);
        }

        // Partial read with offset
        obj.read(sub, 1, &mut read_buf[1..])
            .expect("Failed to read with offset");
        // Partial read with short length
        obj.read(sub, 0, &mut read_buf[1..])
            .expect("Failed to read with short length");
        if size > 1 {
            // partial read with offset and short size
            obj.read(sub, 1, &mut read_buf[2..])
                .expect("Failed to read with offset and short size");
        }
    }
}
