use std::thread::sleep;
use std::time::{Duration, Instant};

use paste::paste;
use zencan_common::{
    can::{CanId, CanMessage, CanReceiver, CanSender},
    i24,
    node_configuration::PdoConfig,
    object_model::{
        object_ids, values::SAVE_CMD, PdoCommParameter, PdoMapping, ReadSize, TimeDifference,
        TimeOfDay,
    },
    protocol::{AbortCode, BlockSegment, LssIdentity, SdoRequest, SdoResponse},
    u24,
};

use crate::sdo_client::{
    BlockSizeChangedTooSmallSnafu, MalformedResponseSnafu, MismatchedObjectIndexSnafu,
    NoResponseSnafu, Result, SdoClientError, ServerAbortSnafu, SocketSendFailedSnafu,
    ToggleNotAlternatedSnafu, UnexpectedResponseSnafu, UnexpectedSizeSnafu,
};

const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_millis(150);

macro_rules! access_methods {
    ($type: ty) => {
        paste! {
            #[doc = concat!("Read a ", stringify!($type), " sub object from the SDO server")]
            pub fn [<read_ $type>](&mut self, index: u16, sub: u8) -> Result<$type> {
                let data = self.upload(index, sub)?;
                if data.len() != <$type as ReadSize>::READ_SIZE {
                    return UnexpectedSizeSnafu.fail();
                }
                Ok($type::from_le_bytes(data.try_into().unwrap()))
            }

            #[doc = concat!("Write a ", stringify!($type), " sub object to the SDO server")]
            pub fn [<write_ $type>](&mut self, index: u16, sub: u8, value: $type) -> Result<()> {
                let data = value.to_le_bytes();
                self.download(index, sub, &data)
            }
        }
    };
}

/// A blocking client for accessing a node's SDO server
///
/// The blocking counterpart of [`crate::SdoClient`].
///
/// A single server can talk to a single client at a time.
#[derive(Debug)]
pub struct SdoClient<S, R> {
    req_cob_id: CanId,
    resp_cob_id: CanId,
    timeout: Duration,
    sender: S,
    receiver: R,
}

impl<S: CanSender, R: CanReceiver> SdoClient<S, R> {
    /// Create a new SdoClient using a node ID
    ///
    /// Nodes have a default SDO server, which uses a COB ID based on the node ID. This is a
    /// shortcut to create a client for that default SDO server.
    ///
    /// It is possible for nodes to have other SDO servers on other COB IDs, and clients for these
    /// can be created using [`Self::new()`]
    pub fn new_std(server_node_id: u8, sender: S, receiver: R) -> Self {
        let req_cob_id = CanId::Std(0x600 + server_node_id as u16);
        let resp_cob_id = CanId::Std(0x580 + server_node_id as u16);
        Self::new(req_cob_id, resp_cob_id, sender, receiver)
    }

    /// Create a new SdoClient from request and response COB IDs
    pub fn new(req_cob_id: CanId, resp_cob_id: CanId, sender: S, receiver: R) -> Self {
        Self {
            req_cob_id,
            resp_cob_id,
            timeout: DEFAULT_RESPONSE_TIMEOUT,
            sender,
            receiver,
        }
    }

    /// Set the timeout for waiting on SDO server responses
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Get the current timeout for waiting on SDO server responses
    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }

    fn send(&mut self, data: [u8; 8]) -> Result<()> {
        let frame = CanMessage::new(self.req_cob_id, &data);
        let mut tries = 3;
        loop {
            match self.sender.send(frame) {
                Ok(()) => return Ok(()),
                // The blocking sender hands the message back rather than an error
                Err(_) => {
                    tries -= 1;
                    sleep(Duration::from_millis(5));
                    if tries == 0 {
                        return SocketSendFailedSnafu {
                            message: "sender rejected the message",
                        }
                        .fail();
                    }
                }
            }
        }
    }

    /// Write data to a sub-object on the SDO server
    pub fn download(&mut self, index: u16, sub: u8, data: &[u8]) -> Result<()> {
        if data.len() <= 4 {
            // Do an expedited transfer
            self.send(SdoRequest::expedited_download(index, sub, data).to_bytes())?;

            let resp = self.wait_for_response()?;
            match_response!(
                resp,
                "ConfirmDownload",
                SdoResponse::ConfirmDownload { index: _, sub: _ } => {
                    Ok(()) // Success!
                }
            )
        } else {
            self.send(
                SdoRequest::initiate_download(index, sub, Some(data.len() as u32)).to_bytes(),
            )?;

            let resp = self.wait_for_response()?;
            match_response!(
                resp,
                "ConfirmDownload",
                SdoResponse::ConfirmDownload { index: _, sub: _ } => { }
            );

            let mut toggle = false;
            let total_segments = data.len().div_ceil(7);
            for n in 0..total_segments {
                let last_segment = n == total_segments - 1;
                let segment_size = (data.len() - n * 7).min(7);
                let seg_msg = SdoRequest::download_segment(
                    toggle,
                    last_segment,
                    &data[n * 7..n * 7 + segment_size],
                );
                self.send(seg_msg.to_bytes())?;

                let resp = self.wait_for_response()?;
                match_response!(
                    resp,
                    "ConfirmDownloadSegment",
                    SdoResponse::ConfirmDownloadSegment { t } => {
                        if t != toggle {
                            let abort_msg =
                                SdoRequest::abort(index, sub, AbortCode::ToggleNotAlternated);
                            self.send(abort_msg.to_bytes())?;
                            return ToggleNotAlternatedSnafu.fail();
                        }
                    }
                );
                toggle = !toggle;
            }
            Ok(())
        }
    }

    /// Read a sub-object on the SDO server
    pub fn upload(&mut self, index: u16, sub: u8) -> Result<Vec<u8>> {
        let mut read_buf = Vec::new();

        self.send(SdoRequest::initiate_upload(index, sub).to_bytes())?;

        let resp = self.wait_for_response()?;

        let expedited = match_response!(
            resp,
            "ConfirmUpload",
            SdoResponse::ConfirmUpload {
                n,
                e,
                s,
                index: _,
                sub: _,
                data,
            } => {
                if e {
                    let mut len = 0;
                    if s {
                        len = 4 - n as usize;
                    }
                    read_buf.extend_from_slice(&data[0..len]);
                }
                e
            }
        );

        if !expedited {
            let mut toggle = false;
            loop {
                self.send(SdoRequest::upload_segment_request(toggle).to_bytes())?;

                let resp = self.wait_for_response()?;
                match_response!(
                    resp,
                    "UploadSegment",
                    SdoResponse::UploadSegment { t, n, c, data } => {
                        if t != toggle {
                            self.send(
                                SdoRequest::abort(index, sub, AbortCode::ToggleNotAlternated)
                                    .to_bytes(),
                            )?;
                            return ToggleNotAlternatedSnafu.fail();
                        }
                        read_buf.extend_from_slice(&data[0..7 - n as usize]);
                        if c {
                            break;
                        }
                    }
                );
                toggle = !toggle;
            }
        }
        Ok(read_buf)
    }

    /// Perform a block download to transfer data to an object
    ///
    /// Block downloads are more efficient for large amounts of data, but may not be supported by
    /// all devices.
    pub fn block_download(&mut self, index: u16, sub: u8, data: &[u8]) -> Result<()> {
        self.send(
            SdoRequest::InitiateBlockDownload {
                cc: true, // CRC supported
                s: true,  // size specified
                index,
                sub,
                size: data.len() as u32,
            }
            .to_bytes(),
        )?;

        let resp = self.wait_for_response()?;

        let (crc_enabled, mut blksize) = match_response!(
            resp,
            "ConfirmBlockDownload",
            SdoResponse::ConfirmBlockDownload {
                sc,
                index: resp_index,
                sub: resp_sub,
                blksize,
            } => {
                if index != resp_index || sub != resp_sub {
                    return MismatchedObjectIndexSnafu {
                        expected: (index, sub),
                        received: (resp_index, resp_sub),
                    }
                    .fail();
                }
                (sc, blksize)
            }
        );

        let mut seqnum = 1;
        let mut last_block_start = 0;
        let mut segment_num = 0;
        let total_segments = data.len().div_ceil(7);

        while segment_num < total_segments {
            let segment_start = segment_num * 7;
            let segment_len = (data.len() - segment_start).min(7);
            // Is this the last segment?
            let c = segment_start + segment_len == data.len();
            let mut segment_data = [0; 7];
            segment_data[0..segment_len]
                .copy_from_slice(&data[segment_start..segment_start + segment_len]);

            let segment = BlockSegment {
                c,
                seqnum,
                data: segment_data,
            };
            self.send(segment.to_bytes())?;

            // Expect a confirmation message after blksize segments are sent, or after sending the
            // complete flag
            if c || seqnum == blksize {
                let resp = self.wait_for_response()?;
                match_response!(
                    resp,
                    "ConfirmBlock",
                    SdoResponse::ConfirmBlock {
                        ackseq,
                        blksize: new_blksize,
                    } => {
                        if ackseq == blksize {
                            // All segments are acknowledged. Block accepted
                            seqnum = 1;
                            segment_num += 1;
                            last_block_start = segment_num;
                        } else {
                            // Missing segments. Resend all segments after ackseq
                            seqnum = ackseq;
                            segment_num = last_block_start + ackseq as usize;
                            // A server may shrink the block size between blocks, which could in
                            // principle drop below what has already been delivered.
                            if new_blksize < seqnum {
                                return BlockSizeChangedTooSmallSnafu.fail();
                            }
                        }
                        blksize = new_blksize;
                    }
                );
            } else {
                seqnum += 1;
                segment_num += 1;
            }
        }

        let crc = if crc_enabled {
            crc16::State::<crc16::XMODEM>::calculate(data)
        } else {
            0
        };

        let n = ((7 - data.len() % 7) % 7) as u8;

        self.send(SdoRequest::EndBlockDownload { n, crc }.to_bytes())?;

        let resp = self.wait_for_response()?;
        match_response!(
            resp,
            "ConfirmBlockDownloadEnd",
            SdoResponse::ConfirmBlockDownloadEnd => { Ok(()) }
        )
    }

    /// Perform a block upload of data from the node
    pub fn block_upload(&mut self, index: u16, sub: u8) -> Result<Vec<u8>> {
        const CRC_SUPPORTED: bool = true;
        const BLKSIZE: u8 = 127;
        const PST: u8 = 0;

        self.send(
            SdoRequest::initiate_block_upload(index, sub, CRC_SUPPORTED, BLKSIZE, PST).to_bytes(),
        )?;

        let resp = self.wait_for_response()?;

        let server_supports_crc = match_response!(
            resp,
            "ConfirmBlockUpload",
            SdoResponse::ConfirmBlockUpload { sc, s: _, index: _, sub: _, size: _ } => {sc}
        );

        self.send(SdoRequest::StartBlockUpload.to_bytes())?;

        let mut rx_data = Vec::new();
        let last_segment;
        loop {
            let segment = self.wait_for_block_segment()?;
            rx_data.extend_from_slice(&segment.data);
            if !segment.c && segment.seqnum == BLKSIZE {
                // Finished sub block, but not yet done. Confirm this sub block and expect more
                self.send(
                    SdoRequest::ConfirmBlock {
                        ackseq: BLKSIZE,
                        blksize: BLKSIZE,
                    }
                    .to_bytes(),
                )?;
            }
            if segment.c {
                last_segment = segment.seqnum;
                break;
            }
        }

        // NOTE: Ignoring the possibility of dropped messages here. Should check seqno to make sure
        // all blocks are received.
        self.send(
            SdoRequest::ConfirmBlock {
                ackseq: last_segment,
                blksize: BLKSIZE,
            }
            .to_bytes(),
        )?;

        let resp = self.wait_for_response()?;
        let (n, crc) = match_response!(
            resp,
            "BlockUploadEnd",
            SdoResponse::BlockUploadEnd { n, crc } => {(n, crc)}
        );

        // Drop the n invalid data bytes
        rx_data.resize(rx_data.len() - n as usize, 0);

        if server_supports_crc {
            let computed_crc = crc16::State::<crc16::XMODEM>::calculate(&rx_data);
            if crc != computed_crc {
                self.send(SdoRequest::abort(index, sub, AbortCode::CrcError).to_bytes())?;
                return Err(SdoClientError::CrcMismatch);
            }
        }

        self.send(SdoRequest::EndBlockUpload.to_bytes())?;

        Ok(rx_data)
    }

    access_methods!(f64);
    access_methods!(f32);
    access_methods!(u64);
    access_methods!(u32);
    access_methods!(u24);
    access_methods!(u16);
    access_methods!(u8);
    access_methods!(i64);
    access_methods!(i32);
    access_methods!(i24);
    access_methods!(i16);
    access_methods!(i8);

    /// Write to a TimeOfDay object on the SDO server
    pub fn write_time_of_day(&mut self, index: u16, sub: u8, data: TimeOfDay) -> Result<()> {
        let data = data.to_le_bytes();
        self.download(index, sub, &data)
    }

    /// Write to a TimeDifference object on the SDO server
    pub fn write_time_difference(
        &mut self,
        index: u16,
        sub: u8,
        data: TimeDifference,
    ) -> Result<()> {
        let data = data.to_le_bytes();
        self.download(index, sub, &data)
    }

    /// Read a string from the SDO server
    pub fn read_utf8(&mut self, index: u16, sub: u8) -> Result<String> {
        let data = self.upload(index, sub)?;
        Ok(String::from_utf8_lossy(&data).into())
    }

    /// Read a TimeOfDay object from the SDO server
    pub fn read_time_of_day(&mut self, index: u16, sub: u8) -> Result<TimeOfDay> {
        let data = self.upload(index, sub)?;
        if data.len() != TimeOfDay::SIZE {
            UnexpectedSizeSnafu.fail()
        } else {
            Ok(TimeOfDay::from_le_bytes(data.try_into().unwrap()))
        }
    }

    /// Read a TimeDifference object from the SDO server
    pub fn read_time_difference(&mut self, index: u16, sub: u8) -> Result<TimeDifference> {
        let data = self.upload(index, sub)?;
        if data.len() != TimeDifference::SIZE {
            UnexpectedSizeSnafu.fail()
        } else {
            Ok(TimeDifference::from_le_bytes(data.try_into().unwrap()))
        }
    }

    /// Read an object as a visible string
    ///
    /// It will be read and assumed to contain valid UTF8 characters
    pub fn read_visible_string(&mut self, index: u16, sub: u8) -> Result<String> {
        let bytes = self.upload(index, sub)?;
        Ok(String::from_utf8_lossy(&bytes).into())
    }

    /// Read an object as a boolean
    pub fn read_bool(&mut self, index: u16, sub: u8) -> Result<bool> {
        let bytes = self.upload(index, sub)?;
        if bytes.len() != 1 {
            return UnexpectedSizeSnafu.fail();
        }
        Ok(bytes[0] != 0)
    }

    /// Write an object as a boolean
    pub fn write_bool(&mut self, index: u16, sub: u8, value: bool) -> Result<()> {
        let data = if value { [1u8] } else { [0u8] };
        self.download(index, sub, &data)
    }

    /// Read the identity object
    ///
    /// All nodes should implement this object
    pub fn read_identity(&mut self) -> Result<LssIdentity> {
        let vendor_id = self.read_u32(object_ids::IDENTITY, 1)?;
        let product_code = self.read_u32(object_ids::IDENTITY, 2)?;
        let revision_number = self.read_u32(object_ids::IDENTITY, 3)?;
        let serial = self.read_u32(object_ids::IDENTITY, 4)?;
        Ok(LssIdentity::new(
            vendor_id,
            product_code,
            revision_number,
            serial,
        ))
    }

    /// Write object 0x1010sub1 to command all objects be saved
    pub fn save_objects(&mut self) -> Result<()> {
        self.write_u32(object_ids::SAVE_OBJECTS, 1, SAVE_CMD)
    }

    /// Read the device name object
    ///
    /// All nodes should implement this object
    pub fn read_device_name(&mut self) -> Result<String> {
        self.read_visible_string(object_ids::DEVICE_NAME, 0)
    }

    /// Read the software version object
    ///
    /// All nodes should implement this object
    pub fn read_software_version(&mut self) -> Result<String> {
        self.read_visible_string(object_ids::SOFTWARE_VERSION, 0)
    }

    /// Read the hardware version object
    ///
    /// All nodes should implement this object
    pub fn read_hardware_version(&mut self) -> Result<String> {
        self.read_visible_string(object_ids::HARDWARE_VERSION, 0)
    }

    /// Configure a transmit PDO on the device
    ///
    /// This is a convenience function to write the PDO comm and mapping objects based on a
    /// [`PdoConfig`].
    pub fn configure_tpdo(&mut self, pdo_num: usize, cfg: &PdoConfig) -> Result<()> {
        let comm_index = 0x1800 + pdo_num as u16;
        let mapping_index = 0x1a00 + pdo_num as u16;
        self.store_pdo_config(comm_index, mapping_index, cfg)
    }

    /// Configure a receive PDO on the device
    ///
    /// This is a convenience function to write the PDO comm and mapping objects based on a
    /// [`PdoConfig`].
    pub fn configure_rpdo(&mut self, pdo_num: usize, cfg: &PdoConfig) -> Result<()> {
        let comm_index = 0x1400 + pdo_num as u16;
        let mapping_index = 0x1600 + pdo_num as u16;
        self.store_pdo_config(comm_index, mapping_index, cfg)
    }

    /// Set the COB_ID config for an RPDO
    ///
    /// Can be used to enable/disable, or change COB ID for a PDO without changing other settings
    pub fn set_rpdo_cob_id(
        &mut self,
        pdo_num: usize,
        cob_id: CanId,
        valid: bool,
        rtr_disabled: bool,
    ) -> Result<()> {
        let comm_index = 0x1400 + pdo_num as u16;
        self.set_pdo_cob_id(comm_index, cob_id, valid, rtr_disabled)
    }

    /// Set the COB_ID config for a TPDO
    ///
    /// Can be used to enable/disable, or change COB ID for a PDO without changing other settings
    pub fn set_tpdo_cob_id(
        &mut self,
        pdo_num: usize,
        cob_id: CanId,
        valid: bool,
        rtr_disabled: bool,
    ) -> Result<()> {
        let comm_index = 0x1800 + pdo_num as u16;
        self.set_pdo_cob_id(comm_index, cob_id, valid, rtr_disabled)
    }

    fn set_pdo_cob_id(
        &mut self,
        comm_index: u16,
        cob_id: CanId,
        valid: bool,
        rtr_disabled: bool,
    ) -> Result<()> {
        let mut cob_value = cob_id.raw() & 0x1FFFFFFF;
        if !valid {
            cob_value |= 1 << 31;
        }
        if cob_id.is_extended() {
            cob_value |= 1 << 29;
        }
        if rtr_disabled {
            cob_value |= 1 << 30;
        }
        self.write_u32(comm_index, 1, cob_value)?;

        Ok(())
    }

    /// Write to a PDO Comm parameter
    fn set_pdo_comm_parameter(&mut self, comm_index: u16, comm: PdoCommParameter) -> Result<()> {
        self.write_u8(comm_index, 2, comm.transmission_type)?;
        self.set_pdo_cob_id(comm_index, comm.cob_id, comm.valid, comm.rtr_disabled)?;
        Ok(())
    }

    fn store_pdo_config(
        &mut self,
        comm_index: u16,
        mapping_index: u16,
        cfg: &PdoConfig,
    ) -> Result<()> {
        let disabled_comm = PdoCommParameter {
            valid: false,
            ..cfg.comm
        };

        // Ensure PDO is disabled
        self.set_pdo_comm_parameter(comm_index, disabled_comm)?;

        // Set the number of valid mappings to 0
        self.write_u8(mapping_index, 0, 0)?;

        // Write the mappings
        assert!(cfg.mappings.len() < 0x40);
        for (i, m) in cfg.mappings.iter().enumerate() {
            let mapping_value = m.to_object_value();
            self.write_u32(mapping_index, (i + 1) as u8, mapping_value)?;
        }

        // Set the number of valid mappings to the number configured
        let num_mappings = cfg.mappings.len() as u8;
        self.write_u8(mapping_index, 0, num_mappings)?;

        // Make PDO valid, if requested
        if cfg.comm.valid {
            self.set_pdo_comm_parameter(comm_index, cfg.comm)?;
        }
        Ok(())
    }

    /// Read the configuration of an RPDO from the node
    pub fn read_rpdo_config(&mut self, pdo_num: usize) -> Result<PdoConfig> {
        let comm_index = 0x1400 + pdo_num as u16;
        let mapping_index = 0x1600 + pdo_num as u16;
        self.read_pdo_config(comm_index, mapping_index)
    }

    /// Read the configuration of a TPDO from the node
    pub fn read_tpdo_config(&mut self, pdo_num: usize) -> Result<PdoConfig> {
        let comm_index = 0x1800 + pdo_num as u16;
        let mapping_index = 0x1a00 + pdo_num as u16;
        self.read_pdo_config(comm_index, mapping_index)
    }

    fn read_pdo_config(&mut self, comm_index: u16, mapping_index: u16) -> Result<PdoConfig> {
        let cob_word = self.read_u32(comm_index, 1)?;
        let transmission_type = self.read_u8(comm_index, 2)?;
        let num_mappings = self.read_u8(mapping_index, 0)?;
        let mut mappings = Vec::with_capacity(num_mappings as usize);
        for i in 0..num_mappings {
            let mapping_raw = self.read_u32(mapping_index, i + 1)?;
            mappings.push(PdoMapping::from_object_value(mapping_raw));
        }
        let valid = cob_word & (1 << 31) == 0;
        let rtr_disabled = cob_word & (1 << 30) != 0;
        let extended = cob_word & (1 << 29) != 0;
        let cob_id = cob_word & 0x1FFFFFFF;
        let cob_id = if extended {
            CanId::extended(cob_id)
        } else {
            CanId::std(cob_id as u16)
        };
        Ok(PdoConfig {
            comm: PdoCommParameter {
                valid,
                rtr_disabled,
                cob_id,
                transmission_type,
            },
            mappings,
        })
    }

    fn wait_for_block_segment(&mut self) -> Result<BlockSegment> {
        let wait_until = Instant::now() + self.timeout;

        loop {
            let remaining = wait_until.saturating_duration_since(Instant::now());

            match self.receiver.recv(remaining) {
                Ok(msg) => {
                    if msg.id == self.resp_cob_id {
                        return msg
                            .data()
                            .try_into()
                            .map_err(|_| MalformedResponseSnafu.build());
                    }
                }
                Err(_) => {
                    if Instant::now() >= wait_until {
                        return NoResponseSnafu.fail();
                    }
                }
            }
        }
    }

    fn wait_for_response(&mut self) -> Result<SdoResponse> {
        let wait_until = Instant::now() + self.timeout;

        loop {
            let remaining = wait_until.saturating_duration_since(Instant::now());

            match self.receiver.recv(remaining) {
                Ok(msg) => {
                    if msg.id == self.resp_cob_id {
                        return msg.try_into().map_err(|_| MalformedResponseSnafu.build());
                    }
                    // Someone else's traffic; keep waiting out the timeout.
                }
                // The receiver cannot say whether it timed out or failed, so
                // let the clock decide whether there is still time to wait.
                Err(_) => {
                    if Instant::now() >= wait_until {
                        return NoResponseSnafu.fail();
                    }
                }
            }
        }
    }
}
