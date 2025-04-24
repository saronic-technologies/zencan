use futures::executor::block_on;
use integration_tests::sim_bus::SimBusReceiver;
use zencan_common::messages::ZencanMessage;
use zencan_common::traits::AsyncCanReceiver;


pub struct BusLogger {
    rx: SimBusReceiver,
}

impl BusLogger {
    pub fn new(rx: SimBusReceiver) -> Self {
        Self { rx }
    }

    pub fn print(&mut self) {
        while let Some(msg) = block_on(self.rx.try_recv()) {
            let parsed_msg: Result<ZencanMessage, _> = msg.try_into();
            if let Ok(msg) = parsed_msg {
                println!("Received message: {:?}", msg);
            } else {
                println!("Received message: {:?}", msg);
            }
        }
    }
}

impl Drop for BusLogger {
    fn drop(&mut self) {
        self.print();
    }
}