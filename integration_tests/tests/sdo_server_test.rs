use zencan_client::sdo_client::SdoClient;
use zencan_common::{objects::ObjectDict, sdo::{SdoRequest, SdoResponse}, traits::{CanFdMessage, CanId, CanSender}};
use zencan_node::{node::Node, sdo_server::SdoServer};
use integration_tests::sim_bus::SimBus;


#[test]
fn test_sdo_server() {
    const TX_COB_ID: CanId = CanId::Std(0x600);
    let mut server = SdoServer::new(TX_COB_ID);

    let mut od = integration_tests::object_dict1::get_od();

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
        &SdoRequest::expedited_download(0x3000, 0, &32u32.to_le_bytes()),
        &mut od,
        &mut |tx_msg| sender.send(tx_msg).unwrap(),
    );

    assert_eq!(
        sender.take_resp(),
        SdoResponse::ConfirmDownload { index: 0x3000, sub: 0 }
    );

    // TODO: Check value is written to object dict

    server.handle_request(
        &SdoRequest::initiate_upload(0x3000, 0),
        &mut od,
        &mut |tx_msg| sender.send(tx_msg).unwrap(),
    );
    assert_eq!(
        sender.take_resp(),
        SdoResponse::ConfirmUpload {
            n: 0,
            e: true,
            s: true,
            index: 0x3000,
            sub: 0,
            data: 32u32.to_le_bytes(),
        }
    );
}

#[test]
fn test_sdo_read() {
    const SLAVE_NODE_ID: u8 = 1;

    let od = integration_tests::object_dict1::get_od();
    let node = Node::new(SLAVE_NODE_ID, od);
    let mut bus = SimBus::new(vec![node]);
    let mut sender = bus.new_sender();

    bus.nodes()[0].enter_preop(&mut |tx_msg| sender.send(tx_msg).unwrap());

    let (sender, receiver) = bus.new_pair();
    let mut client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

    client.download(0x3000, 0, &[0xa, 0xb, 0xc, 0xd]).unwrap();
    let read = client.upload(0x3000, 0).unwrap();

    assert_eq!(vec![0xa, 0xb, 0xc, 0xd], read);
}
