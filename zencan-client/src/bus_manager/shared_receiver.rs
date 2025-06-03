//! Utility for sharing a single socket among tasks
//!

use std::sync::Arc;
use std::sync::Mutex;

use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};
use zencan_common::{traits::AsyncCanReceiver, CanMessage};

#[derive(Debug)]
pub struct NoMsgError;

#[derive(Debug)]
struct SharedRecieiverInner {
    senders: Vec<Sender<CanMessage>>,
}

impl SharedRecieiverInner {
    // pub fn flush(&mut self) {
    //     while let Some(msg) = self.receiver.try_recv() {
    //         self.senders.retain(|s| {
    //             // return true to retain, or false to drop it Since we are using an unbounded
    //             // channel, the only error should be disconnection of the receiver, which will
    //             // happen when a receiver is dropped
    //             match s.try_send(msg) {
    //                 Err(TrySendError::Closed(_)) => false,
    //                 _ => true,
    //             }
    //         });
    //     }
    // }

    pub fn create_rx(&mut self) -> Receiver<CanMessage> {
        let (tx, rx) = channel(100);
        self.senders.push(tx);
        rx
    }
}

pub struct SharedReceiver {
    _task_handle: JoinHandle<()>,
    inner: Arc<Mutex<SharedRecieiverInner>>,
}

impl SharedReceiver {
    pub fn new<R: AsyncCanReceiver + Send + 'static>(mut receiver: R) -> Self {
        let inner = Arc::new(Mutex::new(SharedRecieiverInner {
            senders: Vec::new(),
        }));
        let inner_clone = inner.clone();
        let task_handle = tokio::spawn(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    let inner = inner_clone.lock().unwrap();
                    for s in &inner.senders {
                        if let Err(_e) = s.try_send(msg) {
                            log::warn!("Dropped received message due to overflow");
                        }
                    }
                };
            }
        });
        Self {
            _task_handle: task_handle,
            inner,
        }
    }

    pub fn create_rx(&mut self) -> SharedReceiverChannel {
        let rx = self.inner.lock().unwrap().create_rx();

        SharedReceiverChannel {
            inner: self.inner.clone(),
            receiver: rx,
        }
    }
}

#[derive(Debug)]
pub struct SharedReceiverChannel {
    /// Data shared with the multi consumer Rx
    inner: Arc<Mutex<SharedRecieiverInner>>,
    /// Our receive channel
    receiver: Receiver<CanMessage>,
}

impl Clone for SharedReceiverChannel {
    fn clone(&self) -> Self {
        let receiver = self.inner.lock().unwrap().create_rx();
        Self {
            inner: self.inner.clone(),
            receiver,
        }
    }
}

#[allow(dead_code)]
impl SharedReceiverChannel {
    /// Remove any pending messages from the queue
    pub fn flush(&mut self) {
        // Clear our queue
        while let Ok(_msg) = self.receiver.try_recv() {}
    }

    pub async fn recv(&mut self) -> Result<CanMessage, NoMsgError> {
        self.receiver.recv().await.ok_or(NoMsgError)
    }

    pub fn try_recv(&mut self) -> Option<CanMessage> {
        self.receiver.try_recv().ok()
    }
}

impl AsyncCanReceiver for SharedReceiverChannel {
    type Error = NoMsgError;

    fn try_recv(&mut self) -> Option<CanMessage> {
        self.try_recv()
    }

    fn recv(&mut self) -> impl core::future::Future<Output = Result<CanMessage, Self::Error>> {
        self.recv()
    }
}
