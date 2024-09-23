use canopen_build::compile_eds_to_string;
use eds_parser::ElectronicDataSheet;

extern crate proc_macro;
use proc_macro::TokenStream;
use std::str::FromStr as _;

#[proc_macro]
pub fn build_object_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let str_literal: syn::LitStr = syn::parse(input).expect("Expected string literal as macro argument");
    let eds = ElectronicDataSheet::from_str(str_literal.value()).expect("Error parsing EDS file");
    TokenStream::from_str(&compile_eds_to_string(&eds, true).unwrap()).unwrap().into()
}
