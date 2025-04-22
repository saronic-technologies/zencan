
use zencan_common::messages::NmtState;
use zencan_common::traits::CanSender;

use zencan_node::node::Node;
use zencan_client::master::Master;
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


#[test]
fn test_nmt_init() {

    const SLAVE_NODE_ID: u8 = 1;
    let od = &integration_tests::object_dict1::OD_TABLE;
    let state = &integration_tests::object_dict1::NODE_STATE;
    let node = Node::new(SLAVE_NODE_ID, state, od);
    let mut bus = SimBus::new(vec![node]);

    let (sender, receiver) = bus.new_pair();
    let mut master = Master::new(sender, receiver);
    let (mut sender, _receiver) = bus.new_pair();



    assert_eq!(NmtState::Bootup, bus.nodes()[0].nmt_state());


    bus.nodes()[0].enter_preop(&mut |tx_msg| sender.send(tx_msg).unwrap());


    assert_eq!(NmtState::PreOperational, bus.nodes()[0].nmt_state());

    // Master should have received a boot up message
    let nodes = master.get_nodes();
    assert_eq!(1, nodes.len());
    assert_eq!(SLAVE_NODE_ID, nodes[0].id);
    assert_eq!(NmtState::PreOperational, nodes[0].state);

    // Broadcast start command
    master.nmt_start(0).unwrap();

    assert_eq!(NmtState::Operational, bus.nodes()[0].nmt_state());
    assert_eq!(1, bus.nodes()[0].rx_message_count());

    master.nmt_stop(0).unwrap();

    assert_eq!(NmtState::Stopped, bus.nodes()[0].nmt_state());
    assert_eq!(2, bus.nodes()[0].rx_message_count());
}
