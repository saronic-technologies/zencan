use std::{convert::Infallible, io::Write as _, sync::OnceLock, time::Duration};

use clap::Parser;
use tokio::time::timeout;
use zencan_node::common::{
    traits::{AsyncCanReceiver, AsyncCanSender},
    CanMessage, NodeId,
};
use zencan_node::Node;

use zencan_node::open_socketcan;

mod zencan {
    zencan_node::include_modules!(DEVICE);
}

#[derive(Parser, Debug)]
struct Args {
    socket: String,
    #[clap(long, short, default_value = "255")]
    node_id: u8,
    #[clap(long, short)]
    storage: bool,
}

static OBJECT_STORE_PATH: OnceLock<String> = OnceLock::new();
fn store_objects_callback(reader: &mut dyn embedded_io::Read<Error = Infallible>, _len: usize) {
    let path = OBJECT_STORE_PATH.get().unwrap();
    log::info!("Storing objects to {path}");

    match std::fs::OpenOptions::new().write(true).open(path) {
        Ok(mut f) => {
            let mut buf = [0; 32];
            loop {
                let n = reader.read(&mut buf).unwrap();
                f.write_all(&buf[..n]).unwrap();
                if n != buf.len() {
                    break;
                }
            }
        }
        Err(e) => log::error!("Error storing objects to {}: {:?}", path, e),
    }
}

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init();
    let args = Args::parse();

    log::info!("Logging is working...");
    let node_id = NodeId::try_from(args.node_id).unwrap();
    let mut node = Node::new(
        node_id,
        &zencan::NODE_MBOX,
        &zencan::NODE_STATE,
        &zencan::OD_TABLE,
    );

    if args.storage {
        OBJECT_STORE_PATH
            .set(format!("zencan_node.{}.flash", node_id.raw()))
            .unwrap();
        if let Ok(data) = std::fs::read(OBJECT_STORE_PATH.get().unwrap()) {
            zencan_node::restore_stored_objects(&zencan::OD_TABLE, &data);
        }
        node.register_store_objects(&store_objects_callback);
    }
    let (mut tx, mut rx) = open_socketcan(&args.socket).unwrap();

    // Node requires callbacks be static, so use Box::leak to make static ref from closure on heap
    let process_notify = Box::leak(Box::new(tokio::sync::Notify::new()));
    let notify_cb = Box::leak(Box::new(|| {
        process_notify.notify_one();
    }));
    zencan::NODE_MBOX.set_process_notify_callback(notify_cb);

    // Spawn a task to receive messages
    tokio::spawn(async move {
        loop {
            let msg = match rx.recv().await {
                Ok(msg) => msg,
                Err(e) => {
                    log::error!("Error receiving message: {e:?}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };
            if let Err(msg) = zencan::NODE_MBOX.store_message(msg) {
                log::warn!("Unhandled RX message: {:?}", msg);
            }
        }
    });

    loop {
        let mut tx_messages = Vec::new();

        // Run node processing, collecting messages to send
        node.process(&mut |msg: CanMessage| {
            tx_messages.push(msg);
        });

        // push the collected messages out to the socket
        for msg in tx_messages {
            if let Err(e) = tx.send(msg).await {
                log::error!("Error sending CAN message to socket: {e:?}");
            }
        }

        // Wait for notification to run, or a timeout
        timeout(Duration::from_millis(1), process_notify.notified())
            .await
            .ok();
    }
}
