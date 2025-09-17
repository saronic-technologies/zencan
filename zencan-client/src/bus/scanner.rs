use std::collections::HashMap;

use futures::future::join_all;
use snafu::Snafu;
use zencan_common::{lss::LssIdentity, AsyncCanReceiver, AsyncCanSender};

use crate::{sdo_client::ISDOClientBuilder, LssError, SdoClient, SdoClientError};

/// Error returned by scanner operations
#[derive(Clone, Debug, PartialEq, Snafu)]
pub enum ScannerError {
    /// Failed to get identity from a scanned node
    #[snafu(display("Cannot get identity from node {node_id}: {source}"))]
    IdentityReadFailed {
        /// The node ID that failed to provide identity
        node_id: u8,
        /// The underlying SDO error
        source: SdoClientError,
    },
    /// Unknown Error
    UnknownError
}

#[derive(Debug, Clone)]
pub struct BusNode {
    /// The node's CAN ID (1-127)
    pub node_id: u8,
    /// The node's LSS identity information
    /// This is required as we need to be able to pair
    /// nodes with their 128-bit identities
    pub identity: LssIdentity,
    /// The device name as reported by the node
    pub device_name: Option<String>,
    /// The software version as reported by the node
    pub software_version: Option<String>,
    /// The hardware version as reported by the node
    pub hardware_version: Option<String>,
}

async fn scan_node<S: AsyncCanSender + Sync + Send, R :AsyncCanReceiver + Sync + Send>(
    node_id: u8,
    mut sdo_client :SdoClient<S, R>
) -> Result<BusNode, ScannerError> {
    log::info!("Scanning Node {node_id}");
    let identity = match sdo_client.read_identity().await {
        Ok(id) => id,
        Err(SdoClientError::NoResponse) => {
            log::info!("No response from node {node_id}");
            return Err(ScannerError::IdentityReadFailed { node_id, source: SdoClientError::NoResponse });
        }
        Err(e) => {
            // A server responded, but we failed to read identity. An unexpected situation, as all
            // nodes should implement the identity object
            log::error!("SDO Abort Response scanning node {node_id} identity: {e:?}");
            return Err(ScannerError::IdentityReadFailed { node_id, source: e });
        }
    };
    let device_name = match sdo_client.read_device_name().await {
        Ok(s) => Some(s),
        Err(SdoClientError::NoResponse) => None,
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

    Ok(BusNode {
        node_id,
        identity,
        device_name,
        software_version,
        hardware_version,
    })
}

/// The bus scanner is used just to scan a CANOpen bus by node, which we provide
/// a helper method for
pub struct BusScanner<S, R> 
    where S :AsyncCanSender, R :AsyncCanReceiver {

    // We use a builder so we can control when our receiver and sender are
    // actually constructed, and when they are destroyed.  This works well for
    // sockets, because we don't have them open longer than they need to be
    sdo_client_builder :Box<dyn ISDOClientBuilder<S, R>>,
}

impl<S: AsyncCanSender + Sync + Send, R :AsyncCanReceiver + Sync + Send> BusScanner<S, R> {
    /// Create a new Bus Scanner
    pub fn new(
        sdo_client_builder :Box<dyn ISDOClientBuilder<S, R>>
    ) -> Self {
        Self {
            sdo_client_builder
        }
    }

    /// Perform a bus scan
    pub async fn scan(&mut self, node_ids :&[u8]) -> Result<Vec<BusNode>, ScannerError> {
        let mut return_value :Vec<BusNode> = vec![];

        const N_PARALLEL: usize = 10;

        let mut futures = Vec::new();

        for chunk in node_ids.chunks(128 / N_PARALLEL) {
            let chunk = Vec::from_iter(chunk.iter().cloned());
            // Pair the node ID with its SDO client
            let block_values :Vec<(u8, std::result::Result<SdoClient<S, R>, Box<dyn std::error::Error + Send + Sync>>)> =
                chunk.iter().map(
                  |node_id| (*node_id, self.sdo_client_builder.set_node_id(*node_id).build())
                ).collect();
            futures.push(async {
                let mut block_nodes = Vec::new();
                for block_data in block_values {
                    match scan_node(block_data.0, block_data.1.map_err(|_| ScannerError::UnknownError)?).await {
                        Ok(node) => block_nodes.push(node),
                        Err(e) => return Err(e),
                    }
                }
                Ok(block_nodes)
            });
        }

        // for block in chunks {
        //     futures.push(async {
        //         let mut block_nodes = Vec::new();
        //         for block_data in block {
        //             block_nodes.push(
        //                 scan_node(
        //                     block_data.0, 
        //                     block_data.1
        //                 ).await
        //             );
        //         }
        //         block_nodes
        //     });
        // }

        let results = join_all(futures).await;
        for result in results {
            match result {
                Ok(nodes) => return_value.extend(nodes),
                Err(e) => return Err(e),
            }
        }

        Ok(return_value)
    }
}
