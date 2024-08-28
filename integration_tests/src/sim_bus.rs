use std::sync::{Arc, Mutex};

use canopen::traits::{CanFdMessage, CanReceiver, CanSender};
use futures::channel::mpsc::{Sender, Receiver, channel, TryRecvError};


type SharedQueueList = Arc<Mutex<Vec<Sender<CanFdMessage>>>>;

pub struct SimCanSender {
    senders: SharedQueueList,
}

impl CanSender for SimCanSender {
    fn send(&mut self, msg: CanFdMessage) -> Result<(), CanFdMessage> {
        let mut senders = self.senders.lock().unwrap();
        for s in &mut *senders {
            s.try_send(msg).map_err(|e| e.into_inner())?;
        }
        Ok(())
    }
}

pub struct SimCanReceiver {
    receiver: Receiver<CanFdMessage>,
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
}

pub struct SimBus {
    senders: SharedQueueList,
}

impl SimBus {
    const QSIZE: usize = 100;

    pub fn new() -> Self {
        let senders = Arc::new(Mutex::new(Vec::new()));
        Self { senders }
    }

    pub fn new_pair(&mut self) -> (SimCanSender, SimCanReceiver) {
        let mut senders = self.senders.lock().unwrap();
        let (tx, rx) = channel(Self::QSIZE);
        senders.push(tx);

        (
            SimCanSender { senders: self.senders.clone() },
            SimCanReceiver { receiver: rx },
        )
    }
}
