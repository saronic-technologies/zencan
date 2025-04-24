//! Test node PDO operations
//!

use std::time::Duration;

use zencan_client::sdo_client::SdoClient;
use zencan_common::{
    messages::SyncObject, objects::ODEntry, traits::{AsyncCanReceiver, AsyncCanSender, CanFdMessage, CanId}
};
use zencan_node::{node::Node, node_state::NodeStateAccess};
use zencan_node::node_mbox::{NodeMboxRead, NodeMboxWrite};
use integration_tests::{object_dict1, sim_bus::{SimBus, SimBusReceiver, SimBusSender}};
use futures::executor::block_on;

mod bus_logger;
use bus_logger::BusLogger;


fn setup<'a, M: NodeMboxWrite + NodeMboxRead, S: NodeStateAccess>(od: &'static [ODEntry], mbox: &'static M, state: &'static S) -> (
    Node<'static>,
    SdoClient<SimBusSender<'a>, SimBusReceiver>,
    SimBus<'a>,
) {
    const SLAVE_NODE_ID: u8 = 1;

    let mut node = Node::new(mbox, state, od);
    node.set_node_id(SLAVE_NODE_ID);

    let mut bus = SimBus::new(vec![mbox]);

    let sender = bus.new_sender();
    let receiver = bus.new_receiver();
    let client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

    (node, client, bus)
}


#[tokio::test]
#[serial_test::serial]
async fn test_rpdo_assignment() {
    let od = &object_dict1::OD_TABLE;
    let state = &object_dict1::NODE_STATE;
    let mbox = &object_dict1::NODE_MBOX;

    let (mut node, mut client, mut bus) = setup(od, mbox, state);
    let mut sender = bus.new_sender();
    let rx = bus.new_receiver();

    let _bus_logger = BusLogger::new(rx);
    node.enter_preop(&mut |tx_msg| block_on(sender.send(tx_msg)).unwrap());

    let node_process_task = async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            node.process(&mut |tx_msg| block_on(sender.send(tx_msg)).unwrap());
        }
    };

    let mut sender = bus.new_sender();

    let test_task = async move {
        // Readback the largest sub index
        assert_eq!(2, client.upload_u8(0x1400, 0).await.unwrap());

        // TODO: Check default COMM values

        // Set COB-ID and readback
        // Valid bit set, and ID == 0x201.
        let cob_id_word: u32 = (1<<31) | 0x201;
        client.download_u32(0x1400, 1, cob_id_word).await.unwrap();

        let readback_cob_id_word = client.upload_u32(0x1400, 1).await.unwrap();
        assert_eq!(cob_id_word, readback_cob_id_word);

        // Set RPDO1 to map to object 0x2000, subindex 1, length 32 bits
        let mapping_entry: u32 = (0x2000 << 16) | (1 << 8) | 32;
        client.download_u32(0x1600, 1, mapping_entry).await.unwrap();

        // Now send a PDO message and it should update the mapped object
        sender.send(CanFdMessage::new(CanId::Std(0x201), &500u32.to_le_bytes())).await.unwrap();

        // Delay a bit, because node process() method has to be called for PDO to apply
        tokio::time::sleep(Duration::from_millis(10)).await;
        // Readback the mapped object; the PDO message above should have updated it
        assert_eq!(500, client.upload_u32(0x2000, 1).await.unwrap());
    };


    let _ = tokio::select! {
        _ = node_process_task => {}
        _ = test_task => {}
    };
}

#[tokio::test]
#[serial_test::serial]
async fn test_tpdo_asignment() {
    let od = &integration_tests::object_dict2::OD_TABLE;
    let state = &integration_tests::object_dict2::NODE_STATE;
    let mbox = &integration_tests::object_dict2::NODE_MBOX;
    let (mut node, mut client, mut bus) = setup(od, mbox, state);

    let mut sender = bus.new_sender();
    let mut rx = bus.new_receiver();
    node.enter_preop(&mut |tx_msg| block_on(sender.send(tx_msg)).unwrap());

    // Set COB-ID
    const TPDO_COMM1_ID: u16 = 0x1800;
    const PDO_COMM_COB_SUBID: u8 = 1;
    const PDO_COMM_TRANSMISSION_TYPE_SUBID: u8 = 2;

    // Set the TPDO COB ID
    client
        .download(TPDO_COMM1_ID, PDO_COMM_COB_SUBID, &0x181u32.to_le_bytes()).await
        .unwrap();
    // Set to sync driven
    client
        .download(
            TPDO_COMM1_ID,
            PDO_COMM_TRANSMISSION_TYPE_SUBID,
            &0u8.to_le_bytes(),
        ).await
        .unwrap();

    rx.flush();

    let mut sender = bus.new_sender();
    let sync_msg = SyncObject::new(1).into();
    sender.send(sync_msg).await.unwrap();

    // We expect to receive the sync message just recieved first
    let rx_sync_msg = rx
        .recv(Duration::from_millis(50)).await
        .expect("Expected SYNC message, no CAN message received");
    assert_eq!(sync_msg.id, rx_sync_msg.id);
    // Then expect a PDO message
    let msg = rx
        .recv(Duration::from_millis(50)).await
        .expect("Expected PDO, no CAN message received");
    assert_eq!(CanId::std(0x181), msg.id);
}
