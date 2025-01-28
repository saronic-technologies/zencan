use std::{cell::{RefCell, RefMut}, rc::Rc, sync::{Arc, Mutex}, time::Duration};

use zencan_common::traits::{CanFdMessage, CanReceiver, CanSender};
use zencan_node::node::Node;
use futures::channel::mpsc::{Sender, Receiver, channel, TryRecvError};


type SharedQueueList = Vec<Sender<CanFdMessage>>;
type SharedSenderList = Vec<RefCell<Box<dyn CanSender>>>;
pub struct SimCanSender<'table, 'cb, const N: usize> {
    senders: Rc<RefCell<SharedQueueList>>,
    nodes: Rc<Vec<RefCell<Node<'table, 'cb, N>>>>,
}

impl<'table, 'cb, const N: usize> CanSender for SimCanSender<'table, 'cb, N> {
    fn send(&mut self, msg: CanFdMessage) -> Result<(), CanFdMessage> {

        for s in self.senders.borrow_mut().iter_mut() {
            s.try_send(msg).map_err(|e| {
                println!("Error sending: {:?}", e);
                e.into_inner()
            })?;
        }

        let mut messages_to_send = Vec::new();
        for node in self.nodes.iter() {
            // Nodes will send messages in response to receiving them. When trying to send the
            // message to node which is currently being delivered to, the borrow will fail because
            // it is already borrowed up the stack frame. This is a good thing; we don't really want
            // to deliver sent messages back to the sender.
            if let Ok(mut sender) = node.try_borrow_mut() {
                sender.handle_message(msg, &mut |tx_msg| messages_to_send.push(tx_msg));
            }
        }

        for msg in messages_to_send {
            self.send(msg).unwrap();
        }

        Ok(())
    }
}

pub struct SimCanReceiver {
    receiver: Receiver<CanFdMessage>,
}

impl SimCanReceiver {
    pub fn flush(&mut self) {
        while let Some(_) = self.try_recv() {}
    }
}

impl CanReceiver for SimCanReceiver {
    fn try_recv(&mut self) -> Option<CanFdMessage> {
        match self.receiver.try_next() {
            Ok(result) => match result {
                Some(msg) => Some(msg),
                None => {
                    println!("Channel closed");
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn recv(&mut self, _timeout: Duration) -> Result<CanFdMessage, ()> {
        match self.receiver.try_next() {
            Ok(result) => match result {
                Some(msg) => {
                    Ok(msg)
                }
                None => {
                    println!("Channel closed");
                    Err(())
                }
            },
            Err(_) => Err(()),
        }
    }
}

// pub struct SimBusBuilder {
//     senders: SharedQueueList,
//     sinks: SharedSenderList,
// }

// impl SimBusBuilder {
//     pub fn new() -> Self {
//         let senders = Vec::new();
//         let sinks = Vec::new();
//         Self { senders,  sinks }
//     }

//     pub fn add_sink(&mut self, node: impl CanSender + 'static) {
//         self.sinks.push(RefCell::new(Box::new(node)));
//     }

//     pub fn build(self) -> SimBus {
//         let senders = self.senders;
//         let nodes = Rc::new(self.nodes);
//         SimBus { senders, sinks }
//     }
// }

pub struct SimBus<'table, 'cb, const N: usize> {
    senders: Rc<RefCell<SharedQueueList>>,
    nodes: Rc<Vec<RefCell<Node<'table, 'cb, N>>>>,
}

impl<'table, 'cb, const N: usize> SimBus<'table, 'cb, N> {
    const QSIZE: usize = 100;

    pub fn new(nodes: Vec<Node<'table, 'cb, N>>) -> Self {
        let senders = Rc::new(RefCell::new(Vec::new()));
        // Vec<Node> -> Vec<RefCell<Node>>
        let nodes = Rc::new(nodes.into_iter().map(|n| RefCell::new(n)).collect());
        Self { senders, nodes }
    }

    pub fn new_pair(&mut self) -> (SimCanSender<'table, 'cb, N>, SimCanReceiver) {
        let sender = self.new_sender();
        let (tx, rx) = channel(Self::QSIZE);
        self.senders.borrow_mut().push(tx);
        let receiver = SimCanReceiver { receiver: rx };
        (sender, receiver)
    }

    pub fn new_sender(&mut self) -> SimCanSender<'table, 'cb, N> {
        let (senders, nodes) = (self.senders.clone(), self.nodes.clone());
        SimCanSender { senders, nodes }
    }

    /// Accessor to allow tests to access nodes on the bus while they are owned by the SimBus
    pub fn nodes(&mut self) -> Vec<RefMut<Node<'table, 'cb, N>>> {
        self.nodes.iter().map(|n| n.borrow_mut()).collect()
    }
}

// pub struct NodeWrapper<'a> {
//     pub node: Node<'a>,
//     pub sender: SimCanSender<'a>,
// }

// impl<'a> NodeWrapper<'a> {
//     pub fn new(node: Node<'a>, sender: SimCanSender<'a>) -> Self {
//         Self { node, sender }
//     }
// }

// impl<'a> CanSender for NodeWrapper<'a> {
//     fn send(&mut self, msg: CanFdMessage) -> Result<(), CanFdMessage> {
//         self.node.handle_message(msg, &mut |msg| self.sender.send(msg).unwrap());
//         Ok(())
//     }
// }