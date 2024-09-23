use canopen_common::{
    objects::{ObjectDict, SubObject},
    sdo::{AbortCode, SdoRequest, SdoResponse},
    traits::{CanFdMessage, CanId, CanSender},
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
enum State {
    #[default]
    Idle,
    DownloadSegment,
    UploadSegment,
}

pub struct SdoServer {
    toggle_state: bool,
    state: State,
    segment_counter: u16,
    index: u16,
    sub: u8,
    tx_cob_id: CanId,
}

/// Attempt to find a subobject in the object dict
///
/// Returns an Err<SdoResponse> with the appropriate SDO abort response on failure
fn lookup_sub_object<'a>(
    od: &ObjectDict<'a>,
    index: u16,
    sub: u8,
) -> Result<SubObject<'a>, SdoResponse> {
    let obj = match od.find(index) {
        Some(obj) => obj,
        None => return Err(SdoResponse::abort(index, sub, AbortCode::NoSuchObject)),
    };

    let sub_obj = match obj.get_sub(sub) {
        Some(obj) => obj,
        None => return Err(SdoResponse::abort(index, sub, AbortCode::NoSuchSubIndex)),
    };

    Ok(sub_obj)
}

impl SdoServer {
    pub fn new(tx_cob_id: CanId) -> Self {
        let toggle_state = false;
        let state = State::Idle;
        let segment_counter = 0;
        let index = 0;
        let sub = 0;
        Self {
            toggle_state,
            state,
            segment_counter,
            tx_cob_id,
            index,
            sub,
        }
    }

    fn validate_download_size(
        &self,
        dl_size: usize,
        subobj: &SubObject,
    ) -> Result<(), SdoResponse> {
        if subobj.data_type.is_str() {
            // Strings can write shorter lengths
            if dl_size > subobj.size {
                return Err(SdoResponse::abort(
                    self.index,
                    self.sub,
                    AbortCode::DataTypeMismatchLengthHigh,
                ));
            }
        } else {
            // All other types require exact size
            if dl_size < subobj.size {
                return Err(SdoResponse::abort(
                    self.index,
                    self.sub,
                    AbortCode::DataTypeMismatchLengthLow,
                ));
            } else if dl_size > subobj.size {
                return Err(SdoResponse::abort(
                    self.index,
                    self.sub,
                    AbortCode::DataTypeMismatchLengthHigh,
                ));
            }
        }
        Ok(())
    }
    pub fn handle_request(
        &mut self,
        req: &SdoRequest,
        od: &ObjectDict,
        sender: &mut dyn FnMut(CanFdMessage),
    ) {
        match req {
            SdoRequest::InitiateUpload { index, sub } => {
                let subobj = match lookup_sub_object(od, *index, *sub) {
                    Ok(subobj) => subobj,
                    Err(response) => {
                        sender(response.to_can_message(self.tx_cob_id));
                        self.state = State::Idle;
                        return;
                    }
                };

                let mut buf = [0u8; 4];
                self.toggle_state = false;

                if subobj.current_size() <= 4 {
                    // Do expedited upload
                    subobj.read(0, &mut buf[0..subobj.current_size()]);
                    sender(
                        SdoResponse::expedited_upload(*index, *sub, &buf[0..subobj.current_size()])
                            .to_can_message(self.tx_cob_id),
                    );
                    self.state = State::Idle;
                } else {
                    // Segmented upload
                    self.state = State::UploadSegment;
                    self.segment_counter = 0;
                    sender(
                        SdoResponse::upload_acknowledge(*index, *sub, subobj.current_size() as u32)
                            .to_can_message(self.tx_cob_id),
                    );
                }
            }
            SdoRequest::InitiateDownload {
                n,
                e,
                s,
                index,
                sub,
                data,
            } => {
                self.index = *index;
                self.sub = *sub;
                if *e {
                    // Doing an expedited download
                    let subobj = match lookup_sub_object(od, *index, *sub) {
                        Ok(subobj) => subobj,
                        Err(response) => {
                            sender(response.to_can_message(self.tx_cob_id));
                            return;
                        }
                    };

                    // Verify data size requested by client fits object, and abort if not
                    let dl_size = 4 - *n as usize;
                    if let Err(abort_resp) = self.validate_download_size(dl_size, &subobj) {
                        self.state = State::Idle;
                        sender(abort_resp.to_can_message(self.tx_cob_id));
                        return;
                    }

                    subobj.write(0, &data[0..dl_size]);
                    // When writing a string with length less than buffer, zero terminate
                    // Note: dl_size != subobj.size implies the data type of the object is a string
                    if dl_size < subobj.size {
                        subobj.write(dl_size, &[0]);
                    }

                    let resp = SdoResponse::download_acknowledge(*index, *sub)
                        .to_can_message(self.tx_cob_id);
                    sender(resp);
                } else {
                    // starting a segmented download
                    let subobj = match lookup_sub_object(od, *index, *sub) {
                        Ok(subobj) => subobj,
                        Err(response) => {
                            sender(response.to_can_message(self.tx_cob_id));
                            return;
                        }
                    };

                    // If size is provided, verify data size requested by client fits object, and
                    // abort if not
                    if *s {
                        let dl_size = 4 - *n as usize;
                        if let Err(abort_resp) = self.validate_download_size(dl_size, &subobj) {
                            self.state = State::Idle;
                            sender(abort_resp.to_can_message(self.tx_cob_id));
                            return;
                        }
                    }

                    self.toggle_state = false;
                    self.segment_counter = 0;
                    self.state = State::DownloadSegment;

                    sender(
                        SdoResponse::download_acknowledge(*index, *sub)
                            .to_can_message(self.tx_cob_id),
                    )
                }
            }
            SdoRequest::DownloadSegment { t, n, c, data } => {
                if self.state != State::DownloadSegment {
                    let resp = SdoResponse::abort(
                        self.index,
                        self.sub,
                        AbortCode::InvalidCommandSpecifier,
                    )
                    .to_can_message(self.tx_cob_id);
                    sender(resp);
                    self.state = State::Idle;
                    return;
                }

                if *t != self.toggle_state {
                    let resp =
                        SdoResponse::abort(self.index, self.sub, AbortCode::ToggleNotAlternated)
                            .to_can_message(self.tx_cob_id);
                    sender(resp);
                    self.state = State::Idle;
                    return;
                }

                // Unwrap safety: If in DownloadSegment state, then the existence of the sub object
                // is already established.
                let subobj = lookup_sub_object(od, self.index, self.sub).unwrap();
                let offset = self.segment_counter as usize * 7;
                let segment_size = 7 - *n as usize;
                let write_len = offset + segment_size;
                // Make sure this segment won't overrun the allocated storage
                if write_len > subobj.size {
                    sender(
                        SdoResponse::abort(
                            self.index,
                            self.sub,
                            AbortCode::DataTypeMismatchLengthHigh,
                        )
                        .to_can_message(self.tx_cob_id),
                    );
                    self.state = State::Idle;
                    return;
                }
                subobj.write(offset, &data[0..segment_size]);
                // If this is the last segment, and it's shorter than the object, zero terminate
                if *c {
                    if write_len < subobj.size {
                        subobj.write(write_len, &[0]);
                    }
                }
                let resp = SdoResponse::download_segment_acknowledge(self.toggle_state)
                    .to_can_message(self.tx_cob_id);
                sender(resp);
                self.toggle_state = !self.toggle_state;
                self.segment_counter += 1;
            }

            SdoRequest::ReqUploadSegment { t } => {
                if self.state != State::UploadSegment {
                    sender(
                        SdoResponse::abort(
                            self.index,
                            self.sub,
                            AbortCode::InvalidCommandSpecifier,
                        )
                        .to_can_message(self.tx_cob_id),
                    );
                    self.state = State::Idle;
                    return;
                }
                if *t != self.toggle_state {
                    sender(
                        SdoResponse::abort(self.index, self.sub, AbortCode::ToggleNotAlternated)
                            .to_can_message(self.tx_cob_id),
                    );
                    self.state = State::Idle;
                    return;
                }
                // Unwrap safety: We validate sub exists before ever setting self.index and self.sub
                let subobj = od.find_sub(self.index, self.sub).unwrap();
                let read_offset = self.segment_counter as usize * 7;
                let read_size = (subobj.current_size() - read_offset).min(7);
                let mut buf = [0; 7];
                subobj.read(self.segment_counter as usize * 7, &mut buf[0..read_size]);
                // Compute complete bit (is this the last segment of the upload?)
                let c = (read_size + read_offset) == subobj.current_size();
                sender(
                    SdoResponse::upload_segment(self.toggle_state, c, &buf[0..read_size])
                        .to_can_message(self.tx_cob_id),
                );
                self.segment_counter += 1;

                self.toggle_state = !self.toggle_state;
                if c {
                    self.state = State::Idle;
                }
            }
            SdoRequest::InitiateBlockDownload {
                cc,
                s,
                cs,
                index,
                sub,
                size,
            } => todo!(),
            SdoRequest::InitiateBlockUpload {} => todo!(),
            SdoRequest::Abort {
                index: _,
                sub: _,
                abort_code: _,
            } => {
                self.state = State::Idle;
            }
        }
    }
}
