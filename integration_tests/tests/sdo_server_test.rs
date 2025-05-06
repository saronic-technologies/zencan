use zencan_client::sdo_client::SdoClient;
use zencan_common::{sdo::{SdoRequest, SdoResponse}, traits::AsyncCanSender};
use zencan_node::{node::{Node, NodeId}, sdo_server::SdoServer};
use integration_tests::sim_bus::SimBus;
use futures::executor::block_on;

mod utils;
use utils::test_with_background_process;

#[test]
fn test_sdo_server() {
    let mut server = SdoServer::new();

    let od = &integration_tests::object_dict1::OD_TABLE;

    // struct MockSender {
    //     pub last_message: Option<CanFdMessage>,
    // }
    // impl CanSender for MockSender {
    //     fn send(&mut self, msg: CanFdMessage) -> Result<(), CanFdMessage> {
    //         self.last_message = Some(msg);
    //         Ok(())
    //     }
    // }

    // impl MockSender {
    //     fn get_resp(&self) -> SdoResponse {
    //         self.last_message.expect("No response message").try_into().unwrap()
    //     }

    //     fn take_resp(&mut self) -> SdoResponse {
    //         self.last_message.take().expect("No response message").try_into().unwrap()
    //     }
    // }

    //let mut sender = MockSender { last_message: None };

    let resp = server.handle_request(
        &SdoRequest::expedited_download(0x3000, 0, &32u32.to_le_bytes()),
        od,
    ).expect("No response to expedited download");

    assert_eq!(
        resp,
        SdoResponse::ConfirmDownload { index: 0x3000, sub: 0 }
    );

    // TODO: Check value is written to object dict

    let resp = server.handle_request(
        &SdoRequest::initiate_upload(0x3000, 0),
        od,
    ).expect("No response to initiate upload");
    assert_eq!(
        resp,
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

#[tokio::test]
async fn test_sdo_read() {
    const SLAVE_NODE_ID: u8 = 1;

    let od = &integration_tests::object_dict1::OD_TABLE;
    let state = &integration_tests::object_dict1::NODE_STATE;
    let mbox = &integration_tests::object_dict1::NODE_MBOX;
    let mut node = Node::new(NodeId::new(SLAVE_NODE_ID).unwrap(), mbox, state, od);
    let mut bus = SimBus::new(vec![mbox]);
    let mut sender = bus.new_sender();

    test_with_background_process(&mut node, &mut sender, async move {
        let sender = bus.new_sender();
        let receiver = bus.new_receiver();
        let mut client = SdoClient::new_std(SLAVE_NODE_ID, sender, receiver);

        client.download(0x3000, 0, &[0xa, 0xb, 0xc, 0xd]).await.unwrap();
        let read = client.upload(0x3000, 0).await.unwrap();

        assert_eq!(vec![0xa, 0xb, 0xc, 0xd], read);
    }).await;

}
