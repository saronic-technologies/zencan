use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use std::{collections::HashMap, sync::Arc, time::Instant};

use futures::future::join_all;
use zencan_common::lss::LssIdentity;
use zencan_common::messages::{NmtState, ZencanMessage};
use zencan_common::{
    traits::{AsyncCanReceiver, AsyncCanSender},
    NodeId,
};

use super::shared_sender::SharedSender;
use crate::sdo_client::{SdoClient, SdoClientError};

use super::shared_receiver::{SharedReceiver, SharedReceiverChannel};

/// Manage a zencan bus
pub struct BusManager<S: AsyncCanSender + Sync + Send> {
    state_rx: SharedReceiverChannel,
    nodes: HashMap<u8, NodeInfo>,
    sdo_clients: SdoClientMutex<S>,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub node_id: u8,
    pub identity: Option<LssIdentity>,
    pub device_name: Option<String>,
    pub software_version: Option<String>,
    pub hardware_version: Option<String>,
    pub last_seen: Instant,
    pub nmt_state: Option<NmtState>,
}

impl core::fmt::Display for NodeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Node {}: {}\n",
            self.node_id,
            self.nmt_state
                .map(|s| s.to_string())
                .unwrap_or("Unknown State".into())
        )?;
        match self.identity {
            Some(id) => write!(
                f,
                "    Identity vendor: {:X}, product: {:X}, revision: {:X}, serial: {:X}\n",
                id.vendor_id, id.product_code, id.revision, id.serial
            )?,
            None => write!(f, "    Identity: Unknown\n")?,
        }
        write!(
            f,
            "    Device Name: '{}'\n",
            self.device_name.as_deref().unwrap_or("Unknown")
        )?;
        write!(
            f,
            "    Versions: '{}' SW, '{}' HW\n",
            self.software_version.as_deref().unwrap_or("Unknown"),
            self.hardware_version.as_deref().unwrap_or("Unknown")
        )?;
        let age = Instant::now().duration_since(self.last_seen);
        write!(f, "    Last Seen: {}s ago\n", age.as_secs())?;

        Ok(())
    }
}

impl NodeInfo {
    pub fn new(node_id: u8) -> Self {
        Self {
            node_id,
            last_seen: Instant::now(),
            device_name: None,
            identity: None,
            software_version: None,
            hardware_version: None,
            nmt_state: None,
        }
    }

    /// Update / merge new information about the node
    pub fn update(&mut self, info: &NodeInfo) {
        if info.device_name.is_some() {
            self.device_name = info.device_name.clone();
        }
        if info.identity.is_some() {
            self.identity = info.identity.clone();
        }
        if info.software_version.is_some() {
            self.software_version = info.software_version.clone();
        }
        if info.hardware_version.is_some() {
            self.hardware_version = info.hardware_version.clone();
        }
        if info.nmt_state.is_some() {
            self.nmt_state = info.nmt_state.clone();
        }
        self.last_seen = Instant::now();
    }
}

async fn scan_node<S: AsyncCanSender + Sync + Send>(
    node_id: u8,
    clients: &SdoClientMutex<S>,
) -> Option<NodeInfo> {
    let mut sdo_client = clients.lock(node_id);
    log::info!("Scanning Node {node_id}");
    let identity = match sdo_client.read_identity().await {
        Ok(id) => Some(id),
        Err(SdoClientError::NoResponse) => {
            log::warn!("No response");
            return None;
        }
        Err(e) => {
            // A server responded, but we failed to read identity. An unexpected situation, as all
            // nodes should implement the identity object
            log::error!("SDO Abort Response scanning node {node_id} identity: {e:?}");
            None
        }
    };
    let device_name = match sdo_client.read_device_name().await {
        Ok(s) => Some(s),
        Err(SdoClientError::NoResponse) => return None,
        Err(e) => {
            log::error!("SDO Abort Response scanning node {node_id} device name: {e:?}");
            None
        }
    };
    let software_version = match sdo_client.read_software_version().await {
        Ok(s) => Some(s),
        Err(e) => {
            log::error!("SDO Abort Response scanning node {node_id} SW version: {e:?}");
            None
        }
    };
    let hardware_version = match sdo_client.read_hardware_version().await {
        Ok(s) => Some(s),
        Err(e) => {
            log::error!("SDO Abort Response scanning node {node_id} HW version: {e:?}");
            None
        }
    };
    Some(NodeInfo {
        node_id,
        identity,
        device_name,
        software_version,
        hardware_version,
        nmt_state: None,
        last_seen: Instant::now(),
    })
}

struct SdoClientGuarded<'a, S, R>
where
    S: AsyncCanSender,
    R: AsyncCanReceiver,
{
    _guard: std::sync::MutexGuard<'a, ()>,
    client: SdoClient<S, R>,
}

impl<'a, S, R> Deref for SdoClientGuarded<'a, S, R>
where
    S: AsyncCanSender,
    R: AsyncCanReceiver,
{
    type Target = SdoClient<S, R>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl<'a, S, R> DerefMut for SdoClientGuarded<'a, S, R>
where
    S: AsyncCanSender,
    R: AsyncCanReceiver,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

struct SdoClientMutex<S>
where
    S: AsyncCanSender + Sync,
{
    sender: SharedSender<S>,
    receiver: SharedReceiverChannel,
    clients: HashMap<u8, Mutex<()>>,
}

impl<S> SdoClientMutex<S>
where
    S: AsyncCanSender + Sync,
{
    pub fn new(sender: SharedSender<S>, receiver: SharedReceiverChannel) -> Self {
        let mut clients = HashMap::new();
        for i in 0u8..128 {
            clients.insert(i, Mutex::new(()));
        }

        Self {
            sender,
            receiver,
            clients,
        }
    }

    pub fn lock<'a>(&self, id: u8) -> SdoClientGuarded<SharedSender<S>, SharedReceiverChannel> {
        if id < 1 || id > 127 {
            panic!("ID {} out of range", id);
        }
        let guard = self.clients.get(&id).unwrap().lock().unwrap();
        let client = SdoClient::new_std(id, self.sender.clone(), self.receiver.clone());
        SdoClientGuarded {
            _guard: guard,
            client,
        }
    }
}

impl<S: AsyncCanSender + Sync + Send> BusManager<S> {
    pub fn new(sender: S, receiver: impl AsyncCanReceiver + Sync + Send + 'static) -> Self {
        let mut receiver = SharedReceiver::new(receiver);
        let state_rx = receiver.create_rx();
        let sender = SharedSender::new(Arc::new(tokio::sync::Mutex::new(sender)));
        let sdo_clients = SdoClientMutex::new(sender.clone(), receiver.create_rx());
        Self {
            state_rx,
            sdo_clients,
            nodes: HashMap::new(),
        }
    }

    pub fn node_list(&self) -> Vec<NodeInfo> {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for (_id, n) in &self.nodes {
            nodes.push(n.clone());
        }

        nodes.sort_by_key(|n| n.node_id);
        nodes
    }

    pub async fn scan_nodes(&mut self) -> Vec<NodeInfo> {
        const N_PARALLEL: usize = 4;

        let ids = Vec::from_iter(1..128u8);
        let mut nodes = Vec::new();

        let mut chunks = Vec::new();
        for chunk in ids.chunks(128 / N_PARALLEL) {
            chunks.push(Vec::from_iter(chunk.iter().cloned()));
        }

        let mut futures = Vec::new();

        for block in chunks {
            futures.push(async {
                let mut block_nodes = Vec::new();
                for id in block {
                    block_nodes.push(scan_node(id, &self.sdo_clients).await);
                }
                block_nodes
            });
        }

        let results = join_all(futures).await;
        for r in results {
            nodes.extend(r.into_iter().filter(|n| n.is_some()).map(|n| n.unwrap()));
        }

        // Update our nodes
        for n in &nodes {
            self.nodes.insert(n.node_id, n.clone());
        }
        nodes
    }

    pub fn process(&mut self) {
        while let Some(msg) = self.state_rx.try_recv() {
            if let Ok(zencan_msg) = ZencanMessage::try_from(msg) {
                match zencan_msg {
                    ZencanMessage::Heartbeat(heartbeat) => {
                        let id_num = heartbeat.node;
                        if let Ok(node_id) = NodeId::try_from(id_num) {
                            if !self.nodes.contains_key(&id_num) {
                                self.nodes.insert(id_num, NodeInfo::new(node_id.raw()));
                            } else {
                                self.nodes.get_mut(&id_num).unwrap().last_seen = Instant::now();
                            }
                        } else {
                            log::warn!("Invalid heartbeat node ID {id_num} received");
                        }
                    }
                    _ => (),
                }
            }
            let _ = msg;
        }
    }
}
