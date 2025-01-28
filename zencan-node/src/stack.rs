// use core::{marker::PhantomData, mem::MaybeUninit};

// use crate::{
//     messages::{
//         is_std_sdo_request, zencanMessage, NmtCommand, NmtCommandCmd, NmtState,
//     },
//     nmt::NmtSlave,
//     sdo::{SdoClient, SdoServer},
//     traits::{CanFdMessage, CanId, CanReceiver, CanSender, MessageHandler},
// };

// const MAX_SDO_CLIENTS: usize = 256;

// pub struct LssServer {}
// pub struct Stack<S: CanSender, R: CanReceiver> {
//     nmt_slave: NmtSlave,
//     sdo_server: Option<SdoServer>,
//     sdo_clients: [Option<MessageHandler>; MAX_SDO_CLIENTS],
//     sender: S,
//     receiver: R,
//     message_count: u32,
// }


// impl<S: CanSender, R: CanReceiver> Stack<S, R> {
//     pub fn new(node_id: Option<u8>, sender: S, receiver: R) -> Self {
//         let nmt_slave = NmtSlave::new(node_id);
//         let message_count = 0;
//         let sdo_server = None;
//         let sdo_clients = core::array::from_fn(|_| None);

//         Self {
//             nmt_slave,
//             sender,
//             receiver,
//             message_count,
//             sdo_server,
//             sdo_clients,
//         }
//     }

//     pub fn update(&mut self) {
//         let mut messages = 0;
//         while let Some(msg) = self.receiver.try_recv() {
//             // Some messages can only be handled after we have a node id
//             if let Some(node_id) = self.get_node_id() {
//                 if is_std_sdo_request(msg.id(), node_id) {
//                     if let Some(sdo_server) = &mut self.sdo_server {
//                         sdo_server.handle_request(msg.data(), &mut self.sender);
//                     }
//                 }
//             }

//             let open_msg: zencanMessage = if let Ok(m) = msg.try_into() {
//                 m
//             } else {
//                 continue;
//             };
//             self.nmt_slave.update(Some(&open_msg));
//             messages += 1;
//         }
//         self.message_count += messages;

//         if messages == 0 {
//             self.nmt_slave.update(None);
//         }
//     }

//     fn dispatch_msg(&mut self, msg: CanFdMessage) {}

//     pub fn send_nmt_cmd(&mut self, cmd: NmtCommandCmd, node: u8) -> Result<(), ()> {
//         let message = NmtCommand { cmd, node };
//         self.sender.send(message.into()).map_err(|_| ())?;
//         Ok(())
//     }

//     pub fn get_node_id(&self) -> Option<u8> {
//         self.nmt_slave.node_id()
//     }

//     pub fn get_nmt_state(&self) -> NmtState {
//         self.nmt_slave.state()
//     }

//     pub fn rx_message_count(&self) -> u32 {
//         self.message_count
//     }

//     pub fn boot_up(&mut self) {
//         self.sdo_server = Some(SdoServer::new(CanId::Std(
//             0x580 + self.get_node_id().unwrap() as u16,
//         )))
//     }

//     pub fn register_sdo_client(&mut self, client: &mut SdoClient) {
//         for i in 0..self.sdo_clients.len() {
//             if self.sdo_clients[i].is_none() {
//                 self.sdo_clients[i] = Some(client.message_handler());
//                 return
//             }
//         }
//         panic!("Exceeded maximum SDO client allocation")
//     }
// }
