
use zencan_common::messages::NmtState;
use zencan_common::traits::AsyncCanSender;

use zencan_node::node::Node;
use zencan_client::nmt_master::Master;
use integration_tests::sim_bus::SimBus;
//type SimNode<'a> = Node<SimCanSender, SimCanReceiver>;

// fn get_2_devices() -> (SimStack, SimStack) {
//     const MASTER_NODE_ID: u8 = 0;
//     const SLAVE_NODE_ID: u8 = 1;

//     let mut bus = SimBus::new();
//     let (sender, receiver) = bus.new_pair();
//     let master = Stack::new(Some(MASTER_NODE_ID), sender, receiver);
//     let (sender, receiver) = bus.new_pair();
//     let slave = Stack::new(Some(SLAVE_NODE_ID), sender, receiver);
//     (master, slave)
// }


#[tokio::test]
async fn test_nmt_init() {

    const SLAVE_NODE_ID: u8 = 1;
    let od = &integration_tests::object_dict1::OD_TABLE;
    let state = &integration_tests::object_dict1::NODE_STATE;
    let mut node = Node::new(state, od);
    node.set_node_id(SLAVE_NODE_ID);
    let mut bus = SimBus::new(vec![state]);

    let sender = bus.new_sender();
    let receiver = bus.new_receiver();
    let mut master = Master::new(sender, receiver);
    let mut sender = bus.new_sender();

    assert_eq!(NmtState::Bootup, node.nmt_state());


    node.enter_preop(&mut |tx_msg| { futures::executor::block_on(sender.send(tx_msg)).unwrap() });


    assert_eq!(NmtState::PreOperational, node.nmt_state());

    // Master should have received a boot up message
    let nodes = master.get_nodes().await;
    assert_eq!(1, nodes.len());
    assert_eq!(SLAVE_NODE_ID, nodes[0].id);
    assert_eq!(NmtState::PreOperational, nodes[0].state);

    // Broadcast start command
    master.nmt_start(0).await.unwrap();

    // Run a node process call
    bus.process([&mut node].as_mut_slice());

    assert_eq!(NmtState::Operational, node.nmt_state());
    assert_eq!(1, node.rx_message_count());

    master.nmt_stop(0).await.unwrap();
    // Run a node process call
    bus.process([&mut node].as_mut_slice());


    assert_eq!(NmtState::Stopped, node.nmt_state());
    assert_eq!(2, node.rx_message_count());
}
