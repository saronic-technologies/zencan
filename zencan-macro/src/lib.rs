// use eds_parser::ElectronicDataSheet;

extern crate proc_macro;
use proc_macro::TokenStream;
use zencan_build::{device_config::DeviceConfig, device_config_to_string};
use std::str::FromStr as _;


#[proc_macro]
pub fn build_object_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let str_literal: syn::LitStr = syn::parse(input).expect("Expected string literal as macro argument");
    let device = DeviceConfig::load_from_str(&str_literal.value()).expect("Error parsing EDS file");
    TokenStream::from_str(&device_config_to_string(&device, true).unwrap()).unwrap()
}
