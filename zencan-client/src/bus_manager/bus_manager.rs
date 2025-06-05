use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use std::{collections::HashMap, sync::Arc, time::Instant};

use futures::future::join_all;
use tokio::task::JoinHandle;
use zencan_common::lss::LssIdentity;
use zencan_common::messages::{NmtCommand, NmtCommandCmd, NmtState, ZencanMessage};
use zencan_common::{
    traits::{AsyncCanReceiver, AsyncCanSender},
    NodeId,
};

use super::shared_sender::SharedSender;
use crate::sdo_client::{SdoClient, SdoClientError};

use super::shared_receiver::{SharedReceiver, SharedReceiverChannel};

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
        writeln!(
            f,
            "Node {}: {}",
            self.node_id,
            self.nmt_state
                .map(|s| s.to_string())
                .unwrap_or("Unknown State".into())
        )?;
        match self.identity {
            Some(id) => writeln!(
                f,
                "    Identity vendor: {:X}, product: {:X}, revision: {:X}, serial: {:X}",
                id.vendor_id, id.product_code, id.revision, id.serial
            )?,
            None => writeln!(f, "    Identity: Unknown")?,
        }
        writeln!(
            f,
            "    Device Name: '{}'",
            self.device_name.as_deref().unwrap_or("Unknown")
        )?;
        writeln!(
            f,
            "    Versions: '{}' SW, '{}' HW",
            self.software_version.as_deref().unwrap_or("Unknown"),
            self.hardware_version.as_deref().unwrap_or("Unknown")
        )?;
        let age = Instant::now().duration_since(self.last_seen);
        writeln!(f, "    Last Seen: {}s ago", age.as_secs())?;

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
            self.identity = info.identity;
        }
        if info.software_version.is_some() {
            self.software_version = info.software_version.clone();
        }
        if info.hardware_version.is_some() {
            self.hardware_version = info.hardware_version.clone();
        }
        if info.nmt_state.is_some() {
            self.nmt_state = info.nmt_state;
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

pub struct SdoClientGuard<'a, S, R>
where
    S: AsyncCanSender,
    R: AsyncCanReceiver,
{
    _guard: std::sync::MutexGuard<'a, ()>,
    client: SdoClient<S, R>,
}

impl<S, R> Deref for SdoClientGuard<'_, S, R>
where
    S: AsyncCanSender,
    R: AsyncCanReceiver,
{
    type Target = SdoClient<S, R>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl<S, R> DerefMut for SdoClientGuard<'_, S, R>
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

    pub fn lock(&self, id: u8) -> SdoClientGuard<SharedSender<S>, SharedReceiverChannel> {
        if !(1..=127).contains(&id) {
            panic!("ID {} out of range", id);
        }
        let guard = self.clients.get(&id).unwrap().lock().unwrap();
        let client = SdoClient::new_std(id, self.sender.clone(), self.receiver.clone());
        SdoClientGuard {
            _guard: guard,
            client,
        }
    }
}

/// Manage a zencan bus
pub struct BusManager<S: AsyncCanSender + Sync + Send> {
    sender: SharedSender<S>,
    nodes: Arc<tokio::sync::Mutex<HashMap<u8, NodeInfo>>>,
    sdo_clients: SdoClientMutex<S>,
    _monitor_task: JoinHandle<()>,
}

impl<S: AsyncCanSender + Sync + Send> BusManager<S> {
    pub fn new(sender: S, receiver: impl AsyncCanReceiver + Sync + 'static) -> Self {
        let mut receiver = SharedReceiver::new(receiver);
        let sender = SharedSender::new(Arc::new(tokio::sync::Mutex::new(sender)));
        let sdo_clients = SdoClientMutex::new(sender.clone(), receiver.create_rx());

        let mut state_rx = receiver.create_rx();
        let nodes = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        let monitor_task = {
            let nodes = nodes.clone();
            tokio::spawn(async move {
                loop {
                    if let Ok(msg) = state_rx.recv().await {
                        if let Ok(ZencanMessage::Heartbeat(heartbeat)) =
                            ZencanMessage::try_from(msg)
                        {
                            let id_num = heartbeat.node;
                            if let Ok(node_id) = NodeId::try_from(id_num) {
                                let mut nodes = nodes.lock().await;
                                if let std::collections::hash_map::Entry::Vacant(e) =
                                    nodes.entry(id_num)
                                {
                                    e.insert(NodeInfo::new(node_id.raw()));
                                } else {
                                    let node = nodes.get_mut(&id_num).unwrap();
                                    node.nmt_state = Some(heartbeat.state);
                                    node.last_seen = Instant::now();
                                }
                            } else {
                                log::warn!("Invalid heartbeat node ID {id_num} received");
                            }
                        }
                    }
                }
            })
        };

        Self {
            sender,
            sdo_clients,
            nodes,
            _monitor_task: monitor_task,
        }
    }

    /// Get an SDO client for a particular node
    ///
    /// This function may block if another task is using the required SDO client, as it ensures
    /// exclusive access to each node's SDO server.
    pub fn sdo_client(
        &self,
        node_id: u8,
    ) -> SdoClientGuard<SharedSender<S>, SharedReceiverChannel> {
        self.sdo_clients.lock(node_id)
    }

    pub async fn node_list(&self) -> Vec<NodeInfo> {
        let node_map = self.nodes.lock().await;
        let mut nodes = Vec::with_capacity(node_map.len());
        for n in node_map.values() {
            nodes.push(n.clone());
        }

        nodes.sort_by_key(|n| n.node_id);
        nodes
    }

    pub async fn scan_nodes(&mut self) -> Vec<NodeInfo> {
        const N_PARALLEL: usize = 10;

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
            nodes.extend(r.into_iter().flatten());
        }

        let mut node_map = self.nodes.lock().await;
        // Update our nodes
        for n in &nodes {
            node_map.insert(n.node_id, n.clone());
        }
        nodes
    }

    /// Send application reset command
    ///
    /// node - The node ID to command, or 0 to broadcast to all nodes
    pub async fn nmt_reset_app(&mut self, node: u8) {
        self.send_nmt_cmd(NmtCommandCmd::ResetApp, node).await
    }

    /// Send communications reset command
    ///
    /// node - The node ID to command, or 0 to broadcast to all nodes
    pub async fn nmt_reset_comms(&mut self, node: u8) {
        self.send_nmt_cmd(NmtCommandCmd::ResetComm, node).await
    }

    /// Send start operation command
    ///
    /// node - The node ID to command, or 0 to broadcast to all nodes
    pub async fn nmt_start(&mut self, node: u8) {
        self.send_nmt_cmd(NmtCommandCmd::Start, node).await
    }

    /// Send start operation command
    ///
    /// node - The node ID to command, or 0 to broadcast to all nodes
    pub async fn nmt_stop(&mut self, node: u8) {
        self.send_nmt_cmd(NmtCommandCmd::Stop, node).await
    }

    async fn send_nmt_cmd(&mut self, cmd: NmtCommandCmd, node: u8) {
        let message = NmtCommand { cmd, node };
        self.sender.send(message.into()).await.ok();
    }
}
