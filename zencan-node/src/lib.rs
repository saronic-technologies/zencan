#![no_std]
#![allow(clippy::comparison_chain)]

pub mod lss_slave;
pub mod nmt;
pub mod node;
pub mod node_mbox;
pub mod node_state;
pub mod sdo_server;
pub mod stack;

/// Re-expore the proc macro for building code from an inline EDS
pub use zencan_macro::build_object_dict;

// Re-export types used by generated code
pub use zencan_common as common;
pub use crossbeam;
pub use critical_section;

/// Include the code generated for the object dict in the buils script.
#[macro_export]
macro_rules! include_modules {
    ($name: tt) => {
        include!(
            env!(
                concat!(
                    "ZENCAN_INCLUDE_GENERATED_",
                    stringify!($name),
                ),
                concat!(
                    "Missing env var ",
                    "ZENCAN_INCLUDE_GENERATED_",
                    stringify!($name),
                    ". Did you generate an object dictionary named ",
                    stringify!($name),
                    " in build.rs?"
                )
            )
        );
    };
}