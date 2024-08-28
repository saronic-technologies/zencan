use futures::{channel::mpsc::Sender, sink::Feed, Future};


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CanId {
    Extended(u32),
    Std(u16),
}

impl CanId {
    pub const fn extended(id: u32) -> CanId {
        CanId::Extended(id)
    }

    pub const fn std(id: u16) -> CanId {
        CanId::Std(id)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CanFdMessage {
    pub data: [u8; 64],
    pub dlc: u8,
    pub id: CanId,
}

impl Default for CanFdMessage {
    fn default() -> Self {
        Self { data: [0; 64], dlc: 0, id: CanId::Std(0) }
    }
}
impl CanFdMessage {
    pub fn id(&self) -> CanId {
        self.id
    }

    pub fn data(&self) -> &[u8] {
        &self.data[0..self.dlc as usize]
    }
}

pub trait CanSender {
    fn send(&mut self, msg: CanFdMessage) -> Result<(), CanFdMessage>;
}

pub trait CanReceiver {
    fn try_recv(&mut self) -> Option<CanFdMessage>;
}


pub struct MessageHandler {
    pub id: CanId,
    pub sender: Sender<CanFdMessage>,
}

// pub(crate) trait MessageHandler {
//     fn wants_id(can_id: CanId) -> bool;

//     async fn handle(msg: &CanFdMessage);
// }

// pub trait ClientManager {
//     fn register_sdo_client()
// }