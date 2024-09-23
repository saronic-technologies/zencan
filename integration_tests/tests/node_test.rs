use std::{sync::{atomic::AtomicBool, Arc}, time::Duration};

use canopen_client::sdo_client::SdoClient;
use canopen_common::{
    objects::ObjectDict,
    traits::{CanReceiver, CanSender},
};
use canopen_node::{build_object_dict, node::Node};
use integration_tests::sim_bus::{SimBus, SimCanReceiver, SimCanSender};


build_object_dict!(
    r#"[FileInfo]
FileName=sample.eds
FileVersion=1
FileRevision=1
LastEDS=
EDSVersion=4.0
Description=
CreationTime=1:58PM
CreationDate=09-10-2024
CreatedBy=
ModificationTime=1:58PM
ModificationDate=09-10-2024
ModifiedBy=

[DeviceInfo]
VendorName=Tester
VendorNumber=
ProductName=Test Product
ProductNumber=
RevisionNumber=0
BaudRate_10=0
BaudRate_20=0
BaudRate_50=0
BaudRate_125=0
BaudRate_250=0
BaudRate_500=0
BaudRate_800=0
BaudRate_1000=1
SimpleBootUpMaster=0
SimpleBootUpSlave=0
Granularity=8
DynamicChannelsSupported=0
CompactPDO=0
GroupMessaging=0
NrOfRXPDO=0
NrOfTXPDO=0
LSS_Supported=0
NG_Slave=0

[DummyUsage]
Dummy0001=0
Dummy0002=0
Dummy0003=0
Dummy0004=0
Dummy0005=0
Dummy0006=0
Dummy0007=0

[Comments]
Lines=0

[MandatoryObjects]
SupportedObjects=0

[OptionalObjects]
SupportedObjects=0

[ManufacturerObjects]
SupportedObjects=3
1=0x2000
2=0x2001
3=0x2002

[2000]
ParameterName=Array Example
ObjectType=0x8
;StorageLocation=RAM
SubNumber=0x3

[2000sub0]
ParameterName=Highest sub-index supported
ObjectType=0x7
;StorageLocation=RAM
DataType=0x0005
AccessType=ro
DefaultValue=0x02
PDOMapping=0

[2000sub1]
ParameterName=Sub Object 1
ObjectType=0x7
;StorageLocation=RAM
DataType=0x0004
AccessType=ro
DefaultValue=123
PDOMapping=0

[2000sub2]
ParameterName=Sub Object 1
ObjectType=0x7
;StorageLocation=RAM
DataType=0x0004
AccessType=ro
DefaultValue=-1
PDOMapping=0

[2001]
ParameterName=Record Example
ObjectType=0x9
;StorageLocation=RAM
SubNumber=0x4

[2001sub0]
ParameterName=Highest sub-index supported
ObjectType=0x7
;StorageLocation=RAM
DataType=0x0005
AccessType=ro
DefaultValue=0x04
PDOMapping=0

[2001sub1]
ParameterName=Sub Object 1
ObjectType=0x7
;StorageLocation=RAM
DataType=0x0007
AccessType=rw
DefaultValue=140
PDOMapping=0

[2001sub3]
ParameterName=Sub Object 1
ObjectType=0x7
;StorageLocation=RAM
DataType=0x0003
AccessType=ro
DefaultValue=0x10
PDOMapping=0

[2001sub4]
ParameterName=Sub Object 1
ObjectType=0x7
;StorageLocation=RAM
DataType=0x000F
AccessType=ro
DefaultValue=0x200
PDOMapping=0

[2002]
ParameterName=String Var
ObjectType=0x7
;StorageLocation=RAM
DataType=0x0009
AccessType=rw
DefaultValue=Some String
PDOMapping=0
"#
);

fn get_od() -> ObjectDict<'static> {
    ObjectDict { table: &OD_TABLE }
}

fn setup() -> (SdoClient<SimCanSender<'static>, SimCanReceiver>, SimBus<'static>) {
    const SLAVE_NODE_ID: u8 = 1;

    let od = get_od();
    let node = Node::new(SLAVE_NODE_ID, od);

    let mut bus = SimBus::new(vec![node]);

    let (sender, receiver) = bus.new_pair();
    let client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

    (client, bus)
}

#[test]
pub fn test_string_write() {
    let (mut client, mut bus) = setup();
    let mut sender = bus.new_sender();

    bus.nodes()[0].enter_preop(&mut |tx_msg| sender.send(tx_msg).unwrap());

    // Transfer a string short enough to be done expedited
    client.download(0x2002, 0, "Test".as_bytes()).unwrap();
    let readback = client.upload(0x2002, 0).unwrap();
    assert_eq!("Test".as_bytes(), readback);
    // Transfer a longer string which will do segmented transfer
    client.download(0x2002, 0, "Testers".as_bytes()).unwrap();
    let readback = client.upload(0x2002, 0).unwrap();
    assert_eq!("Testers".as_bytes(), readback);
    // Transfer an even longer string which will do segmented transfer with two segments
    client.download(0x2002, 0, "Testers123".as_bytes()).unwrap();
    let readback = client.upload(0x2002, 0).unwrap();
    assert_eq!("Testers123".as_bytes(), readback);
    // Transfer as max-length string (the default value in EDS is 11 characters long)
    client.download(0x2002, 0, "Testers1234".as_bytes()).unwrap();
    let readback = client.upload(0x2002, 0).unwrap();
    assert_eq!("Testers1234".as_bytes(), readback);
}

// #[test]
// pub fn test_record_access() {
//     let OBJECT_ID: u16 = 0x2001;

//     let (mut client, mut bus) = setup();
//     let mut sender = bus.new_sender();

//     bus.nodes()[0].enter_preop(&mut |tx_msg| sender.send(tx_msg).unwrap());

//     let size_data = client.upload(OBJECT_ID, 0).unwrap();
//     assert_eq!(1, size_data.len());
//     client.download(OBJECT_ID, , data)
// }
