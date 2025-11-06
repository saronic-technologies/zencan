use futures::future::join_all;
use snafu::Snafu;
use zencan_common::{lss::LssIdentity};

use crate::{sdo_client::ISDOClientBuilder, SdoClient, SdoClientError};

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
    #[snafu(display("Duplicate node detected {node_id}"))]
    DuplicateNodeDetected {
        node_id :u8
    },
    /// Unknown Error
    UnknownError
}

#[derive(Debug, Clone, Copy)]
/// Resulting Bus node from a Bus scanner's scan
pub struct BusNode {
    /// The node's CAN ID (1-127)
    pub node_id: u8,
    /// The node's LSS identity information
    /// This is required as we need to be able to pair
    /// nodes with their 128-bit identities
    pub identity: LssIdentity,
}

async fn scan_node(
    node_id: u8,
    sdo_client :SdoClient
) -> anyhow::Result<Option<BusNode>> {
    log::info!("Scanning Node {node_id}");
    let identity = match sdo_client.read_identity().await {
        Ok(id) => id,
        Err(SdoClientError::NoResponse) => {
            log::info!("No response from node {node_id}");
            return Ok(None);
        }
        // When we read the identity, we perform 4 reads.  If there is a duplicate node, the
        // first read will trigger 2 of the same response in the socketbuff, the second of which
        // will be de-queued when we try to read the response to the actual second request.  The
        // index and sub-index won't match, and we will get the MismatchedObject error.

        Err(SdoClientError::MismatchedObjectIndex { expected :_, received :_ }) => {
            return Err(ScannerError::DuplicateNodeDetected { node_id: node_id }.into());
        }
        Err(e) => {
            // A server responded, but we failed to read identity. An unexpected situation, as all
            // nodes should implement the identity object
            log::error!("SDO Abort Response scanning node {node_id} identity: {e:?}");
            return Err(ScannerError::IdentityReadFailed { node_id, source: e }.into());
        }
    };

    Ok(Some(BusNode {
        node_id,
        identity,
    }))
}

/// The bus scanner is used just to scan a CANOpen bus by node, which we provide
/// a helper method for
pub struct BusScanner {
    // We use a builder so we can control when our receiver and sender are
    // actually constructed, and when they are destroyed.  This works well for
    // sockets, because we don't have them open longer than they need to be
    sdo_client_builder :Box<dyn ISDOClientBuilder>,
}

impl BusScanner {
    /// Create a new Bus Scanner
    pub fn new(
        sdo_client_builder :Box<dyn ISDOClientBuilder>
    ) -> anyhow::Result<Self> {
        Ok(Self {
            sdo_client_builder
        })
    }

    /// Scans the entire CanOPEN Bus (128 possible nodes)
    pub async fn full_scan(&mut self) -> anyhow::Result<Vec<BusNode>> {
        let full_range :Vec<u8> = (0u8..128).collect();
        self.scan(&full_range).await
    }

    /// Perform a bus scan
    // Mutable because we modify our builder for each scan, to get an SdoClient that
    // we use to perform the scan.
    pub async fn scan(&mut self, node_ids :&[u8]) -> anyhow::Result<Vec<BusNode>> {
        let mut return_value :Vec<BusNode> = vec![];

        const N_PARALLEL: usize = 10;

        let mut futures = Vec::new();

        for chunk in node_ids.chunks(128 / N_PARALLEL) {
            let chunk = Vec::from_iter(chunk.iter().cloned());
            // Pair the node ID with its SDO client
            let block_values :Vec<(u8, anyhow::Result<SdoClient>)> =
                chunk.iter().map(
                  |node_id| (*node_id, self.sdo_client_builder.set_node_id(*node_id).build())
                ).collect();
            // We've built the SDO client for this node ID, so now we can make a future that
            // scans the specific chunk we are currently on
            futures.push(async {
                let mut block_nodes = Vec::new();
                for block_data in block_values {
                    match scan_node(block_data.0, block_data.1.map_err(|_| ScannerError::UnknownError)?).await {
                        Ok(node) => if node.is_some() { block_nodes.push(node.unwrap()) },
                        Err(e) => return Err(e),
                    }
                }
                Ok(block_nodes)
            });
        }

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

