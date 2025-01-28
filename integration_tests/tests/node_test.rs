

use zencan_client::sdo_client::{SdoClient, SdoClientError};
use zencan_common::{
    objects::ObjectDict,
    sdo::AbortCode,
    traits::CanSender,
};
use zencan_node::node::Node;
use integration_tests::{object_dict1::get_od, sim_bus::{SimBus, SimCanReceiver, SimCanSender}};


fn setup<'a, const N: usize>(od: ObjectDict<'static, 'a, N>) -> (
    SdoClient<SimCanSender<'static, 'a, N>, SimCanReceiver>,
    SimBus<'static, 'a, N>,
) {
    const SLAVE_NODE_ID: u8 = 1;

    let node = Node::new(SLAVE_NODE_ID, od);

    let mut bus = SimBus::new(vec![node]);

    let (sender, receiver) = bus.new_pair();
    let client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

    (client, bus)
}

#[test]
pub fn test_string_write() {
    let (mut client, mut bus) = setup(get_od());
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
    client
        .download(0x2002, 0, "Testers1234".as_bytes())
        .unwrap();
    let readback = client.upload(0x2002, 0).unwrap();
    assert_eq!("Testers1234".as_bytes(), readback);
}

#[test]
pub fn test_record_access() {
    const OBJECT_ID: u16 = 0x2001;

    let (mut client, mut bus) = setup(get_od());
    let mut sender = bus.new_sender();

    bus.nodes()[0].enter_preop(&mut |tx_msg| sender.send(tx_msg).unwrap());

    let size_data = client.upload(OBJECT_ID, 0).unwrap();
    assert_eq!(1, size_data.len());
    assert_eq!(4, size_data[0]); // Highest sub index supported

    // Check default values of all sub indices
    let sub1_bytes = client.upload(OBJECT_ID, 1).unwrap();
    assert_eq!(4, sub1_bytes.len());
    assert_eq!(140, u32::from_le_bytes(sub1_bytes.try_into().unwrap()));
    let sub3_bytes = client.upload(OBJECT_ID, 3).unwrap();
    assert_eq!(2, sub3_bytes.len());
    assert_eq!(0x20, u16::from_le_bytes(sub3_bytes.try_into().unwrap()));

    // Write/readback sub1
    client
        .download(OBJECT_ID, 1, &4567u32.to_le_bytes())
        .unwrap();
    let sub1_bytes = client.upload(OBJECT_ID, 1).unwrap();
    assert_eq!(4567, u32::from_le_bytes(sub1_bytes.try_into().unwrap()));

    // Sub3 is read-only; writing should return an abort
    let res = client.download(OBJECT_ID, 3, &100u16.to_le_bytes());
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        SdoClientError::ServerAbort {
            abort_code: AbortCode::ReadOnly as u32
        }
    );
}

#[test]
pub fn test_array_access() {
    const OBJECT_ID: u16 = 0x2000;

    let (mut client, mut bus) = setup(get_od());
    let mut sender = bus.new_sender();

    bus.nodes()[0].enter_preop(&mut |tx_msg| sender.send(tx_msg).unwrap());

    let size_data = client.upload(OBJECT_ID, 0).unwrap();
    assert_eq!(1, size_data.len());
    assert_eq!(2, size_data[0]); // Highest sub index supported

    // Read back default values
    let data = client.upload(OBJECT_ID, 1).unwrap();
    assert_eq!(4, data.len());
    assert_eq!(123, i32::from_le_bytes(data.try_into().unwrap()));

    let data = client.upload(OBJECT_ID, 2).unwrap();
    assert_eq!(4, data.len());
    assert_eq!(-1, i32::from_le_bytes(data.try_into().unwrap()));

    // Write and read
    client.download(OBJECT_ID, 1, &(-40i32).to_le_bytes()).unwrap();
    let data = client.upload(OBJECT_ID, 1).unwrap();
    assert_eq!(-40, i32::from_le_bytes(data.try_into().unwrap()));

    client.download(OBJECT_ID, 2, &(99i32).to_le_bytes()).unwrap();
    let data = client.upload(OBJECT_ID, 2).unwrap();
    assert_eq!(99, i32::from_le_bytes(data.try_into().unwrap()));

}