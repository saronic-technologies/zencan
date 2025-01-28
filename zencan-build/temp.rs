pub struct MutData {
    object1000: u32,
    #[doc = "Error register"]
    object1001: u8,
    object1018_sub1: u32,
    object1018_sub2: u32,
    object1018_sub3: u32,
    object1018_sub4: u32,
    object1003_sub0: u8,
    object1003: u32,
    object1005: u32,
    object1006: u32,
    object1007: u32,
    object1010_sub0: u8,
    object1010: u32,
    object1011_sub0: u8,
    object1011: u32,
    object1012: u32,
    object1014: u32,
    object1015: u16,
    object1016_sub0: u8,
    object1016: u32,
    object1017: u16,
    #[doc = "Synchronous counter overflow value"]
    object1019: u8,
    object1200_sub1: u32,
    object1200_sub2: u32,
    object1280_sub1: u32,
    object1280_sub2: u32,
    object1280_sub3: u8,
    object1400_sub1: u32,
    object1400_sub2: u8,
    object1401_sub1: u32,
    object1401_sub2: u8,
    object1402_sub1: u32,
    object1402_sub2: u8,
    object1403_sub1: u32,
    object1403_sub2: u8,
    object1600_sub1: u32,
    object1600_sub2: u32,
    object1600_sub3: u32,
    object1600_sub4: u32,
    object1600_sub5: u32,
    object1600_sub6: u32,
    object1600_sub7: u32,
    object1600_sub8: u32,
    object1601_sub1: u32,
    object1601_sub2: u32,
    object1601_sub3: u32,
    object1601_sub4: u32,
    object1601_sub5: u32,
    object1601_sub6: u32,
    object1601_sub7: u32,
    object1601_sub8: u32,
    object1602_sub1: u32,
    object1602_sub2: u32,
    object1602_sub3: u32,
    object1602_sub4: u32,
    object1602_sub5: u32,
    object1602_sub6: u32,
    object1602_sub7: u32,
    object1602_sub8: u32,
    object1603_sub1: u32,
    object1603_sub2: u32,
    object1603_sub3: u32,
    object1603_sub4: u32,
    object1603_sub5: u32,
    object1603_sub6: u32,
    object1603_sub7: u32,
    object1603_sub8: u32,
    object1800_sub1: u32,
    object1800_sub2: u8,
    object1800_sub3: u16,
    object1800_sub5: u16,
    object1801_sub1: u32,
    object1801_sub2: u8,
    object1801_sub3: u16,
    object1801_sub5: u16,
    object1802_sub1: u32,
    object1802_sub2: u8,
    object1802_sub3: u16,
    object1802_sub5: u16,
    object1803_sub1: u32,
    object1803_sub2: u8,
    object1803_sub3: u16,
    object1803_sub5: u16,
    object1a00_sub1: u32,
    object1a00_sub2: u32,
    object1a00_sub3: u32,
    object1a00_sub4: u32,
    object1a00_sub5: u32,
    object1a00_sub6: u32,
    object1a00_sub7: u32,
    object1a00_sub8: u32,
    object1a01_sub1: u32,
    object1a01_sub2: u32,
    object1a01_sub3: u32,
    object1a01_sub4: u32,
    object1a01_sub5: u32,
    object1a01_sub6: u32,
    object1a01_sub7: u32,
    object1a01_sub8: u32,
    object1a02_sub1: u32,
    object1a02_sub2: u32,
    object1a02_sub3: u32,
    object1a02_sub4: u32,
    object1a02_sub5: u32,
    object1a02_sub6: u32,
    object1a02_sub7: u32,
    object1a02_sub8: u32,
    object1a03_sub1: u32,
    object1a03_sub2: u32,
    object1a03_sub3: u32,
    object1a03_sub4: u32,
    object1a03_sub5: u32,
    object1a03_sub6: u32,
    object1a03_sub7: u32,
    object1a03_sub8: u32,
    object2000: u32,
}
pub struct ConstData {
    object1018_sub0: u8,
    object1200_sub0: u8,
    object1280_sub0: u8,
    object1400_sub0: u8,
    object1401_sub0: u8,
    object1402_sub0: u8,
    object1403_sub0: u8,
    object1600_sub0: u8,
    object1601_sub0: u8,
    object1602_sub0: u8,
    object1603_sub0: u8,
    object1800_sub0: u8,
    object1801_sub0: u8,
    object1802_sub0: u8,
    object1803_sub0: u8,
    object1a00_sub0: u8,
    object1a01_sub0: u8,
    object1a02_sub0: u8,
    object1a03_sub0: u8,
}
static mut MUT_DATA: MutData = MutData {
    object1000: 0u32,
    object1001: 0u8,
    object1018_sub1: 0u32,
    object1018_sub2: 0u32,
    object1018_sub3: 0u32,
    object1018_sub4: 0u32,
    object1003_sub0: 17u8,
    object1003: [
        0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
        0u32,
    ],
    object1005: 128u32,
    object1006: 0u32,
    object1007: 0u32,
    object1010_sub0: 4u8,
    object1010: [1u32, 1u32, 1u32, 1u32],
    object1011_sub0: 4u8,
    object1011: [1u32, 1u32, 1u32, 1u32],
    object1012: 256u32,
    object1014: 128u32,
    object1015: 0u16,
    object1016_sub0: 8u8,
    object1016: [0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32],
    object1017: 0u16,
    object1019: 0u8,
    object1200_sub1: 1536u32,
    object1200_sub2: 1408u32,
    object1280_sub1: 2147483648u32,
    object1280_sub2: 2147483648u32,
    object1280_sub3: 1u8,
    object1400_sub1: 2147484160u32,
    object1400_sub2: 254u8,
    object1401_sub1: 2147484416u32,
    object1401_sub2: 254u8,
    object1402_sub1: 2147484672u32,
    object1402_sub2: 254u8,
    object1403_sub1: 2147484928u32,
    object1403_sub2: 254u8,
    object1600_sub1: 0u32,
    object1600_sub2: 0u32,
    object1600_sub3: 0u32,
    object1600_sub4: 0u32,
    object1600_sub5: 0u32,
    object1600_sub6: 0u32,
    object1600_sub7: 0u32,
    object1600_sub8: 0u32,
    object1601_sub1: 0u32,
    object1601_sub2: 0u32,
    object1601_sub3: 0u32,
    object1601_sub4: 0u32,
    object1601_sub5: 0u32,
    object1601_sub6: 0u32,
    object1601_sub7: 0u32,
    object1601_sub8: 0u32,
    object1602_sub1: 0u32,
    object1602_sub2: 0u32,
    object1602_sub3: 0u32,
    object1602_sub4: 0u32,
    object1602_sub5: 0u32,
    object1602_sub6: 0u32,
    object1602_sub7: 0u32,
    object1602_sub8: 0u32,
    object1603_sub1: 0u32,
    object1603_sub2: 0u32,
    object1603_sub3: 0u32,
    object1603_sub4: 0u32,
    object1603_sub5: 0u32,
    object1603_sub6: 0u32,
    object1603_sub7: 0u32,
    object1603_sub8: 0u32,
    object1800_sub1: 3221225856u32,
    object1800_sub2: 254u8,
    object1800_sub3: 0u16,
    object1800_sub5: 0u16,
    object1801_sub1: 3221226112u32,
    object1801_sub2: 254u8,
    object1801_sub3: 0u16,
    object1801_sub5: 0u16,
    object1802_sub1: 3221226368u32,
    object1802_sub2: 254u8,
    object1802_sub3: 0u16,
    object1802_sub5: 0u16,
    object1803_sub1: 3221226624u32,
    object1803_sub2: 254u8,
    object1803_sub3: 0u16,
    object1803_sub5: 0u16,
    object1a00_sub1: 0u32,
    object1a00_sub2: 0u32,
    object1a00_sub3: 0u32,
    object1a00_sub4: 0u32,
    object1a00_sub5: 0u32,
    object1a00_sub6: 0u32,
    object1a00_sub7: 0u32,
    object1a00_sub8: 0u32,
    object1a01_sub1: 0u32,
    object1a01_sub2: 0u32,
    object1a01_sub3: 0u32,
    object1a01_sub4: 0u32,
    object1a01_sub5: 0u32,
    object1a01_sub6: 0u32,
    object1a01_sub7: 0u32,
    object1a01_sub8: 0u32,
    object1a02_sub1: 0u32,
    object1a02_sub2: 0u32,
    object1a02_sub3: 0u32,
    object1a02_sub4: 0u32,
    object1a02_sub5: 0u32,
    object1a02_sub6: 0u32,
    object1a02_sub7: 0u32,
    object1a02_sub8: 0u32,
    object1a03_sub1: 0u32,
    object1a03_sub2: 0u32,
    object1a03_sub3: 0u32,
    object1a03_sub4: 0u32,
    object1a03_sub5: 0u32,
    object1a03_sub6: 0u32,
    object1a03_sub7: 0u32,
    object1a03_sub8: 0u32,
    object2000: 0u32,
};
const CONST_DATA: ConstData = ConstData {
    object1018_sub0: 4u8,
    object1200_sub0: 2u8,
    object1280_sub0: 3u8,
    object1400_sub0: 5u8,
    object1401_sub0: 5u8,
    object1402_sub0: 5u8,
    object1403_sub0: 5u8,
    object1600_sub0: 0u8,
    object1601_sub0: 0u8,
    object1602_sub0: 0u8,
    object1603_sub0: 0u8,
    object1800_sub0: 6u8,
    object1801_sub0: 6u8,
    object1802_sub0: 6u8,
    object1803_sub0: 6u8,
    object1a00_sub0: 0u8,
    object1a01_sub0: 0u8,
    object1a02_sub0: 0u8,
    object1a03_sub0: 0u8,
};
static OBJECT1000: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Ro,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1000 as *const u32 as *const u8 },
                4usize,
            ),
        )),
        size: 4usize,
    });
static OBJECT1001: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt8,
        access_type: zencan_common::objects::AccessType::Ro,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1001 as *const u8 },
                1usize,
            ),
        )),
        size: 1usize,
    });
static OBJECT1018_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 4usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1018_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1018_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1018_sub3 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1018_sub4 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1018: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Ro),
            Some(zencan_common::objects::AccessType::Ro),
            Some(zencan_common::objects::AccessType::Ro),
            Some(zencan_common::objects::AccessType::Ro),
        ],
        storage: &OBJECT1018_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1018_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 4usize, 4usize, 4usize],
    });
static OBJECT1003: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Array(zencan_common::objects::Array {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Ro,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { MUT_DATA.object1003.as_ptr() as *const u8 },
                4usize,
            ),
        )),
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1003_sub0 as *const u8 },
                1,
            ),
        )),
        size: 4usize,
    });
static OBJECT1005: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1005 as *const u32 as *const u8 },
                4usize,
            ),
        )),
        size: 4usize,
    });
static OBJECT1006: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1006 as *const u32 as *const u8 },
                4usize,
            ),
        )),
        size: 4usize,
    });
static OBJECT1007: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1007 as *const u32 as *const u8 },
                4usize,
            ),
        )),
        size: 4usize,
    });
static OBJECT1010: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Array(zencan_common::objects::Array {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { MUT_DATA.object1010.as_ptr() as *const u8 },
                4usize,
            ),
        )),
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1010_sub0 as *const u8 },
                1,
            ),
        )),
        size: 4usize,
    });
static OBJECT1011: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Array(zencan_common::objects::Array {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { MUT_DATA.object1011.as_ptr() as *const u8 },
                4usize,
            ),
        )),
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1011_sub0 as *const u8 },
                1,
            ),
        )),
        size: 4usize,
    });
static OBJECT1012: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1012 as *const u32 as *const u8 },
                4usize,
            ),
        )),
        size: 4usize,
    });
static OBJECT1014: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1014 as *const u32 as *const u8 },
                4usize,
            ),
        )),
        size: 4usize,
    });
static OBJECT1015: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt16,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1015 as *const u16 as *const u8 },
                2usize,
            ),
        )),
        size: 2usize,
    });
static OBJECT1016: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Array(zencan_common::objects::Array {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { MUT_DATA.object1016.as_ptr() as *const u8 },
                4usize,
            ),
        )),
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1016_sub0 as *const u8 },
                1,
            ),
        )),
        size: 4usize,
    });
static OBJECT1017: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt16,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1017 as *const u16 as *const u8 },
                2usize,
            ),
        )),
        size: 2usize,
    });
static OBJECT1019: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt8,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object1019 as *const u8 },
                1usize,
            ),
        )),
        size: 1usize,
    });
static OBJECT1200_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 2usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1200_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1200_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1200: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Ro),
            Some(zencan_common::objects::AccessType::Ro),
        ],
        storage: &OBJECT1200_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1200_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 4usize],
    });
static OBJECT1280_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 3usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1280_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1280_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1280_sub3 as *const u8 as *const u8 },
            1usize,
        ),
    ))),
];
static OBJECT1280: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt8),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1280_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1280_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 4usize, 1usize],
    });
static OBJECT1400_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 3usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1400_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1400_sub2 as *const u8 as *const u8 },
            1usize,
        ),
    ))),
    None,
];
static OBJECT1400: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt8),
            None,
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            None,
        ],
        storage: &OBJECT1400_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1400_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 1usize, 0usize],
    });
static OBJECT1401_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 3usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1401_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1401_sub2 as *const u8 as *const u8 },
            1usize,
        ),
    ))),
    None,
];
static OBJECT1401: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt8),
            None,
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            None,
        ],
        storage: &OBJECT1401_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1401_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 1usize, 0usize],
    });
static OBJECT1402_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 3usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1402_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1402_sub2 as *const u8 as *const u8 },
            1usize,
        ),
    ))),
    None,
];
static OBJECT1402: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt8),
            None,
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            None,
        ],
        storage: &OBJECT1402_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1402_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 1usize, 0usize],
    });
static OBJECT1403_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 3usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1403_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1403_sub2 as *const u8 as *const u8 },
            1usize,
        ),
    ))),
    None,
];
static OBJECT1403: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt8),
            None,
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            None,
        ],
        storage: &OBJECT1403_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1403_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 1usize, 0usize],
    });
static OBJECT1600_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 8usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1600_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1600_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1600_sub3 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1600_sub4 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1600_sub5 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1600_sub6 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1600_sub7 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1600_sub8 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1600: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1600_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1600_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[
            4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize,
        ],
    });
static OBJECT1601_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 8usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1601_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1601_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1601_sub3 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1601_sub4 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1601_sub5 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1601_sub6 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1601_sub7 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1601_sub8 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1601: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1601_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1601_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[
            4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize,
        ],
    });
static OBJECT1602_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 8usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1602_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1602_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1602_sub3 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1602_sub4 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1602_sub5 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1602_sub6 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1602_sub7 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1602_sub8 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1602: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1602_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1602_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[
            4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize,
        ],
    });
static OBJECT1603_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 8usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1603_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1603_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1603_sub3 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1603_sub4 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1603_sub5 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1603_sub6 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1603_sub7 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1603_sub8 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1603: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1603_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1603_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[
            4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize,
        ],
    });
static OBJECT1800_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 5usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1800_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1800_sub2 as *const u8 as *const u8 },
            1usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1800_sub3 as *const u16 as *const u8 },
            2usize,
        ),
    ))),
    None,
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1800_sub5 as *const u16 as *const u8 },
            2usize,
        ),
    ))),
];
static OBJECT1800: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt8),
            Some(zencan_common::objects::DataType::UInt16),
            None,
            Some(zencan_common::objects::DataType::UInt16),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            None,
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1800_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1800_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 1usize, 2usize, 0usize, 2usize],
    });
static OBJECT1801_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 5usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1801_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1801_sub2 as *const u8 as *const u8 },
            1usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1801_sub3 as *const u16 as *const u8 },
            2usize,
        ),
    ))),
    None,
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1801_sub5 as *const u16 as *const u8 },
            2usize,
        ),
    ))),
];
static OBJECT1801: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt8),
            Some(zencan_common::objects::DataType::UInt16),
            None,
            Some(zencan_common::objects::DataType::UInt16),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            None,
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1801_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1801_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 1usize, 2usize, 0usize, 2usize],
    });
static OBJECT1802_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 5usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1802_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1802_sub2 as *const u8 as *const u8 },
            1usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1802_sub3 as *const u16 as *const u8 },
            2usize,
        ),
    ))),
    None,
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1802_sub5 as *const u16 as *const u8 },
            2usize,
        ),
    ))),
];
static OBJECT1802: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt8),
            Some(zencan_common::objects::DataType::UInt16),
            None,
            Some(zencan_common::objects::DataType::UInt16),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            None,
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1802_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1802_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 1usize, 2usize, 0usize, 2usize],
    });
static OBJECT1803_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 5usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1803_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1803_sub2 as *const u8 as *const u8 },
            1usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1803_sub3 as *const u16 as *const u8 },
            2usize,
        ),
    ))),
    None,
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1803_sub5 as *const u16 as *const u8 },
            2usize,
        ),
    ))),
];
static OBJECT1803: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt8),
            Some(zencan_common::objects::DataType::UInt16),
            None,
            Some(zencan_common::objects::DataType::UInt16),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            None,
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1803_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1803_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[4usize, 1usize, 2usize, 0usize, 2usize],
    });
static OBJECT1a00_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 8usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a00_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a00_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a00_sub3 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a00_sub4 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a00_sub5 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a00_sub6 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a00_sub7 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a00_sub8 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1A00: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1a00_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1a00_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[
            4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize,
        ],
    });
static OBJECT1a01_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 8usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a01_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a01_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a01_sub3 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a01_sub4 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a01_sub5 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a01_sub6 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a01_sub7 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a01_sub8 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1A01: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1a01_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1a01_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[
            4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize,
        ],
    });
static OBJECT1a02_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 8usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a02_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a02_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a02_sub3 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a02_sub4 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a02_sub5 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a02_sub6 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a02_sub7 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a02_sub8 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1A02: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1a02_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1a02_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[
            4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize,
        ],
    });
static OBJECT1a03_STORAGE: [Option<
    critical_section::Mutex<core::cell::RefCell<zencan_common::objects::ObjectStorage>>,
>; 8usize] = [
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a03_sub1 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a03_sub2 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a03_sub3 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a03_sub4 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a03_sub5 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a03_sub6 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a03_sub7 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
    Some(critical_section::Mutex::new(core::cell::RefCell::new(
        zencan_common::objects::ObjectStorage::Ram(
            unsafe { &MUT_DATA.object1a03_sub8 as *const u32 as *const u8 },
            4usize,
        ),
    ))),
];
static OBJECT1A03: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Record(zencan_common::objects::Record {
        data_types: &[
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
            Some(zencan_common::objects::DataType::UInt32),
        ],
        access_types: &[
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
            Some(zencan_common::objects::AccessType::Rw),
        ],
        storage: &OBJECT1a03_STORAGE,
        storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &CONST_DATA.object1a03_sub0 as *const u8 },
                1,
            ),
        )),
        sizes: &[
            4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize, 4usize,
        ],
    });
static OBJECT2000: zencan_common::objects::ObjectData =
    zencan_common::objects::ObjectData::Var(zencan_common::objects::Var {
        data_type: zencan_common::objects::DataType::UInt32,
        access_type: zencan_common::objects::AccessType::Rw,
        storage: critical_section::Mutex::new(core::cell::RefCell::new(
            zencan_common::objects::ObjectStorage::Ram(
                unsafe { &MUT_DATA.object2000 as *const u32 as *const u8 },
                4usize,
            ),
        )),
        size: 4usize,
    });
pub static OD_TABLE: [zencan_common::objects::ODEntry; 34usize] = {
    [
        zencan_common::objects::ODEntry {
            index: 4096u16,
            data: &OBJECT1000,
        },
        zencan_common::objects::ODEntry {
            index: 4097u16,
            data: &OBJECT1001,
        },
        zencan_common::objects::ODEntry {
            index: 4120u16,
            data: &OBJECT1018,
        },
        zencan_common::objects::ODEntry {
            index: 4099u16,
            data: &OBJECT1003,
        },
        zencan_common::objects::ODEntry {
            index: 4101u16,
            data: &OBJECT1005,
        },
        zencan_common::objects::ODEntry {
            index: 4102u16,
            data: &OBJECT1006,
        },
        zencan_common::objects::ODEntry {
            index: 4103u16,
            data: &OBJECT1007,
        },
        zencan_common::objects::ODEntry {
            index: 4112u16,
            data: &OBJECT1010,
        },
        zencan_common::objects::ODEntry {
            index: 4113u16,
            data: &OBJECT1011,
        },
        zencan_common::objects::ODEntry {
            index: 4114u16,
            data: &OBJECT1012,
        },
        zencan_common::objects::ODEntry {
            index: 4116u16,
            data: &OBJECT1014,
        },
        zencan_common::objects::ODEntry {
            index: 4117u16,
            data: &OBJECT1015,
        },
        zencan_common::objects::ODEntry {
            index: 4118u16,
            data: &OBJECT1016,
        },
        zencan_common::objects::ODEntry {
            index: 4119u16,
            data: &OBJECT1017,
        },
        zencan_common::objects::ODEntry {
            index: 4121u16,
            data: &OBJECT1019,
        },
        zencan_common::objects::ODEntry {
            index: 4608u16,
            data: &OBJECT1200,
        },
        zencan_common::objects::ODEntry {
            index: 4736u16,
            data: &OBJECT1280,
        },
        zencan_common::objects::ODEntry {
            index: 5120u16,
            data: &OBJECT1400,
        },
        zencan_common::objects::ODEntry {
            index: 5121u16,
            data: &OBJECT1401,
        },
        zencan_common::objects::ODEntry {
            index: 5122u16,
            data: &OBJECT1402,
        },
        zencan_common::objects::ODEntry {
            index: 5123u16,
            data: &OBJECT1403,
        },
        zencan_common::objects::ODEntry {
            index: 5632u16,
            data: &OBJECT1600,
        },
        zencan_common::objects::ODEntry {
            index: 5633u16,
            data: &OBJECT1601,
        },
        zencan_common::objects::ODEntry {
            index: 5634u16,
            data: &OBJECT1602,
        },
        zencan_common::objects::ODEntry {
            index: 5635u16,
            data: &OBJECT1603,
        },
        zencan_common::objects::ODEntry {
            index: 6144u16,
            data: &OBJECT1800,
        },
        zencan_common::objects::ODEntry {
            index: 6145u16,
            data: &OBJECT1801,
        },
        zencan_common::objects::ODEntry {
            index: 6146u16,
            data: &OBJECT1802,
        },
        zencan_common::objects::ODEntry {
            index: 6147u16,
            data: &OBJECT1803,
        },
        zencan_common::objects::ODEntry {
            index: 6656u16,
            data: &OBJECT1a00,
        },
        zencan_common::objects::ODEntry {
            index: 6657u16,
            data: &OBJECT1a01,
        },
        zencan_common::objects::ODEntry {
            index: 6658u16,
            data: &OBJECT1a02,
        },
        zencan_common::objects::ODEntry {
            index: 6659u16,
            data: &OBJECT1a03,
        },
        zencan_common::objects::ODEntry {
            index: 8192u16,
            data: &OBJECT2000,
        },
    ]
};
