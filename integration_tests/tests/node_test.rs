use futures::executor::block_on;
use integration_tests::{
    object_dict1,
    sim_bus::{SimBus, SimBusReceiver, SimBusSender},
};
use zencan_client::sdo_client::{SdoClient, SdoClientError};
use zencan_common::{messages::ZencanMessage, objects::ODEntry, sdo::AbortCode, traits::{AsyncCanReceiver, AsyncCanSender}};
use zencan_node::node::Node;
use zencan_node::node_mbox::{NodeMboxRead, NodeMboxWrite};

fn setup<'a, NS: NodeMboxWrite + NodeMboxRead>(
    od: &'static [ODEntry],
    node_state: &'static NS,
) -> (
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

struct BusLogger {
    rx: SimBusReceiver,
}

impl BusLogger {
    pub fn new(rx: SimBusReceiver) -> Self {
        Self { rx }
    }

    pub fn print(&mut self) {
        while let Some(msg) = block_on(self.rx.try_recv()) {
            let parsed_msg: Result<ZencanMessage, _> = msg.try_into();
            if let Ok(msg) = parsed_msg {
                println!("Received message: {:?}", msg);
            } else {
                println!("Received message: {:?}", msg);
            }
        }
    }
}

impl Drop for BusLogger {
    fn drop(&mut self) {
        self.print();
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_string_write() {
    let (mut node, mut client, mut bus) = setup(&object_dict1::OD_TABLE, &object_dict1::NODE_MBOX);
    let mut sender = bus.new_sender();
    let _logger = BusLogger::new(bus.new_receiver());

    node.enter_preop(&mut |tx_msg| block_on(sender.send(tx_msg)).unwrap());

    let node_process_task = async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            node.process(&mut |tx_msg| block_on(sender.send(tx_msg)).unwrap());
        }
    };

    let test_task = async move {
        // Transfer a string short enough to be done expedited
        client.download(0x2002, 0, "Test".as_bytes()).await.unwrap();
        let readback = client.upload(0x2002, 0).await.unwrap();
        assert_eq!("Test".as_bytes(), readback);
        // Transfer a longer string which will do segmented transfer
        client
            .download(0x2002, 0, "Testers".as_bytes())
            .await
            .unwrap();
        let readback = client.upload(0x2002, 0).await.unwrap();
        assert_eq!("Testers".as_bytes(), readback);
        // Transfer an even longer string which will do segmented transfer with two segments
        client
            .download(0x2002, 0, "Testers123".as_bytes())
            .await
            .unwrap();
        let readback = client.upload(0x2002, 0).await.unwrap();
        assert_eq!("Testers123".as_bytes(), readback);
        // Transfer as max-length string (the default value in EDS is 11 characters long)
        client
            .download(0x2002, 0, "Testers1234".as_bytes())
            .await
            .unwrap();
        let readback = client.upload(0x2002, 0).await.unwrap();
        assert_eq!("Testers1234".as_bytes(), readback);
    };

    let _ = tokio::select! {
        _ = node_process_task => {}
        _ = test_task => {}
    };
}

#[tokio::test]
#[serial_test::serial]
async fn test_record_access() {
    const OBJECT_ID: u16 = 0x2001;

    let od = &integration_tests::object_dict1::OD_TABLE;
    let state = &object_dict1::NODE_MBOX;
    let (mut node, mut client, mut bus) = setup(od, state);
    let mut sender = bus.new_sender();

    node.enter_preop(&mut |tx_msg| block_on(sender.send(tx_msg)).unwrap());

    let node_process_task = async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            node.process(&mut |tx_msg| block_on(sender.send(tx_msg)).unwrap());
        }
    };

    let test_task = async move {

        let size_data = client.upload(OBJECT_ID, 0).await.unwrap();
        assert_eq!(1, size_data.len());
        assert_eq!(4, size_data[0]); // Highest sub index supported

        // Check default values of all sub indices
        let sub1_bytes = client.upload(OBJECT_ID, 1).await.unwrap();
        assert_eq!(4, sub1_bytes.len());
        assert_eq!(140, u32::from_le_bytes(sub1_bytes.try_into().unwrap()));
        let sub3_bytes = client.upload(OBJECT_ID, 3).await.unwrap();
        assert_eq!(2, sub3_bytes.len());
        assert_eq!(0x20, u16::from_le_bytes(sub3_bytes.try_into().unwrap()));

        // Write/readback sub1
        client
            .download(OBJECT_ID, 1, &4567u32.to_le_bytes())
            .await
            .unwrap();
        let sub1_bytes = client.upload(OBJECT_ID, 1).await.unwrap();
        assert_eq!(4567, u32::from_le_bytes(sub1_bytes.try_into().unwrap()));

        // Sub3 is read-only; writing should return an abort
        let res = client.download(OBJECT_ID, 3, &100u16.to_le_bytes()).await;
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            SdoClientError::ServerAbort {
                index: OBJECT_ID,
                sub: 3,
                abort_code: AbortCode::ReadOnly as u32
            }
        );
    };

    let _ = tokio::select! {
        _ = node_process_task => {}
        _ = test_task => {}
    };
}

#[tokio::test]
#[serial_test::serial]
async fn test_array_access() {
    const OBJECT_ID: u16 = 0x2000;

    let od = &integration_tests::object_dict1::OD_TABLE;
    let (mut node, mut client, mut bus) = setup(od, &object_dict1::NODE_MBOX);
    let mut sender = bus.new_sender();


    let mut send_cb = |tx_msg| {block_on(sender.send(tx_msg)).unwrap()};
    node.enter_preop(&mut send_cb);

    let node_process_task = async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            node.process(&mut send_cb);
        }
    };

    let test_task = async move {
        let size_data = client.upload(OBJECT_ID, 0).await.unwrap();
        assert_eq!(1, size_data.len());
        assert_eq!(2, size_data[0]); // Highest sub index supported

        // Read back default values
        let data = client.upload(OBJECT_ID, 1).await.unwrap();
        assert_eq!(4, data.len());
        assert_eq!(123, i32::from_le_bytes(data.try_into().unwrap()));

        let data = client.upload(OBJECT_ID, 2).await.unwrap();
        assert_eq!(4, data.len());
        assert_eq!(-1, i32::from_le_bytes(data.try_into().unwrap()));


        // Write and read
        client
            .download(OBJECT_ID, 1, &(-40i32).to_le_bytes())
            .await
            .unwrap();
        let data = client.upload(OBJECT_ID, 1).await.unwrap();
        assert_eq!(-40, i32::from_le_bytes(data.try_into().unwrap()));

        client
            .download(OBJECT_ID, 2, &(99i32).to_le_bytes())
            .await
            .unwrap();
        let data = client.upload(OBJECT_ID, 2).await.unwrap();
        assert_eq!(99, i32::from_le_bytes(data.try_into().unwrap()));
    };

    let _ = tokio::select! {
        _ = node_process_task => {}
        _ = test_task => {}
    };
}
