#![no_std]

pub mod lss;
pub mod nmt;
pub mod node;
pub mod sdo_server;
pub mod stack;


/// Re-expore the proc macro for building code from an inline EDS
pub use zencan_macro::build_object_dict;

/// Include the code generated for the object dict in the buils script.
#[macro_export]
macro_rules! include_modules {
    () => {
        include!(env!("CANOPEN_INCLUDE_GENERATED"));
    };
}