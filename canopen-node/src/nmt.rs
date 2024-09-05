
use canopen_common::messages::{CanOpenMessage, NmtCommandCmd, NmtState};


pub struct NmtSlave {
    node_id: Option<u8>,
    state: NmtState,
    app_reset_callback: Option<&'static mut dyn FnMut()>,
    nmt_state_callback: Option<&'static mut dyn FnMut(NmtState)>,
}

impl NmtSlave {
    pub fn new(node_id: Option<u8>) -> Self {
        let state = NmtState::Bootup;

        Self {
            node_id,
            state,
            app_reset_callback: None,
            nmt_state_callback: None,
        }
    }

    pub fn update(&mut self, msg: Option<&CanOpenMessage>) {
        let prev_state = self.state;
        if let Some(msg) = msg {
            match msg {
                CanOpenMessage::NmtCommand(cmd) => {
                    match cmd.cmd {
                        NmtCommandCmd::Start => self.state = NmtState::Operational,
                        NmtCommandCmd::Stop => self.state = NmtState::Stopped,
                        NmtCommandCmd::EnterPreOp => self.state = NmtState::PreOperational,
                        NmtCommandCmd::ResetApp => {
                            if let Some(cb) = self.app_reset_callback.as_mut() {
                                cb();
                            }
                            self.state = NmtState::PreOperational;
                        },
                        NmtCommandCmd::ResetComm => self.state = NmtState::PreOperational,
                    }
                },
                _ => (),
            }
        }

        if self.node_id.is_some() && self.state == NmtState::Bootup {
            if let Some(cb) = self.app_reset_callback.as_mut() {
                cb();
            }
            self.state = NmtState::PreOperational;
        }

        if self.state != prev_state {
            if let Some(cb) = self.nmt_state_callback.as_mut() {
                cb(self.state);
            }
        }
    }

    pub fn set_nmt_state_callback(&mut self, callback: &'static mut dyn FnMut(NmtState)) {
        self.nmt_state_callback = Some(callback);
    }

    pub fn state(&self) -> NmtState {
        self.state
    }

    pub fn node_id(&self) -> Option<u8> {
        self.node_id
    }
}

pub struct NmtMaster {

}