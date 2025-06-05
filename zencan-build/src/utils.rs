//! Utilities
//!
//!
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Utility for generating code snippet
pub fn scalar_write_snippet(field_name: &syn::Ident, ty: &syn::Type) -> TokenStream {
    let setter_name = format_ident!("set_{}", field_name);
    quote! {
        if offset != 0 {
            return Err(zencan_node::common::sdo::AbortCode::UnsupportedAccess);
        }
        let value = #ty::from_le_bytes(data.try_into().map_err(|_| {
            if data.len() < size_of::<#ty>() {
                zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthLow
            } else {
                zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthHigh
            }
        })?);
        self.#setter_name(value);
    }
}

/// Utility for generating code snippet
pub fn scalar_read_snippet(field_name: &syn::Ident) -> TokenStream {
    let getter_name = format_ident!("get_{}", field_name);
    quote! {
        let bytes = self.#getter_name().to_le_bytes();
        if offset + buf.len() > bytes.len() {
            return Err(zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthHigh);
        }
        buf.copy_from_slice(&bytes[offset..offset + buf.len()]);
    }
}

/// Utility for generating code snippet
pub fn string_write_snippet(field_name: &syn::Ident, size: usize) -> TokenStream {
    quote! {
        if offset + data.len() > #size {
            return Err(zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthHigh);
        }

        // Unwrap safety: closure always returns Some(_) so fetch_update will never fail
        self.#field_name.fetch_update(|old| {
            let mut new = old;
            new[offset..offset + data.len()].copy_from_slice(data);
            Some(new)
        }).unwrap();
    }
}

/// Utility for generating code snippet
pub fn string_read_snippet(field_name: &syn::Ident, size: usize) -> TokenStream {
    quote! {
        if offset + buf.len() > #size {
            return Err(zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthHigh);
        }

        let value = self.#field_name.load();
        buf.copy_from_slice(&value[offset..offset + buf.len()]);
    }
}
