use std::future::Future;

use futures::executor::block_on;
use integration_tests::sim_bus::SimBusSender;
use zencan_node::node::Node;
use zencan_common::traits::AsyncCanSender;

#[allow(dead_code)]
pub async fn test_with_background_process<'a, 'b>(
    node: &mut Node<'a>,
    sender: &mut SimBusSender<'b>,
    test_task: impl Future<Output = ()> + 'static,
) {

    // Call process once, to make sure the node is initialized before SDO requests come in
    node.process(&mut |tx_msg| block_on(sender.send(tx_msg)).unwrap());

    let node_process_task = async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            node.process(&mut |tx_msg| block_on(sender.send(tx_msg)).unwrap());
        }
    };

    let _ = tokio::select! {
        _ = node_process_task => {}
        _ = test_task => {}
    };
}
