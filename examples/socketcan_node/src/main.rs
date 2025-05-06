use clap::Parser;
use zencan_node::common::traits::{CanFdMessage, CanId};
use zencan_node::node::{Node, NodeId, NodeIdNum};

use socketcan::tokio::CanSocket;
use socketcan::{CanFrame, EmbeddedFrame, Frame};
use zencan_node::node_mbox::NodeMboxWrite;


mod zencan {
    zencan_node::include_modules!(DEVICE);
}

#[derive(Parser, Debug)]
struct Args {
    socket: String,
    #[clap(long, short, default_value = "255")]
    node_id: u8,
}

#[tokio::main]
async fn main() {

    // Initialize the logger
    env_logger::init();
    let args = Args::parse();

    log::info!("Logging is working...");
    let node_id = NodeId::try_from(args.node_id).unwrap();
    let mut node = Node::new(node_id, &zencan::NODE_MBOX, &zencan::NODE_STATE, &zencan::OD_TABLE);

    // Spawn a message receive background task
    let socket_name = args.socket.clone();

    tokio::spawn(async move {
        let socket = CanSocket::open(&socket_name).unwrap();
        loop {
            let frame: CanFrame = socket.read_frame().await.unwrap();
            let can_id = match frame.can_id() {
                socketcan::CanId::Standard(id) => CanId::Std(id.as_raw()),
                socketcan::CanId::Extended(id) => CanId::Extended(id.as_raw()),
            };

            let msg = CanFdMessage::new(can_id, frame.data());

            if let Err(msg) = zencan::NODE_MBOX.store_message(msg) {
                println!("Unhandled message received: {:?}", msg);
            }
        }
    });


    let socket = CanSocket::open(&args.socket).unwrap();
    loop {
        let mut tx_messages = Vec::new();

        // Run node processing, collecting messages to send
        node.process(&mut |msg: CanFdMessage| {
            tx_messages.push(msg);
        });

        // Now push the collected messages out to the socket
        for msg in tx_messages {
            let can_id = match msg.id() {
                CanId::Std(id) => {
                    socketcan::CanId::Standard(socketcan::StandardId::new(id).unwrap())
                }
                CanId::Extended(id) => {
                    socketcan::CanId::Extended(socketcan::ExtendedId::new(id).unwrap())
                }
            };
            let frame = CanFrame::new(can_id, msg.data()).unwrap();
            socket.write_frame(frame).await.unwrap();
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }


}
