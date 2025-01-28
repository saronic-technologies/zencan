//! A hand-coded example node instantiation
//!
//! Normally, this would be auto-generated from an EDS file using the zencan-build crate. But this
//! is here to provide an example of what the generated code looks like and test facility
//! 0

pub struct MutData {
    object1000_sub0: u8,
    object1000_sub1: u32,
    object1000_sub2: f32,
}

const ARRAY_SIZE_1001: usize = 2;
pub struct ConstData {
    object1001: [u32; ARRAY_SIZE_1001],
    object1001_sub0: u8,
}

static MUT_DATA: MutData = MutData {
    object1000_sub0: 2,
    object1000_sub1: 120,
    object1000_sub2: 3.14159,
};

const CONST_DATA: ConstData = ConstData {
    object1001: [10, 20],
    object1001_sub0: 1,
};

static OBJECT1000_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 2usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1000_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1000_sub2 as *const f32 as *const u8 },
            4usize,
        ),
    ))),
];

static OBJECT1000: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        storage: &OBJECT1000_STORAGE,
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::Real32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1000_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4, 4],
    });

static OBJECT1001: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Array(zencan_common::objects::Array {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Ro,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { CONST_DATA.object1001.as_ptr() as *const u8 },
                4usize,
            ),
        )),
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1001_sub0 as *const u8 },
                1,
            ),
        )),
        size: 4usize,
    });

pub static OD_TABLE: [zencan_common::objects::ODEntry; 2usize] = {
    [
        zencan_common::objects::ODEntry {
            index: 0x1000,
            data: &OBJECT1000,
        },
        zencan_common::objects::ODEntry {
            index: 0x1001,
            data: &OBJECT1001,
        },
    ]
};


fn main() {

}
