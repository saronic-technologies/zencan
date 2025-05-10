use socketcan::{Frame, EmbeddedFrame};
use zencan_client::common::{
    messages::{CanError, CanId, CanMessage, ZencanMessage},
    traits::AsyncCanReceiver,
};
use clap::Parser;

#[derive(Parser)]
struct Args {
    socket: String,
}

pub enum Message {
    Unrecognized(CanMessage),
    Recognized(ZencanMessage),
}

/// Convert a socketcan CanId to a zencan CanId
fn convert_rx_canid(rx_id: socketcan::CanId) -> CanId {
    match rx_id {
        socketcan::CanId::Standard(standard_id) => CanId::std(standard_id.as_raw()),
        socketcan::CanId::Extended(extended_id) => CanId::extended(extended_id.as_raw()),
    }
}

impl TryFrom<socketcan::CanFrame> for Message {
    type Error = CanError;
    fn try_from(value: socketcan::CanFrame) -> Result<Self, Self::Error> {
        let id = convert_rx_canid(value.can_id());

        // Convert to zencan CanMessage, unless it is an error frame
        let msg = match value {
            socketcan::CanFrame::Data(frame) => CanMessage::new(id, frame.data()),
            socketcan::CanFrame::Remote(_) => CanMessage::new_rtr(id),
            socketcan::CanFrame::Error(frame) =>
                return Err(CanError::from_raw(frame.error_bits() as u8)),
        };

        // Attempt to parse as a recognized Zencan message, and fallback to displaying it as a
        // generic can message
        let zenmsg: Result<ZencanMessage, _> = msg.try_into();
        Ok(match zenmsg {
            Ok(msg) => Message::Recognized(msg),
            Err(_) => Message::Unrecognized(msg),
        })
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let (_tx, mut rx) = zencan_client::socketcan::open_socketcan(&args.socket);

    loop {
        if let Ok(msg) = rx.recv().await {
            let time = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, false);
            println!("{time}: {msg:?}");
        }
    }

}