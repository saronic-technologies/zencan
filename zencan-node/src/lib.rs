//! Zencan Node implementation
//!
//! Used to implement a node
#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![warn(missing_docs)]
#![allow(clippy::comparison_chain)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod lss_slave;

mod node;
mod node_mbox;
mod node_state;
pub(crate) mod pdo;
mod persist;
mod sdo_server;
mod storage;

// Re-export proc macros
pub use zencan_macro::build_object_dict;
pub use zencan_macro::record_object;

// Re-export types used by generated code
pub use critical_section;
pub use heapless;
pub use zencan_common as common;

pub use node::Node;
pub use node_mbox::NodeMbox;
pub use node_state::{NodeState, NodeStateAccess};
pub use persist::restore_stored_objects;

#[cfg(feature = "socketcan")]
#[cfg_attr(docsrs, doc(cfg(feature = "socketcan")))]
pub use common::open_socketcan;

/// Include the code generated for the object dict in the build script.
#[macro_export]
macro_rules! include_modules {
    ($name: tt) => {
        include!(env!(
            concat!("ZENCAN_INCLUDE_GENERATED_", stringify!($name),),
            concat!(
                "Missing env var ",
                "ZENCAN_INCLUDE_GENERATED_",
                stringify!($name),
                ". Did you generate an object dictionary named ",
                stringify!($name),
                " in build.rs?"
            )
        ));
    };
}
