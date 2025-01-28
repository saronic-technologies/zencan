//! Test node PDO operations
//!

use std::time::Duration;

use zencan_client::sdo_client::{SdoClient, SdoClientError};
use zencan_common::{
    messages::SyncObject,
    objects::ObjectDict,
    sdo::AbortCode,
    traits::{CanId, CanReceiver, CanSender},
};
use zencan_node::node::Node;
use integration_tests::sim_bus::{SimBus, SimCanReceiver, SimCanSender};

fn setup<'a, const N: usize>(
    od: ObjectDict<'static, 'a, N>,
) -> (
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
fn test_tpdo_asignment() {
    let (mut client, mut bus) = setup(integration_tests::object_dict2::get_od());
    let mut sender = bus.new_sender();
    let (_sender, mut rx) = bus.new_pair();
    bus.nodes()[0].enter_preop(&mut |tx_msg| sender.send(tx_msg).unwrap());

    // Set COB-ID
    const TPDO_COMM1_ID: u16 = 0x1800;
    const PDO_COMM_COB_SUBID: u8 = 1;
    const PDO_COMM_TRANSMISSION_TYPE_SUBID: u8 = 2;

    // Set the TPDO COB ID
    client
        .download(TPDO_COMM1_ID, PDO_COMM_COB_SUBID, &0x181u32.to_le_bytes())
        .unwrap();
    // Set to sync driven
    client
        .download(
            TPDO_COMM1_ID,
            PDO_COMM_TRANSMISSION_TYPE_SUBID,
            &0u8.to_le_bytes(),
        )
        .unwrap();

    rx.flush();

    let mut sender = bus.new_sender();
    let sync_msg = SyncObject::new(1).into();
    sender.send(sync_msg).unwrap();

    // We expect to receive the sync message just recieved first
    let rx_sync_msg = rx
        .recv(Duration::from_millis(50))
        .expect("Expected SYNC message, no CAN message received");
    assert_eq!(sync_msg.id, rx_sync_msg.id);
    // Then expect a PDO message
    let msg = rx
        .recv(Duration::from_millis(50))
        .expect("Expected PDO, no CAN message received");
    assert_eq!(CanId::std(0x181), msg.id);
}
