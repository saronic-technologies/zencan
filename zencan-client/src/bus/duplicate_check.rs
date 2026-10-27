use std::{num::NonZeroU32, time::Duration};

use futures::future::join_all;

use crate::sdo_client::SdoClientBuilder;

const DEFAULT_SETTLE_TIME: Duration = Duration::from_millis(200);

// Worst-case bus time for two nodes on the same ID to both deliver an 8 byte SDO response, when
// their responses start on the same bit and collide. With no other traffic:
//
// - A full frame is at most 135 bits: 98 stuffable bits + 24 stuff bits, 10 bits of CRC
//   delimiter/ACK/EOF, and 3 bits of intermission.
// - A failed attempt is at most 126 bits: the frame up to the last data bit (83 bits + 20 stuff
//   bits), up to 12 bits of superimposed error flags, an 8 bit error delimiter, and intermission.
// - The losing node becomes error passive (TEC >= 128) after at most 16 failed attempts, after
//   which its error flags no longer destroy the winner's frame.
// - The loser then waits 8 bits (suspend transmission) before sending its own frame.

/// Bit times from the request until the first response is received: 16 failed attempts + 1 frame
const FIRST_RESPONSE_BITS: u64 = 16 * 126 + 135;
/// Bit times between the first and second responses: suspend transmission + 1 frame
const SECOND_RESPONSE_BITS: u64 = 8 + 135;
/// Margin added to the bus time for device response latency, the spread in latency between nodes,
/// and higher priority traffic
const BITRATE_TIMING_MARGIN: Duration = Duration::from_millis(50);

/// The serial numbers that responded on a single node ID
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSerials {
    /// The node's CAN ID (1-127)
    pub node_id: u8,
    /// The distinct serial numbers (0x1018sub4) that responded on this node ID, sorted
    pub serials: Vec<u32>,
}

impl NodeSerials {
    /// True if more than one node is using this node ID
    pub fn is_duplicate(&self) -> bool {
        self.serials.len() > 1
    }
}

/// Finds nodes sharing a node ID by reading every serial number that answers on each ID
///
/// LSS can't always find these: fast scan ignores configured nodes, and not every device supports
/// LSS Identify Remote Slave. Instead, this sends one SDO read of 0x1018sub4 per node ID and
/// collects every response. See [`SdoClient::read_serial_numbers`](crate::SdoClient::read_serial_numbers).
///
/// Nothing else should be making SDO requests to the checked nodes while a check runs. Increase
/// `settle_time` on slow or busy buses; the default of 200 ms is comfortable at 50 kbit/s.
pub struct DuplicateChecker {
    // As with the BusScanner, a builder lets each node get its own client (and socket), which is
    // only open for the duration of the check
    sdo_client_builder: Box<dyn SdoClientBuilder>,
    settle_time: Duration,
    // Overrides the built clients' request timeout when set
    response_timeout: Option<Duration>,
}

impl DuplicateChecker {
    /// Create a new duplicate checker
    ///
    /// `settle_time` is how long to keep waiting for further responses after the last one
    /// received. Defaults to 200 ms. The time to wait for the first response is the request
    /// timeout of the clients built by `sdo_client_builder`.
    pub fn new(sdo_client_builder: Box<dyn SdoClientBuilder>, settle_time: Option<Duration>) -> Self {
        Self {
            sdo_client_builder,
            settle_time: settle_time.unwrap_or(DEFAULT_SETTLE_TIME),
            response_timeout: None,
        }
    }

    /// Create a duplicate checker with timeouts derived from the bus bitrate (bits/s)
    ///
    /// Both timeouts are the worst-case bus time for two colliding responses to resolve, plus a
    /// 50 ms margin for device response latency and other traffic:
    ///
    /// - Response timeout (request to first response): 2151 bit times + 50 ms
    /// - Settle time (between responses): 143 bit times + 50 ms
    ///
    /// | Bitrate    | Response timeout | Settle time |
    /// |------------|------------------|-------------|
    /// | 10 kbit/s  | 265.1 ms         | 64.3 ms     |
    /// | 20 kbit/s  | 157.6 ms         | 57.2 ms     |
    /// | 50 kbit/s  | 93.0 ms          | 52.9 ms     |
    /// | 125 kbit/s | 67.2 ms          | 51.1 ms     |
    /// | 250 kbit/s | 58.6 ms          | 50.6 ms     |
    /// | 500 kbit/s | 54.3 ms          | 50.3 ms     |
    /// | 1 Mbit/s   | 52.2 ms          | 50.1 ms     |
    ///
    /// This overrides the request timeout of the clients built by `sdo_client_builder`. Use
    /// [`Self::new`] instead if your devices are slower to respond than the margin allows.
    pub fn for_bitrate(sdo_client_builder: Box<dyn SdoClientBuilder>, bitrate: NonZeroU32) -> Self {
        let bit_times = |bits: u64| Duration::from_nanos(bits * 1_000_000_000 / bitrate.get() as u64);
        Self {
            sdo_client_builder,
            settle_time: bit_times(SECOND_RESPONSE_BITS) + BITRATE_TIMING_MARGIN,
            response_timeout: Some(bit_times(FIRST_RESPONSE_BITS) + BITRATE_TIMING_MARGIN),
        }
    }

    /// Check every valid node ID (1-127)
    pub async fn full_check(&mut self) -> anyhow::Result<Vec<NodeSerials>> {
        let full_range: Vec<u8> = (1u8..128).collect();
        self.check(&full_range).await
    }

    /// Check the given node IDs
    ///
    /// Returns an entry for each node ID that responded; use [`NodeSerials::is_duplicate`] to find
    /// the shared ones.
    pub async fn check(&mut self, node_ids: &[u8]) -> anyhow::Result<Vec<NodeSerials>> {
        const N_PARALLEL: usize = 10;

        let settle_time = self.settle_time;
        let response_timeout = self.response_timeout;
        let mut futures = Vec::new();

        for chunk in node_ids.chunks(node_ids.len().div_ceil(N_PARALLEL).max(1)) {
            let clients: Vec<_> = chunk
                .iter()
                .map(|node_id| {
                    let client = self.sdo_client_builder.with_node_id(*node_id).build_box();
                    let client = match response_timeout {
                        Some(timeout) => client.map(|c| c.with_request_timeout(timeout)),
                        None => client,
                    };
                    (*node_id, client)
                })
                .collect();
            futures.push(async move {
                let mut chunk_nodes = Vec::new();
                for (node_id, client) in clients {
                    let serials = client?
                        .read_serial_numbers(settle_time)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed reading serial numbers from node {node_id}: {e}"))?;
                    if !serials.is_empty() {
                        chunk_nodes.push(NodeSerials { node_id, serials });
                    }
                }
                anyhow::Ok(chunk_nodes)
            });
        }

        let mut nodes = Vec::new();
        for result in join_all(futures).await {
            nodes.extend(result?);
        }
        nodes.sort_by_key(|n| n.node_id);
        Ok(nodes)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use async_trait::async_trait;
    use tokio::sync::{broadcast, Mutex};
    use zencan_common::{
        messages::{CanId, CanMessage},
        sdo::{SdoRequest, SdoResponse},
        traits::{AsyncCanReceiver, AsyncCanSender},
    };

    use super::*;
    use crate::SdoClient;

    /// What the simulated nodes on one node ID send back to a request
    #[derive(Clone)]
    struct Reply {
        delay: Duration,
        msg: CanMessage,
    }

    /// A simulated bus: requests to 0x600+id trigger the configured replies on 0x580+id
    #[derive(Clone)]
    struct MockBus {
        tx: broadcast::Sender<CanMessage>,
        replies: Arc<HashMap<u8, Vec<Reply>>>,
    }

    impl MockBus {
        fn new(replies: HashMap<u8, Vec<Reply>>) -> Self {
            let (tx, _) = broadcast::channel(64);
            Self { tx, replies: Arc::new(replies) }
        }
    }

    struct MockSender(MockBus);

    #[async_trait]
    impl AsyncCanSender for MockSender {
        async fn send(&self, msg: CanMessage) -> anyhow::Result<()> {
            let CanId::Std(id) = msg.id else { return Ok(()) };
            if !(0x600..0x680).contains(&id) || SdoRequest::try_from(msg.data()).is_err() {
                return Ok(());
            }
            let node_id = (id - 0x600) as u8;
            for reply in self.0.replies.get(&node_id).cloned().unwrap_or_default() {
                let tx = self.0.tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(reply.delay).await;
                    tx.send(reply.msg).ok();
                });
            }
            Ok(())
        }
    }

    struct MockReceiver(Mutex<broadcast::Receiver<CanMessage>>);

    #[async_trait]
    impl AsyncCanReceiver for MockReceiver {
        async fn recv(&self) -> anyhow::Result<CanMessage> {
            Ok(self.0.lock().await.recv().await?)
        }
    }

    struct MockBuilder {
        bus: MockBus,
        node_id: u8,
        // Frames queued in the receiver before the client sends anything
        stale: Vec<CanMessage>,
    }

    impl SdoClientBuilder for MockBuilder {
        fn with_node_id(&self, node_id: u8) -> Box<dyn SdoClientBuilder> {
            Box::new(MockBuilder { bus: self.bus.clone(), node_id, stale: self.stale.clone() })
        }
        fn build_box(self: Box<Self>) -> anyhow::Result<SdoClient> {
            (*self).build()
        }
        fn set_node_id(mut self, node_id: u8) -> Self {
            self.node_id = node_id;
            self
        }
        fn build(self) -> anyhow::Result<SdoClient> {
            let receiver = MockReceiver(Mutex::new(self.bus.tx.subscribe()));
            for msg in self.stale {
                self.bus.tx.send(msg)?;
            }
            Ok(SdoClient::new_std(
                self.node_id,
                Box::new(MockSender(self.bus)),
                Box::new(receiver),
                None,
            ))
        }
    }

    fn serial_reply(node_id: u8, sub: u8, serial: u32, delay_ms: u64) -> Reply {
        Reply {
            delay: Duration::from_millis(delay_ms),
            msg: SdoResponse::expedited_upload(0x1018, sub, &serial.to_le_bytes())
                .to_can_message(CanId::Std(0x580 + node_id as u16)),
        }
    }

    fn checker(replies: HashMap<u8, Vec<Reply>>, stale: Vec<CanMessage>) -> DuplicateChecker {
        let builder = MockBuilder { bus: MockBus::new(replies), node_id: 0, stale };
        DuplicateChecker::new(Box::new(builder), Some(Duration::from_millis(100)))
    }

    #[tokio::test]
    async fn no_nodes() {
        let result = checker(HashMap::new(), vec![]).check(&[1, 2, 3]).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn single_node() {
        let replies = HashMap::from([(85, vec![serial_reply(85, 4, 0x0017F1A5, 0)])]);
        let result = checker(replies, vec![]).check(&[84, 85, 86]).await.unwrap();
        assert_eq!(result, vec![NodeSerials { node_id: 85, serials: vec![0x0017F1A5] }]);
        assert!(!result[0].is_duplicate());
    }

    #[tokio::test]
    async fn duplicate_node() {
        let replies = HashMap::from([(
            85,
            vec![serial_reply(85, 4, 0x0017F1A5, 0), serial_reply(85, 4, 0x00001000, 50)],
        )]);
        let result = checker(replies, vec![]).check(&[85]).await.unwrap();
        assert_eq!(result, vec![NodeSerials { node_id: 85, serials: vec![0x00001000, 0x0017F1A5] }]);
        assert!(result[0].is_duplicate());
    }

    #[tokio::test]
    async fn identical_responses_count_once() {
        let replies = HashMap::from([(
            85,
            vec![serial_reply(85, 4, 0x0017F1A5, 0), serial_reply(85, 4, 0x0017F1A5, 20)],
        )]);
        let result = checker(replies, vec![]).check(&[85]).await.unwrap();
        assert_eq!(result, vec![NodeSerials { node_id: 85, serials: vec![0x0017F1A5] }]);
    }

    #[tokio::test]
    async fn stale_frames_are_drained() {
        let replies = HashMap::from([(85, vec![serial_reply(85, 4, 0x0017F1A5, 0)])]);
        let stale = vec![serial_reply(85, 4, 0xDEADBEEF, 0).msg];
        let result = checker(replies, stale).check(&[85]).await.unwrap();
        assert_eq!(result, vec![NodeSerials { node_id: 85, serials: vec![0x0017F1A5] }]);
    }

    fn bitrate_checker(replies: HashMap<u8, Vec<Reply>>, bitrate: u32) -> DuplicateChecker {
        let builder = MockBuilder { bus: MockBus::new(replies), node_id: 0, stale: vec![] };
        DuplicateChecker::for_bitrate(Box::new(builder), NonZeroU32::new(bitrate).unwrap())
    }

    #[test]
    fn bitrate_timings() {
        let checker = bitrate_checker(HashMap::new(), 50_000);
        assert_eq!(checker.response_timeout, Some(Duration::from_micros(93_020)));
        assert_eq!(checker.settle_time, Duration::from_micros(52_860));

        let checker = bitrate_checker(HashMap::new(), 10_000);
        assert_eq!(checker.response_timeout, Some(Duration::from_micros(265_100)));
        assert_eq!(checker.settle_time, Duration::from_micros(64_300));
    }

    #[tokio::test]
    async fn bitrate_response_timeout_is_applied() {
        // 150 ms is beyond the client's default 100 ms timeout, but within 10 kbit/s' 265 ms
        let replies = HashMap::from([(85, vec![serial_reply(85, 4, 0x0017F1A5, 150)])]);
        let result = bitrate_checker(replies, 10_000).check(&[85]).await.unwrap();
        assert_eq!(result, vec![NodeSerials { node_id: 85, serials: vec![0x0017F1A5] }]);

        // ...and beyond 1 Mbit/s' 52 ms
        let replies = HashMap::from([(85, vec![serial_reply(85, 4, 0x0017F1A5, 150)])]);
        let result = bitrate_checker(replies, 1_000_000).check(&[85]).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn unrelated_responses_are_ignored() {
        let replies = HashMap::from([(
            85,
            vec![serial_reply(85, 4, 0x0017F1A5, 0), serial_reply(85, 1, 0x29B, 20)],
        )]);
        let result = checker(replies, vec![]).check(&[85]).await.unwrap();
        assert_eq!(result, vec![NodeSerials { node_id: 85, serials: vec![0x0017F1A5] }]);
    }
}
