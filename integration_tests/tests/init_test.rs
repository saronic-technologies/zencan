use std::time::Duration;

use canopen_common::{messages::{
    self,
    NmtCommandCmd::{self, Start},
    NmtState,
}, objects::{Object, ObjectDict, OD_TABLE}, sdo::{SdoRequest, SdoResponse}, traits::{CanFdMessage, CanId, CanReceiver}};
use canopen_common::traits::CanSender;

use canopen_node::{node::Node, sdo_server::SdoServer};
use canopen_client::{master::Master, sdo_client::SdoClient};
use integration_tests::sim_bus::{SimBus, SimCanReceiver, SimCanSender};
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
    use CanReceiver;

    const SLAVE_NODE_ID: u8 = 1;
    let mut od = ObjectDict { table: &OD_TABLE };

    let mut bus = SimBus::new();
    let (sender, receiver) = bus.new_pair();
    let mut master = Master::new(sender, receiver);
    let (sender, mut receiver) = bus.new_pair();
    let mut slave = Node::new(SLAVE_NODE_ID, sender, od);

    let mut dispatch = |slave: &mut Node<SimCanSender>| {
        while let Some(msg) = receiver.try_recv() {
            slave.handle_message(msg);
        }
    };

    dispatch(&mut slave);


    assert_eq!(NmtState::Bootup, slave.nmt_state());


    slave.enter_preop();


    dispatch(&mut slave);

    assert_eq!(NmtState::PreOperational, slave.nmt_state());

    // Master should have received a boot up message
    let nodes = master.get_nodes();
    assert_eq!(1, nodes.len());
    assert_eq!(SLAVE_NODE_ID, nodes[0].id);
    assert_eq!(NmtState::PreOperational, nodes[0].state);

    // Broadcast start command
    master.nmt_start(0).unwrap();

    dispatch(&mut slave);

    assert_eq!(NmtState::Operational, slave.nmt_state());
    assert_eq!(1, slave.rx_message_count());

    master.nmt_stop(0).unwrap();

    dispatch(&mut slave);

    assert_eq!(NmtState::Stopped, slave.nmt_state());
    assert_eq!(2, slave.rx_message_count());
}

#[test]
fn test_sdo_server() {
    const TX_COB_ID: CanId = CanId::Std(0x600);
    let mut server = SdoServer::new(TX_COB_ID);

    let mut od = ObjectDict { table: &OD_TABLE };

    struct MockSender {
        pub last_message: Option<CanFdMessage>,
    }
    impl CanSender for MockSender {
        fn send(&mut self, msg: CanFdMessage) -> Result<(), CanFdMessage> {
            self.last_message = Some(msg);
            Ok(())
        }
    }

    impl MockSender {
        fn get_resp(&self) -> SdoResponse {
            self.last_message.expect("No response message").try_into().unwrap()
        }

        fn take_resp(&mut self) -> SdoResponse {
            self.last_message.take().expect("No response message").try_into().unwrap()
        }
    }

    let mut sender = MockSender { last_message: None };

    server.handle_request(
        &SdoRequest::expedited_download(0x1000, 0, &32u32.to_le_bytes()),
        &mut od,
        &mut sender
    );

    assert_eq!(
        sender.take_resp(),
        SdoResponse::ConfirmDownload { index: 0x1000, sub: 0 }
    );

    // TODO: Check value is written to object dict

    server.handle_request(
        &SdoRequest::initiate_upload(0x1000, 0),
        &mut od,
        &mut sender
    );
    assert_eq!(
        sender.take_resp(),
        SdoResponse::ConfirmUpload {
            n: 0,
            e: true,
            s: true,
            index: 0x1000,
            sub: 0,
            data: 32u32.to_le_bytes(),
        }
    );


}

#[test]
fn test_sdo_read() {
    const SLAVE_NODE_ID: u8 = 1;

    let mut bus = SimBus::new();
    let (sender, mut receiver) = bus.new_pair();
    let od = ObjectDict { table: &OD_TABLE };
    let mut node1 = Node::new(SLAVE_NODE_ID, sender, od);

    node1.enter_preop();

    // Yuck, threaded tests! But...need to have the slave node processing messages while master
    // blocks on responses
    std::thread::spawn(move || {
        loop {
            while let Some(msg) = receiver.try_recv() {
                node1.handle_message(msg);
            }
            std::thread::sleep(Duration::from_micros(500));
        }
    });

    let (sender, receiver) = bus.new_pair();
    let mut client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

    client.download(0x1000, 0, &[0xa, 0x44]).unwrap();
    let read = client.upload(0x1000, 0).unwrap();

    assert_eq!(vec![0xa, 0x44], read);
}
