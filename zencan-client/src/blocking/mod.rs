//! Blocking counterparts of the async client APIs
//!
//! These are built on the synchronous [`CanSender`](zencan_common::can::CanSender) and
//! [`CanReceiver`](zencan_common::can::CanReceiver) traits, so they need no async runtime.
pub mod nmt_master;
mod sdo_client;

pub use nmt_master::NmtMaster;
pub use sdo_client::SdoClient;
