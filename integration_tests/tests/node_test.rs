use integration_tests::{
    object_dict1,
    sim_bus::{SimBus, SimBusReceiver, SimBusSender},
};
use zencan_client::sdo_client::{SdoClient, SdoClientError};
use zencan_common::{objects::ODEntry, sdo::AbortCode, NodeId};
use zencan_node::{
    node::Node,
    node_mbox::{NodeMboxRead, NodeMboxWrite},
    node_state::NodeStateAccess,
};

mod utils;
use utils::{BusLogger, test_with_background_process};

fn setup<'a, M: NodeMboxWrite + NodeMboxRead, S: NodeStateAccess>(
    od: &'static [ODEntry],
    mbox: &'static M,
    state: &'static S,
) -> (
    Node<'static>,
    SdoClient<SimBusSender<'a>, SimBusReceiver>,
    SimBus<'a>,
) {
    const SLAVE_NODE_ID: u8 = 1;

    let node = Node::new(NodeId::new(SLAVE_NODE_ID).unwrap(), mbox, state, od);

    let mut bus = SimBus::new(vec![mbox]);

    let sender = bus.new_sender();
    let receiver = bus.new_receiver();
    let client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

    (node, client, bus)
}

#[tokio::test]
#[serial_test::serial]
async fn test_identity_readback() {
    const IDENTITY_OBJECT_ID: u16 = 0x1018;
    const VENDOR_SUB_ID: u8 = 1;
    const PRODUCT_SUB_ID: u8 = 2;
    const REVISION_SUB_ID: u8 = 3;
    const SERIAL_SUB_ID: u8 = 4;

    let (mut node, mut client, mut bus) = setup(
        &object_dict1::OD_TABLE,
        &object_dict1::NODE_MBOX,
        &object_dict1::NODE_STATE,
    );

    let _logger = BusLogger::new(bus.new_receiver());

    let test_task = async move {
        // Check that the identity matches the values defined in the example1.toml device config
        assert_eq!(
            client
                .read_u32(IDENTITY_OBJECT_ID, VENDOR_SUB_ID)
                .await
                .unwrap(),
            1234,
        );
        assert_eq!(
            client
                .read_u32(IDENTITY_OBJECT_ID, PRODUCT_SUB_ID)
                .await
                .unwrap(),
            12000,
        );
        assert_eq!(
            client
                .read_u32(IDENTITY_OBJECT_ID, REVISION_SUB_ID)
                .await
                .unwrap(),
            1,
        );
        assert_eq!(
            client
                .read_u32(IDENTITY_OBJECT_ID, SERIAL_SUB_ID)
                .await
                .unwrap(),
            0,
        );
    };

    let mut sender = bus.new_sender();
    test_with_background_process(&mut [&mut node], &mut sender, test_task).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_string_write() {
    let (mut node, mut client, mut bus) = setup(
        &object_dict1::OD_TABLE,
        &object_dict1::NODE_MBOX,
        &object_dict1::NODE_STATE,
    );
    let mut sender = bus.new_sender();
    let _logger = BusLogger::new(bus.new_receiver());

    let test_task = async move {
        // Transfer a string short enough to be done expedited
        client.download(0x2002, 0, "Test".as_bytes()).await.unwrap();
        let readback = client.upload(0x2002, 0).await.unwrap();
        assert_eq!("Test".as_bytes(), readback);
        // Transfer a longer string which will do segmented transfer
        client
            .download(0x2002, 0, "Testers".as_bytes())
            .await
            .unwrap();
        let readback = client.upload(0x2002, 0).await.unwrap();
        assert_eq!("Testers".as_bytes(), readback);
        // Transfer an even longer string which will do segmented transfer with two segments
        client
            .download(0x2002, 0, "Testers123".as_bytes())
            .await
            .unwrap();
        let readback = client.upload(0x2002, 0).await.unwrap();
        assert_eq!("Testers123".as_bytes(), readback);
        // Transfer as max-length string (the default value in EDS is 11 characters long)
        client
            .download(0x2002, 0, "Testers1234".as_bytes())
            .await
            .unwrap();
        let readback = client.upload(0x2002, 0).await.unwrap();
        assert_eq!("Testers1234".as_bytes(), readback);
    };

    test_with_background_process(&mut [&mut node], &mut sender, test_task).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_record_access() {
    const OBJECT_ID: u16 = 0x2001;

    let od = &object_dict1::OD_TABLE;
    let state = &object_dict1::NODE_STATE;
    let mbox = &object_dict1::NODE_MBOX;
    let (mut node, mut client, mut bus) = setup(od, mbox, state);

    // Create a logger to display messages on the bus on test failure for debugging
    let _logger = BusLogger::new(bus.new_receiver());

    let mut sender = bus.new_sender();

    let test_task = async move {
        let size_data = client.upload(OBJECT_ID, 0).await.unwrap();
        assert_eq!(1, size_data.len());
        assert_eq!(4, size_data[0]); // Highest sub index supported

        // Check default values of all sub indices
        let sub1_bytes = client.upload(OBJECT_ID, 1).await.unwrap();
        assert_eq!(4, sub1_bytes.len());
        assert_eq!(140, u32::from_le_bytes(sub1_bytes.try_into().unwrap()));
        let sub3_bytes = client.upload(OBJECT_ID, 3).await.unwrap();
        assert_eq!(2, sub3_bytes.len());
        assert_eq!(0x20, u16::from_le_bytes(sub3_bytes.try_into().unwrap()));

        // Write/readback sub1
        client
            .download(OBJECT_ID, 1, &4567u32.to_le_bytes())
            .await
            .unwrap();
        let sub1_bytes = client.upload(OBJECT_ID, 1).await.unwrap();
        assert_eq!(4567, u32::from_le_bytes(sub1_bytes.try_into().unwrap()));

        // Sub3 is read-only; writing should return an abort
        let res = client.download(OBJECT_ID, 3, &100u16.to_le_bytes()).await;
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            SdoClientError::ServerAbort {
                index: OBJECT_ID,
                sub: 3,
                abort_code: AbortCode::ReadOnly as u32
            }
        );
    };

    test_with_background_process(&mut [&mut node], &mut sender, test_task).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_array_access() {
    const OBJECT_ID: u16 = 0x2000;

    let (mut node, mut client, mut bus) = setup(
        &object_dict1::OD_TABLE,
        &object_dict1::NODE_MBOX,
        &object_dict1::NODE_STATE,
    );
    let mut sender = bus.new_sender();

    let _logger = BusLogger::new(bus.new_receiver());

    let test_task = async move {
        let size_data = client.upload(OBJECT_ID, 0).await.unwrap();
        assert_eq!(1, size_data.len());
        assert_eq!(2, size_data[0]); // Highest sub index supported

        // Read back default values
        let data = client.upload(OBJECT_ID, 1).await.unwrap();
        assert_eq!(4, data.len());
        assert_eq!(123, i32::from_le_bytes(data.try_into().unwrap()));

        let data = client.upload(OBJECT_ID, 2).await.unwrap();
        assert_eq!(4, data.len());
        assert_eq!(-1, i32::from_le_bytes(data.try_into().unwrap()));

        // Write and read
        client
            .download(OBJECT_ID, 1, &(-40i32).to_le_bytes())
            .await
            .unwrap();
        let data = client.upload(OBJECT_ID, 1).await.unwrap();
        assert_eq!(-40, i32::from_le_bytes(data.try_into().unwrap()));

        client
            .download(OBJECT_ID, 2, &(99i32).to_le_bytes())
            .await
            .unwrap();
        let data = client.upload(OBJECT_ID, 2).await.unwrap();
        assert_eq!(99, i32::from_le_bytes(data.try_into().unwrap()));
    };

    test_with_background_process(&mut [&mut node], &mut sender, test_task).await;
}
