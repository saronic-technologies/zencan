use zencan_common::{
    lss::{LssIdentity, LssRequest, LssResponse, LssState, LSS_FASTSCAN_CONFIRM},
    messages::MessageError,
};

pub struct LssSlave {
    state: LssState,
    identity: LssIdentity,
    fast_scan_sub: u8,
}

impl LssSlave {
    pub fn new(identity: LssIdentity) -> Self {
        Self {
            state: LssState::Waiting,
            identity,
            fast_scan_sub: 0,
        }
    }

    /// Process an LSS request, updating the state of the slave
    ///
    /// When a response is generated, it will be returned and should be transmitted to the CAN bus
    pub fn process_request(
        &mut self,
        request: LssRequest,
    ) -> Result<Option<LssResponse>, MessageError> {
        match request {
            LssRequest::SwitchModeGlobal { mode } => {
                self.state = LssState::from_byte(mode)?;
                Ok(None)
            }
            LssRequest::FastScan {
                id,
                bit_check,
                sub,
                next,
            } => {
                if self.state == LssState::Waiting {
                    if bit_check == LSS_FASTSCAN_CONFIRM {
                        // Reset state machine and confirm
                        self.fast_scan_sub = 0;
                        Ok(Some(LssResponse::IdentifySlave))
                    } else if self.fast_scan_sub == sub {
                        let mask = 0xFFFFFFFFu32 << bit_check;
                        if self.identity.by_addr(sub) & mask == (id & mask) {
                            self.fast_scan_sub = next;
                            if bit_check == 0 && next < sub {
                                // All bits matched, enter configuration state
                                self.state = LssState::Configuring;
                            }
                            Ok(Some(LssResponse::IdentifySlave))
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_scan_simple() {
        const VENDOR_ID: u32 = 0x0;
        const PRODUCT_CODE: u32 = 0x1;
        const REVISION: u32 = 0x2;
        const SERIAL_NUMBER: u32 = 0x3;
        const IDENTITY: LssIdentity = LssIdentity {
            vendor_id: VENDOR_ID,
            product_code: PRODUCT_CODE,
            revision: REVISION,
            serial_number: SERIAL_NUMBER,
        };

        let mut slave = LssSlave::new(IDENTITY);

        // Send confirmation message, and it should always ACK
        assert_eq!(
            slave.process_request(LssRequest::FastScan {
                id: 0x00000000,
                bit_check: LSS_FASTSCAN_CONFIRM,
                sub: 0,
                next: 1,
            }),
            Ok(Some(LssResponse::IdentifySlave))
        );

        // 0 Matches
        assert_eq!(
            slave.process_request(LssRequest::FastScan {
                id: 0x00000000,
                bit_check: 31,
                sub: 0,
                next: 1,
            }),
            Ok(Some(LssResponse::IdentifySlave))
        );

        // 1 does not match
        assert_eq!(
            slave.process_request(LssRequest::FastScan {
                id: 0x00000001,
                bit_check: 31,
                sub: 0,
                next: 1,
            }),
            Ok(None)
        );
    }

    /// Make sure that the slave goes into the configuration state after a complete scan
    #[test]
    fn test_fast_scan_configure() {
        const VENDOR_ID: u32 = 0x0;
        const PRODUCT_CODE: u32 = 0x1;
        const REVISION: u32 = 0x2;
        const SERIAL_NUMBER: u32 = 0x3;
        const IDENTITY: LssIdentity = LssIdentity {
            vendor_id: VENDOR_ID,
            product_code: PRODUCT_CODE,
            revision: REVISION,
            serial_number: SERIAL_NUMBER,
        };

        let mut slave = LssSlave::new(IDENTITY);

        let mut id = [0, 0, 0, 0];
        let mut sub = 0;
        let mut next = 0;
        let mut bit_check;

        fn send_fs(slave: &mut LssSlave, id: &[u32; 4], bit_check: u8, sub: u8, next: u8) -> bool {
            let resp = slave
                .process_request(LssRequest::FastScan {
                    id: id[sub as usize],
                    bit_check,
                    sub,
                    next,
                })
                .unwrap();

            matches!(resp, Some(LssResponse::IdentifySlave))
        }

        // The first message resets the LSS state machines, and a response confirms that there is at
        // least one unconfigured slave to discover
        assert!(
            send_fs(&mut slave, &id, LSS_FASTSCAN_CONFIRM, sub, next),
            "No confirmation response"
        );

        while sub < 4 {
            bit_check = 32;
            while bit_check > 0 {
                bit_check -= 1;
                if !send_fs(&mut slave, &id, bit_check, sub, next) {
                    id[sub as usize] |= 1 << bit_check;
                }
            }
            next = (sub + 1) % 4;
            assert!(
                send_fs(&mut slave, &id, bit_check, sub, next),
                "No ack after completing sub {}, id: {:?}",
                sub,
                id
            );

            sub += 1;
        }

        assert_eq!(id, [0x0, 0x1, 0x2, 0x3]);
        assert_eq!(slave.state, LssState::Configuring);
    }
}
