use canopen::{
    messages::{
        self,
        NmtCommandCmd::{self, Start},
        NmtState,
    },
    stack::Stack, sdo::SdoClient,
};
use integration_tests::sim_bus::{SimBus, SimCanReceiver, SimCanSender};

type SimStack<'a> = Stack<'a, SimCanSender, SimCanReceiver>;

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
    const MASTER_NODE_ID: u8 = 0;
    const SLAVE_NODE_ID: u8 = 1;

    let mut bus = SimBus::new();
    let (sender, receiver) = bus.new_pair();
    let mut master = Stack::new(Some(MASTER_NODE_ID), sender, receiver);
    let (sender, receiver) = bus.new_pair();
    let mut slave = Stack::new(Some(SLAVE_NODE_ID), sender, receiver);

    master.update();
    slave.update();

    assert_eq!(NmtState::PreOperational, slave.get_nmt_state());

    master
        .send_nmt_cmd(NmtCommandCmd::Start, SLAVE_NODE_ID)
        .unwrap();

    master.update();
    slave.update();

    assert_eq!(NmtState::Operational, slave.get_nmt_state());
    assert_eq!(1, slave.rx_message_count());

    master
        .send_nmt_cmd(NmtCommandCmd::Stop, SLAVE_NODE_ID)
        .unwrap();
    master.update();
    slave.update();

    assert_eq!(NmtState::Stopped, slave.get_nmt_state());
    assert_eq!(2, slave.rx_message_count());
}

#[test]
fn test_sdo_read() {
    const MASTER_NODE_ID: u8 = 0;
    const SLAVE_NODE_ID: u8 = 1;

    let mut bus = SimBus::new();
    let (sender, receiver) = bus.new_pair();
    let mut master = Stack::new(Some(MASTER_NODE_ID), sender, receiver);
    let (sender, receiver) = bus.new_pair();
    let mut slave = Stack::new(Some(SLAVE_NODE_ID), sender, receiver);

    let client = SdoClient::new_std(SLAVE_NODE_ID);
    master.register_sdo_client(&client);
}
