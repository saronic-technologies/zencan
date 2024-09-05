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

    pub fn handle_request(
        &mut self,
        req: &SdoRequest,
        od: &ObjectDict,
        sender: &mut dyn CanSender,
    ) {
        match req {
            SdoRequest::InitiateUpload { index, sub } => {
                let subobj = match lookup_sub_object(od, *index, *sub) {
                    Ok(subobj) => subobj,
                    Err(response) => {
                        sender
                            .send(response.to_can_message(self.tx_cob_id))
                            .unwrap();
                        self.state = State::Idle;
                        return;
                    }
                };

                let mut buf = [0u8; 4];
                self.toggle_state = false;

                if subobj.size <= 4 {
                    // Do expedited upload
                    subobj.read(0, &mut buf[0..subobj.size]);
                    sender
                        .send(
                            SdoResponse::expedited_upload(*index, *sub, &buf[0..subobj.size])
                                .to_can_message(self.tx_cob_id),
                        )
                        .unwrap();
                    self.state = State::Idle;
                } else {
                    // Segmented upload
                    self.state = State::UploadSegment;
                    self.segment_counter = 0;
                    sender
                        .send(
                            SdoResponse::upload_acknowledge(*index, *sub, subobj.size as u32)
                                .to_can_message(self.tx_cob_id),
                        )
                        .unwrap();
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
                    let subobj = match lookup_sub_object(od, *index, *sub) {
                        Ok(subobj) => subobj,
                        Err(response) => {
                            sender
                                .send(response.to_can_message(self.tx_cob_id))
                                .unwrap();
                            return;
                        }
                    };

                    // Check length
                    let dl_size = 4 - *n as usize;
                    // Verify data size fits object
                    // Strings can write shorter lengths; all other types must be exact
                    if subobj.data_type.is_str() {
                        if dl_size > subobj.size {
                            sender.send(
                                SdoResponse::abort(
                                    self.index,
                                    self.sub,
                                    AbortCode::DataTypeMismatchLengthHigh,
                                )
                                .to_can_message(self.tx_cob_id),
                            ).unwrap();
                            self.state = State::Idle;
                        } else {
                            if dl_size < subobj.size {
                                sender.send(
                                    SdoResponse::abort(
                                        self.index,
                                        self.sub,
                                        AbortCode::DataTypeMismatchLengthLow,
                                    )
                                    .to_can_message(self.tx_cob_id),
                                ).unwrap();
                            } else if dl_size > subobj.size {
                                sender.send(
                                    SdoResponse::abort(
                                        self.index,
                                        self.sub,
                                        AbortCode::DataTypeMismatchLengthHigh,
                                    )
                                    .to_can_message(self.tx_cob_id),
                                ).unwrap();
                            }
                        }
                    }
                    subobj.write(0, &data[0..dl_size]);
                    let resp = SdoResponse::download_acknowledge(*index, *sub)
                        .to_can_message(self.tx_cob_id);
                    sender.send(resp).unwrap();
                } else {
                    // Check size matches data field
                    self.toggle_state = false;
                    self.state = State::DownloadSegment;
                    // TODO: Store what we're downloading
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
                    sender.send(resp).unwrap();
                    self.state = State::Idle;
                    return;
                }

                if *t != self.toggle_state {
                    let resp =
                        SdoResponse::abort(self.index, self.sub, AbortCode::ToggleNotAlternated)
                            .to_can_message(self.tx_cob_id);
                    sender.send(resp).unwrap();
                    self.state = State::Idle;
                    return;
                }

                // TODO: store data
                self.toggle_state = *t;
                let resp = SdoResponse::download_segment_acknowledge(self.toggle_state)
                    .to_can_message(self.tx_cob_id);
                sender.send(resp).unwrap();
            }

            SdoRequest::ReqUploadSegment { t } => {
                if self.state != State::UploadSegment {
                    sender
                        .send(
                            SdoResponse::abort(
                                self.index,
                                self.sub,
                                AbortCode::InvalidCommandSpecifier,
                            )
                            .to_can_message(self.tx_cob_id),
                        )
                        .unwrap();
                    self.state = State::Idle;
                    return;
                }
                if *t != self.toggle_state {
                    sender
                        .send(
                            SdoResponse::abort(
                                self.index,
                                self.sub,
                                AbortCode::ToggleNotAlternated,
                            )
                            .to_can_message(self.tx_cob_id),
                        )
                        .unwrap();
                    self.state = State::Idle;
                    return;
                }
                // Unwrap safety: We validate sub exists before ever setting self.index and self.sub
                let subobj = od.find_sub(self.index, self.sub).unwrap();
                let read_offset = self.segment_counter as usize * 7;
                let read_size = (subobj.size - read_offset).max(7);
                let mut buf = [0; 7];
                subobj.read(self.segment_counter as usize * 7, &mut buf[0..read_size]);
                // Compute complete bit (is this the last segment of the upload?)
                let c = (read_size + read_offset) == subobj.size;
                sender
                    .send(
                        SdoResponse::upload_segment(self.toggle_state, c, &buf[0..read_size])
                            .to_can_message(self.tx_cob_id),
                    )
                    .unwrap();

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
        }
    }
}
