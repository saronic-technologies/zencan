use std::time::{Duration, Instant};

use zencan_common::{
    sdo::{AbortCode, ClientCommand, SdoRequest, SdoResponse},
    traits::{CanFdMessage, CanId, CanReceiver, CanSender},
};
use snafu::Snafu;

const RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);

#[derive(Debug, PartialEq, Snafu)]
pub enum SdoClientError {
    NoResponse,
    MalformedResponse,
    UnexpectedResponse,
    ServerAbort { abort_code: u32 },
    ToggleNotAlternated,
}

type Result<T> = std::result::Result<T, SdoClientError>;

pub struct SdoClient<S, R> {
    req_cob_id: CanId,
    resp_cob_id: CanId,
    sender: S,
    receiver: R,
}

impl<S: CanSender, R: CanReceiver> SdoClient<S, R> {
    pub fn new_std(server_node_id: u8, sender: S, receiver: R) -> Self {
        Self {
            req_cob_id: CanId::Std(0x600 + server_node_id as u16),
            resp_cob_id: CanId::Std(0x580 + server_node_id as u16),
            sender,
            receiver,
        }
    }

    pub fn download(&mut self, index: u16, sub: u8, data: &[u8]) -> Result<()> {
        if data.len() <= 4 {
            // Do an expedited transfer
            let msg =
                SdoRequest::expedited_download(index, sub, data).to_can_message(self.req_cob_id);
            self.sender.send(msg).unwrap(); // TODO: Expect errors

            let resp = self.wait_for_response(RESPONSE_TIMEOUT)?;
            match resp {
                SdoResponse::ConfirmDownload { index: _, sub: _ } => {
                    return Ok(()); // Success!
                }
                SdoResponse::Abort {
                    index: _,
                    sub: _,
                    abort_code,
                } => {
                    return ServerAbortSnafu { abort_code }.fail();
                }
                _ => return UnexpectedResponseSnafu.fail(),
            }
        } else {
            let msg = SdoRequest::initiate_download(index, sub, Some(data.len() as u32))
                .to_can_message(self.req_cob_id);
            self.sender.send(msg).unwrap();

            let resp = self.wait_for_response(RESPONSE_TIMEOUT)?;
            match resp {
                SdoResponse::ConfirmDownload { index: _, sub: _ } => (),
                SdoResponse::Abort {
                    index: _,
                    sub: _,
                    abort_code,
                } => return ServerAbortSnafu { abort_code }.fail(),
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
                    .expect("failed sending DL segment");
                let resp = self.wait_for_response(RESPONSE_TIMEOUT)?;
                match resp {
                    SdoResponse::ConfirmDownloadSegment { t } => {
                        // Fail if toggle value doesn't match
                        if t != toggle {
                            let abort_msg =
                                SdoRequest::abort(index, sub, AbortCode::ToggleNotAlternated)
                                    .to_can_message(self.req_cob_id);
                            self.sender.send(abort_msg).expect("Error sending abort");
                            return ToggleNotAlternatedSnafu.fail();
                        }
                        // Otherwise, carry on
                    }
                    SdoResponse::Abort {
                        index: _,
                        sub: _,
                        abort_code,
                    } => return ServerAbortSnafu { abort_code }.fail(),
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

    pub fn upload(&mut self, index: u16, sub: u8) -> Result<Vec<u8>> {
        let mut read_buf = Vec::new();

        let msg = SdoRequest::initiate_upload(index, sub).to_can_message(self.req_cob_id);
        self.sender.send(msg).unwrap();

        let resp = self.wait_for_response(RESPONSE_TIMEOUT)?;

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
                index: _,
                sub: _,
                abort_code,
            } => return ServerAbortSnafu { abort_code }.fail(),
            _ => return UnexpectedResponseSnafu.fail(),
        };

        if !expedited {
            // Read segments
            let mut toggle = false;
            loop {
                let msg =
                    SdoRequest::upload_segment_request(toggle).to_can_message(self.req_cob_id);

                self.sender.send(msg).unwrap();

                let resp = self.wait_for_response(RESPONSE_TIMEOUT)?;
                match resp {
                    SdoResponse::UploadSegment { t, n, c, data } => {
                        if t != toggle {
                            self.sender
                                .send(
                                    SdoRequest::abort(index, sub, AbortCode::ToggleNotAlternated)
                                        .to_can_message(self.req_cob_id),
                                )
                                .expect("Error sending abort");
                            return ToggleNotAlternatedSnafu.fail()
                        }
                        read_buf.extend_from_slice(&data[0..7 - n as usize]);
                        if c {
                            // Transfer complete
                            break;
                        }
                    }
                    SdoResponse::Abort {
                        index: _,
                        sub: _,
                        abort_code,
                    } => return ServerAbortSnafu { abort_code }.fail(),
                    _ => return UnexpectedResponseSnafu.fail(),
                }
                toggle = !toggle;
            }
        }
        Ok(read_buf)
    }

    fn wait_for_response(&mut self, mut timeout: Duration) -> Result<SdoResponse> {
        let wait_until = Instant::now() + timeout;
        loop {
            let msg = self
                .receiver
                .recv(timeout)
                .map_err(|_| NoResponseSnafu.build())?;
            if msg.id == self.resp_cob_id {
                return Ok(msg.try_into().map_err(|_| MalformedResponseSnafu.build())?);
            }
            timeout = wait_until.saturating_duration_since(Instant::now());
            if timeout.is_zero() {
                return NoResponseSnafu.fail();
            }
        }
    }
}
