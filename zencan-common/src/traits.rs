use core::time::Duration;


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

    pub fn raw(&self) -> u32 {
        match self {
            CanId::Extended(id) => *id,
            CanId::Std(id) => *id as u32,
        }
    }

    pub fn is_extended(&self) -> bool {
        match self {
            CanId::Extended(_) => true,
            CanId::Std(_) => false,
        }
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
    pub fn new(id: CanId, data: &[u8]) -> Self {
        let mut msg = Self::default();
        msg.id = id;
        msg.dlc = data.len() as u8;
        if msg.dlc > 64 {
            panic!("Data length exceeds maximum size of 64 bytes");
        }
        msg.data[0..msg.dlc as usize].copy_from_slice(data);
        msg
    }

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
    /// A blocking receive
    fn recv(&mut self, timeout: Duration) -> Result<CanFdMessage, ()>;
}

pub trait AsyncCanSender {
    fn send(&mut self, msg: CanFdMessage) -> impl core::future::Future<Output = Result<(), CanFdMessage>>;
}

pub trait AsyncCanReceiver {
    fn try_recv(&mut self) -> impl core::future::Future<Output = Option<CanFdMessage>>;
    /// A blocking receive
    fn recv(&mut self, timeout: Duration) ->  impl core::future::Future<Output = Result<CanFdMessage, ()>>;
}

// pub(crate) trait MessageHandler {
//     fn wants_id(can_id: CanId) -> bool;

//     async fn handle(msg: &CanFdMessage);
// }

// pub trait ClientManager {
//     fn register_sdo_client()
// }