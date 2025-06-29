use integration_tests::sim_bus::SimBus;
use zencan_client::SdoClient;
use zencan_common::NodeId;
use zencan_node::Node;

mod utils;
use utils::test_with_background_process;

#[tokio::test]
#[serial_test::serial]
async fn test_sdo_read() {
    const SLAVE_NODE_ID: u8 = 1;

    let od = &integration_tests::object_dict1::OD_TABLE;
    let state = &integration_tests::object_dict1::NODE_STATE;
    let mbox = &integration_tests::object_dict1::NODE_MBOX;
    let mut node = Node::init(NodeId::new(SLAVE_NODE_ID).unwrap(), mbox, state, od).finalize();
    let mut bus = SimBus::new(vec![mbox]);
    let mut sender = bus.new_sender();

    test_with_background_process(&mut [&mut node], &mut sender, async move {
        let sender = bus.new_sender();
        let receiver = bus.new_receiver();
        let mut client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

        client
            .download(0x3000, 0, &[0xa, 0xb, 0xc, 0xd])
            .await
            .unwrap();
        let read = client.upload(0x3000, 0).await.unwrap();

        assert_eq!(vec![0xa, 0xb, 0xc, 0xd], read);
    })
    .await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_block_download() {
    const SLAVE_NODE_ID: u8 = 1;

    let od = &integration_tests::object_dict1::OD_TABLE;
    let state = &integration_tests::object_dict1::NODE_STATE;
    let mbox = &integration_tests::object_dict1::NODE_MBOX;
    let mut node = Node::init(NodeId::new(SLAVE_NODE_ID).unwrap(), mbox, state, od).finalize();
    let mut bus = SimBus::new(vec![mbox]);
    let mut sender = bus.new_sender();

    test_with_background_process(&mut [&mut node], &mut sender, async move {
        let sender = bus.new_sender();
        let receiver = bus.new_receiver();
        let mut client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

        let data = Vec::from_iter(0..128);
        client.block_download(0x3006, 0, &data).await.unwrap();

        assert_eq!(
            data,
            integration_tests::object_dict1::OBJECT3006.get_value()[0..data.len()]
        );

        // Now do a long one which will require multiple blocks
        let data = Vec::from_iter((0..1200).map(|i| i as u8));
        client.block_download(0x3006, 0, &data).await.unwrap();

        assert_eq!(
            data,
            integration_tests::object_dict1::OBJECT3006.get_value()
        );
    })
    .await;
}
