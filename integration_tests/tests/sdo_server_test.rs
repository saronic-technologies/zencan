use integration_tests::sim_bus::SimBus;
use zencan_client::SdoClient;
use zencan_common::NodeId;
use zencan_node::Node;

mod utils;
use utils::test_with_background_process;

#[tokio::test]
async fn test_sdo_read() {
    const SLAVE_NODE_ID: u8 = 1;

    let od = &integration_tests::object_dict1::OD_TABLE;
    let state = &integration_tests::object_dict1::NODE_STATE;
    let mbox = &integration_tests::object_dict1::NODE_MBOX;
    let mut node = Node::new(NodeId::new(SLAVE_NODE_ID).unwrap(), mbox, state, od);
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
