// use eds_parser::ElectronicDataSheet;

extern crate proc_macro;
use proc_macro::TokenStream;
use std::str::FromStr as _;
use zencan_build::device_config_to_string;
use zencan_common::device_config::DeviceConfig;

mod derive_record;
use derive_record::record_object_impl;

/// Macro to build an object dict from inline device config TOML
#[proc_macro]
pub fn build_object_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let str_literal: syn::LitStr =
        syn::parse(input).expect("Expected string literal as macro argument");
    let device =
        DeviceConfig::load_from_str(&str_literal.value()).expect("Error parsing device config");
    proc_macro::TokenStream::from_str(&device_config_to_string(&device, true).unwrap()).unwrap()
}

/// Attribute macro to turn a basic struct into a zencan record object.
///
/// It does the following:
/// - It wraps all of the fields in the struct in AtomicCell, to make then Sync
/// - It generates an impl with set_* and get_* methods for accessing each field
/// - It implements the ObjectRawAccess trait for the struct
///
/// It can only be applied to structs with named fields, and the fields can only use a subset of
/// types:
///
/// - u32, u16, u8
/// - i32, i16, i8
/// - f32
/// - [u8; N] (This creates a visible string type sub object. TODO: an attribute to allow it to be
///   the other types of strings would be good )
#[proc_macro_attribute]
pub fn record_object(attr: TokenStream, item: TokenStream) -> TokenStream {
    record_object_impl(attr, item)
}
