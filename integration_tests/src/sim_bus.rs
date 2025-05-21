use std::{cell::RefCell, sync::Arc};

use zencan_common::messages::CanMessage;
use zencan_common::traits::{AsyncCanReceiver, AsyncCanSender};
use zencan_node::node::Node;
use zencan_node::node_mbox::NodeMboxWrite;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct SimBus<'a> {
    node_states: Arc<RefCell<Vec<&'a dyn NodeMboxWrite>>>,
    /// List of all the open channels for sending recieved messages to
    receiver_channels: Arc<RefCell<Vec<UnboundedSender<CanMessage>>>>,
}

impl<'a> SimBus<'a> {
    pub fn new(node_states: Vec<&'a dyn NodeMboxWrite>) -> Self {
        Self {
            node_states: Arc::new(RefCell::new(node_states)),
            receiver_channels: Arc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn new_receiver(&mut self) -> SimBusReceiver {
        let (tx, rx) = unbounded_channel();
        self.receiver_channels.borrow_mut().push(tx);
        SimBusReceiver { channel_rx: rx }
    }

    pub fn new_sender(&mut self) -> SimBusSender<'a> {
        SimBusSender {
            node_states: self.node_states.clone(),
            external_channels: self.receiver_channels.clone(),
        }
    }

    pub fn process(&mut self, nodes: &mut [&mut Node]) {
        let mut to_deliver = Vec::new();
        for (i, n) in nodes.iter_mut().enumerate() {
            n.process(&mut |msg_to_send| to_deliver.push((i, msg_to_send.clone())));
        }
        for (sender_idx, msg) in &to_deliver {
            // Send the message to all nodes except the one that sent it
            for (i, n) in self.node_states.borrow().iter().enumerate() {
                if i != *sender_idx {
                    n.store_message(msg.clone()).ok();
                }
            }
        }
    }
}

pub struct SimBusSender<'a> {
    node_states: Arc<RefCell<Vec<&'a dyn NodeMboxWrite>>>,
    external_channels: Arc<RefCell<Vec<UnboundedSender<CanMessage>>>>,
}

impl<'a> AsyncCanSender for SimBusSender<'a> {
    async fn send(&mut self, msg: CanMessage) -> Result<(), CanMessage> {
        // Send to nodes on the bus
        for ns in self.node_states.borrow().iter() {
            // It doesn't matter if store message fails; that just means the node did not
            // recognize/accept the message
            ns.store_message(msg).ok();
        }
        // Send to external listeners on the bus (those created by `new_receiver()``)
        for rx in self.external_channels.borrow_mut().iter() {
            rx.send(msg.clone()).unwrap();
        }

        Ok(())
    }
}

pub struct SimBusReceiver {
    channel_rx: UnboundedReceiver<CanMessage>,
}

impl SimBusReceiver {
    pub fn flush(&mut self) {
        while let Ok(_) = self.channel_rx.try_recv() {}
    }
}

impl AsyncCanReceiver for SimBusReceiver {
    type Error = ();

    async fn recv(&mut self) -> Result<CanMessage, Self::Error> {
        self.channel_rx.recv().await.ok_or(())
    }

    fn try_recv(&mut self) -> Option<CanMessage> {
        match self.channel_rx.try_recv() {
            Ok(msg) => Some(msg),
            Err(_) => None,
        }
    }

    fn flush(&mut self) {
        while let Ok(_) = self.channel_rx.try_recv() {}
    }
}
