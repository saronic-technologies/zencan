use std::time::Duration;

use snafu::Snafu;
use zencan_common::{
    messages::CanId,
    sdo::{AbortCode, SdoRequest, SdoResponse},
    traits::{AsyncCanReceiver, AsyncCanSender},
};

const RESPONSE_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Debug, PartialEq, Snafu)]
pub enum SdoClientError {
    NoResponse,
    MalformedResponse,
    UnexpectedResponse,
    ServerAbort {
        index: u16,
        sub: u8,
        abort_code: u32,
    },
    ToggleNotAlternated,
    UnexpectedSize,
    /// An error occured reading from the socket
    SocketError,
}

type Result<T> = std::result::Result<T, SdoClientError>;

pub struct SdoClient<S, R> {
    req_cob_id: CanId,
    resp_cob_id: CanId,
    sender: S,
    receiver: R,
}

impl<S: AsyncCanSender, R: AsyncCanReceiver> SdoClient<S, R> {
    pub fn new_std(server_node_id: u8, sender: S, receiver: R) -> Self {
        Self {
            req_cob_id: CanId::Std(0x600 + server_node_id as u16),
            resp_cob_id: CanId::Std(0x580 + server_node_id as u16),
            sender,
            receiver,
        }
    }

    /// Write data to a sub-object on the SDO server
    pub async fn download(&mut self, index: u16, sub: u8, data: &[u8]) -> Result<()> {
        if data.len() <= 4 {
            // Do an expedited transfer
            let msg =
                SdoRequest::expedited_download(index, sub, data).to_can_message(self.req_cob_id);
            self.sender.send(msg).await.unwrap(); // TODO: Expect errors

            let resp = self.wait_for_response(RESPONSE_TIMEOUT).await?;
            match resp {
                SdoResponse::ConfirmDownload { index: _, sub: _ } => {
                    Ok(()) // Success!
                }
                SdoResponse::Abort {
                    index,
                    sub,
                    abort_code,
                } => ServerAbortSnafu {
                    index,
                    sub,
                    abort_code,
                }
                .fail(),
                _ => UnexpectedResponseSnafu.fail(),
            }
        } else {
            let msg = SdoRequest::initiate_download(index, sub, Some(data.len() as u32))
                .to_can_message(self.req_cob_id);
            self.sender.send(msg).await.unwrap();

            let resp = self.wait_for_response(RESPONSE_TIMEOUT).await?;
            match resp {
                SdoResponse::ConfirmDownload { index: _, sub: _ } => (),
                SdoResponse::Abort {
                    index,
                    sub,
                    abort_code,
                } => {
                    return ServerAbortSnafu {
                        index,
                        sub,
                        abort_code,
                    }
                    .fail();
                }
                _ => return UnexpectedResponseSnafu.fail(),
            }

            let mut toggle = false;
            // Send segments
            let total_segments = (data.len() + 6) / 7;
            for n in 0..total_segments {
                let last_segment = n == total_segments - 1;
                let segment_size = (data.len() - n * 7).min(7);
                let seg_msg = SdoRequest::download_segment(
                    toggle,
                    last_segment,
                    &data[n * 7..n * 7 + segment_size],
                )
                .to_can_message(self.req_cob_id);
                self.sender
                    .send(seg_msg)
                    .await
                    .expect("failed sending DL segment");
                let resp = self.wait_for_response(RESPONSE_TIMEOUT).await?;
                match resp {
                    SdoResponse::ConfirmDownloadSegment { t } => {
                        // Fail if toggle value doesn't match
                        if t != toggle {
                            let abort_msg =
                                SdoRequest::abort(index, sub, AbortCode::ToggleNotAlternated)
                                    .to_can_message(self.req_cob_id);
                            self.sender
                                .send(abort_msg)
                                .await
                                .expect("Error sending abort");
                            return ToggleNotAlternatedSnafu.fail();
                        }
                        // Otherwise, carry on
                    }
                    SdoResponse::Abort {
                        index,
                        sub,
                        abort_code,
                    } => {
                        return ServerAbortSnafu {
                            index,
                            sub,
                            abort_code,
                        }
                        .fail();
                    }
                    _ => {
                        // Any other message from the SDO server is unexpected
                        return UnexpectedResponseSnafu.fail();
                    }
                }
                toggle = !toggle;
            }
            Ok(())
        }
    }

    /// Read a sub-object on the SDO server
    pub async fn upload(&mut self, index: u16, sub: u8) -> Result<Vec<u8>> {
        let mut read_buf = Vec::new();

        let msg = SdoRequest::initiate_upload(index, sub).to_can_message(self.req_cob_id);
        self.sender.send(msg).await.unwrap();

        let resp = self.wait_for_response(RESPONSE_TIMEOUT).await?;

        let expedited = match resp {
            SdoResponse::ConfirmUpload {
                n,
                e,
                s: _,
                index: _,
                sub: _,
                data,
            } => {
                if e {
                    read_buf.extend_from_slice(&data[0..4 - n as usize]);
                }
                e
            }
            SdoResponse::Abort {
                index,
                sub,
                abort_code,
            } => {
                return ServerAbortSnafu {
                    index,
                    sub,
                    abort_code,
                }
                .fail();
            }
            _ => return UnexpectedResponseSnafu.fail(),
        };

        if !expedited {
            // Read segments
            let mut toggle = false;
            loop {
                let msg =
                    SdoRequest::upload_segment_request(toggle).to_can_message(self.req_cob_id);

                self.sender.send(msg).await.unwrap();

                let resp = self.wait_for_response(RESPONSE_TIMEOUT).await?;
                match resp {
                    SdoResponse::UploadSegment { t, n, c, data } => {
                        if t != toggle {
                            self.sender
                                .send(
                                    SdoRequest::abort(index, sub, AbortCode::ToggleNotAlternated)
                                        .to_can_message(self.req_cob_id),
                                )
                                .await
                                .expect("Error sending abort");
                            return ToggleNotAlternatedSnafu.fail();
                        }
                        read_buf.extend_from_slice(&data[0..7 - n as usize]);
                        if c {
                            // Transfer complete
                            break;
                        }
                    }
                    SdoResponse::Abort {
                        index,
                        sub,
                        abort_code,
                    } => {
                        return ServerAbortSnafu {
                            index,
                            sub,
                            abort_code,
                        }
                        .fail();
                    }
                    _ => return UnexpectedResponseSnafu.fail(),
                }
                toggle = !toggle;
            }
        }
        Ok(read_buf)
    }

    /// Write to a u32 object on the SDO server
    pub async fn download_u32(&mut self, index: u16, sub: u8, data: u32) -> Result<()> {
        let data = data.to_le_bytes();
        self.download(index, sub, &data).await
    }

    /// Alias for `download_u32`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn write_u32(&mut self, index: u16, sub: u8, data: u32) -> Result<()> {
        self.download_u32(index, sub, data).await
    }

    /// Write to a u16 object on the SDO server
    pub async fn download_u16(&mut self, index: u16, sub: u8, data: u16) -> Result<()> {
        let data = data.to_le_bytes();
        self.download(index, sub, &data).await
    }

    /// Alias for `download_u16`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn write_u16(&mut self, index: u16, sub: u8, data: u16) -> Result<()> {
        self.download_u16(index, sub, data).await
    }

    /// Write to a u16 object on the SDO server
    pub async fn download_u8(&mut self, index: u16, sub: u8, data: u8) -> Result<()> {
        let data = data.to_le_bytes();
        self.download(index, sub, &data).await
    }

    /// Alias for `download_u8`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn write_u8(&mut self, index: u16, sub: u8, data: u8) -> Result<()> {
        self.download_u8(index, sub, data).await
    }

    /// Write to an i32 object on the SDO server
    pub async fn download_i32(&mut self, index: u16, sub: u8, data: i32) -> Result<()> {
        let data = data.to_le_bytes();
        self.download(index, sub, &data).await
    }

    /// Alias for `download_i32`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn write_i32(&mut self, index: u16, sub: u8, data: i32) -> Result<()> {
        self.download_i32(index, sub, data).await
    }

    /// Write to an i16 object on the SDO server
    pub async fn download_i16(&mut self, index: u16, sub: u8, data: i16) -> Result<()> {
        let data = data.to_le_bytes();
        self.download(index, sub, &data).await
    }

    /// Alias for `download_i16`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn write_i16(&mut self, index: u16, sub: u8, data: i16) -> Result<()> {
        self.download_i16(index, sub, data).await
    }

    /// Write to an i8 object on the SDO server
    pub async fn download_i8(&mut self, index: u16, sub: u8, data: i8) -> Result<()> {
        let data = data.to_le_bytes();
        self.download(index, sub, &data).await
    }

    /// Alias for `download_i8`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn write_i8(&mut self, index: u16, sub: u8, data: i8) -> Result<()> {
        self.download_i8(index, sub, data).await
    }

    /// Read a string from the SDO server
    pub async fn upload_utf8(&mut self, index: u16, sub: u8) -> Result<String> {
        let data = self.upload(index, sub).await?;
        Ok(String::from_utf8_lossy(&data).into())
    }
    /// Alias for `upload_utf8`
    pub async fn read_utf8(&mut self, index: u16, sub: u8) -> Result<String> {
        self.upload_utf8(index, sub).await
    }

    /// Read a sub-object from the SDO server, assuming it is an u8
    pub async fn upload_u8(&mut self, index: u16, sub: u8) -> Result<u8> {
        let data = self.upload(index, sub).await?;
        if data.len() != 1 {
            return UnexpectedSizeSnafu.fail();
        }
        Ok(data[0])
    }
    /// Alias for `upload_u8`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn read_u8(&mut self, index: u16, sub: u8) -> Result<u8> {
        self.upload_u8(index, sub).await
    }

    // Read a sub-object from the SDO server, assuming it is an u16
    pub async fn upload_u16(&mut self, index: u16, sub: u8) -> Result<u16> {
        let data = self.upload(index, sub).await?;
        if data.len() != 2 {
            return UnexpectedSizeSnafu.fail();
        }
        Ok(u16::from_le_bytes(data.try_into().unwrap()))
    }

    /// Alias for `upload_u16`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn read_u16(&mut self, index: u16, sub: u8) -> Result<u16> {
        self.upload_u16(index, sub).await
    }

    /// Read a sub-object from the SDO server, assuming it is an u32
    pub async fn upload_u32(&mut self, index: u16, sub: u8) -> Result<u32> {
        let data = self.upload(index, sub).await?;
        if data.len() != 4 {
            return UnexpectedSizeSnafu.fail();
        }
        Ok(u32::from_le_bytes(data.try_into().unwrap()))
    }

    /// Alias for `upload_u32`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn read_u32(&mut self, index: u16, sub: u8) -> Result<u32> {
        self.upload_u32(index, sub).await
    }

    /// Read a sub-object from the SDO server, assuming it is an i8
    pub async fn upload_i8(&mut self, index: u16, sub: u8) -> Result<i8> {
        let data = self.upload(index, sub).await?;
        if data.len() != 1 {
            return UnexpectedSizeSnafu.fail();
        }
        Ok(i8::from_le_bytes(data.try_into().unwrap()))
    }

    /// Alias for `upload_i8`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn read_i8(&mut self, index: u16, sub: u8) -> Result<i8> {
        self.upload_i8(index, sub).await
    }

    /// Read a sub-object from the SDO server, assuming it is an i16
    pub async fn upload_i16(&mut self, index: u16, sub: u8) -> Result<i16> {
        let data = self.upload(index, sub).await?;
        if data.len() != 2 {
            return UnexpectedSizeSnafu.fail();
        }
        Ok(i16::from_le_bytes(data.try_into().unwrap()))
    }

    /// Alias for `upload_i16`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn read_i16(&mut self, index: u16, sub: u8) -> Result<i16> {
        self.upload_i16(index, sub).await
    }

    /// Read a sub-object from the SDO server, assuming it is an i32
    pub async fn upload_i32(&mut self, index: u16, sub: u8) -> Result<i32> {
        let data = self.upload(index, sub).await?;
        if data.len() != 4 {
            return UnexpectedSizeSnafu.fail();
        }
        Ok(i32::from_le_bytes(data.try_into().unwrap()))
    }

    /// Alias for `upload_i32`
    ///
    /// This is a convenience function to allow for a more intuitive API
    pub async fn read_i32(&mut self, index: u16, sub: u8) -> Result<i32> {
        self.upload_i32(index, sub).await
    }

    async fn wait_for_response(&mut self, timeout: Duration) -> Result<SdoResponse> {
        let wait_until = tokio::time::Instant::now() + timeout;
        loop {
            match tokio::time::timeout_at(wait_until, self.receiver.recv()).await {
                // Err indicates the timeout elapsed, so return
                Err(_) => return NoResponseSnafu.fail(),
                // Message was recieved. If it is the resp, return. Otherwise, keep waiting
                Ok(Ok(msg)) => {
                    if msg.id == self.resp_cob_id {
                        return msg.try_into().map_err(|_| MalformedResponseSnafu.build());
                    }
                }
                // Recv returned an error
                Ok(Err(e)) => {
                    log::error!("Error reading from socket: {e:?}");
                    return NoResponseSnafu.fail();
                }
            }
        }
    }
}
