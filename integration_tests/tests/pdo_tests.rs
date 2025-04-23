//! Test node PDO operations
//!

use std::time::Duration;

use zencan_client::sdo_client::SdoClient;
use zencan_common::{
    messages::SyncObject, objects::ODEntry, traits::{CanId, AsyncCanReceiver, AsyncCanSender}
};
use zencan_node::node::Node;
use zencan_node::node_mbox::{NodeMboxRead, NodeMboxWrite};
use integration_tests::sim_bus::{SimBus, SimBusReceiver, SimBusSender};
use futures::executor::block_on;

fn setup<'a, NS: NodeMboxWrite + NodeMboxRead>(od: &'static [ODEntry], node_state: &'static NS) -> (
    Node<'static, 'static>,
    SdoClient<SimBusSender<'a>, SimBusReceiver>,
    SimBus<'a>,
) {
    const SLAVE_NODE_ID: u8 = 1;

    let mut node = Node::new(node_state, od);
    node.set_node_id(SLAVE_NODE_ID);

    let mut bus = SimBus::new(vec![node_state]);

    let sender = bus.new_sender();
    let receiver = bus.new_receiver();
    let client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

    (node, client, bus)
}


#[tokio::test]
async fn test_tpdo_asignment() {
    let od = &integration_tests::object_dict2::OD_TABLE;
    let state = &integration_tests::object_dict2::NODE_MBOX;
    let (mut node, mut client, mut bus) = setup(od, state);

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
