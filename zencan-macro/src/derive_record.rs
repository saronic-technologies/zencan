use darling::util::Flag;
use darling::{ast, FromDeriveInput, FromField, FromMeta};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::parse_macro_input;
use syn::spanned::Spanned;
use syn::{DeriveInput, Expr, Lit, Type};

fn type_to_zentype_and_size(ty: &Type) -> Result<(bool, TokenStream, usize), syn::Error> {
    match ty {
        Type::Array(array) => {
            let elem = array.elem.as_ref();

            let array_err = syn::Error::new(
                array.span(),
                "Only arrays of type [u8] are supported by RecordObject macro",
            );
            let len: usize = match &array.len {
                Expr::Lit(lit) => match &lit.lit {
                    Lit::Int(lit) => lit.base10_parse()?,
                    _ => return Err(array_err),
                },
                _ => panic!("ahh"),
            };
            //let len: usize = parse_exp::<syn::LitInt>(array.len)?.value() as usize;
            let elem = match elem {
                Type::Path(path) => path.path.require_ident()?.clone(),
                _ => return Err(array_err),
            };

            if elem.to_string() == "u8" {
                Ok((
                    true,
                    quote!(zencan_node::common::objects::DataType::VisibleString),
                    len,
                ))
            } else {
                Err(array_err)
            }
        }
        Type::Path(type_path) => {
            let ty = type_path.path.require_ident()?;
            match ty.to_string().as_str() {
                "u32" => Ok((
                    false,
                    quote! { zencan_node::common::objects::DataType::UInt32 },
                    4,
                )),
                "u16" => Ok((
                    false,
                    quote! { zencan_node::common::objects::DataType::UInt16 },
                    2,
                )),
                "u8" => Ok((
                    false,
                    quote! { zencan_node::common::objects::DataType::UInt8 },
                    1,
                )),
                "i32" => Ok((
                    false,
                    quote! { zencan_node::common::objects::DataType::Int32 },
                    4,
                )),
                "i16" => Ok((
                    false,
                    quote! { zencan_node::common::objects::DataType::Int16 },
                    2,
                )),
                "i8" => Ok((
                    false,
                    quote! { zencan_node::common::objects::DataType::Int8 },
                    1,
                )),
                "f32" => Ok((
                    false,
                    quote! { zencan_node::common::objects::DataType::Real32 },
                    4,
                )),
                _ => panic!("OOOOPPPPS"),
            }
        }

        _ => Err(syn::Error::new(
            ty.span(),
            format!("Type {:?} not supported", ty.to_token_stream().to_string()),
        )),
    }
}

#[derive(Debug, Clone, Copy, FromMeta, Default)]
#[darling(default)]
enum PdoMapping {
    #[default]
    None,
    Tpdo,
    Rpdo,
    Both,
}

#[derive(Debug, FromField)]
#[darling(attributes(record))]
struct FieldAttrs {
    ident: Option<syn::Ident>,

    ty: syn::Type,

    #[darling(default)]
    pdo: PdoMapping,
    persist: Flag,
}

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(record),
    supports(struct_named),
    forward_attrs(allow, doc, cfg)
)]
struct RecordObjectReceiver {
    /// The struct ident.
    ident: syn::Ident,

    data: ast::Data<(), FieldAttrs>,
}

fn scalar_write_snippet(field_name: &syn::Ident, ty: &syn::Type, _size: usize) -> TokenStream {
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

fn scalar_read_snippet(field_name: &syn::Ident) -> TokenStream {
    let getter_name = format_ident!("get_{}", field_name);
    quote! {
        let bytes = self.#getter_name().to_le_bytes();
        if offset + buf.len() > bytes.len() {
            return Err(zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthHigh);
        }
        buf.copy_from_slice(&bytes[offset..offset + buf.len()]);
    }
}

fn string_write_snippet(field_name: &syn::Ident, size: usize) -> TokenStream {
    quote! {
        if offset + data.len() > #size {
            return Err(zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthHigh);
        }

        // Unwrap safety: closure always returns Some(_) so fetch_update will never fail
        self.#field_name.fetch_update(|old| {
            let mut new = old.clone();
            new[offset..offset + data.len()].copy_from_slice(data);
            Some(new)
        }).unwrap();
    }
}

fn string_read_snippet(field_name: &syn::Ident, size: usize) -> TokenStream {
    quote! {
        if offset + buf.len() > #size {
            return Err(zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthHigh);
        }

        let value = self.#field_name.load();
        buf.copy_from_slice(&value[offset..offset + buf.len()]);
    }
}

fn generate_object_raw_access_impl(receiver: &RecordObjectReceiver) -> TokenStream {


    let struct_name = &receiver.ident;
    let fields = receiver
        .data
        .as_ref()
        .take_struct()
        .expect("Should never be enum")
        .fields;

    let max_sub_idx = fields.len() as u8;

    let mut write_cases = quote! {
        0 => return Err(zencan_node::common::sdo::AbortCode::ReadOnly),
    };
    let mut read_cases = quote! {
        0 => {
            if offset > 0 {
                return Err(zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthHigh);
            }
            if buf.len() > 1 {
                return Err(zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthHigh);
            }
            if buf.len() == 0 {
                return Err(zencan_node::common::sdo::AbortCode::DataTypeMismatchLengthLow);
            }
            buf[0] = #max_sub_idx;
            Ok(())
        }
    };
    let mut sub_info_cases = quote! {
        0 => Ok(zencan_node::common::objects::SubInfo {
            size: 1,
            data_type: zencan_node::common::objects::DataType::UInt8,
            access_type: zencan_node::common::objects::AccessType::Const,
            ..Default::default()
        }),
    };



    let mut sub_idx = 1u8;
    for field in &fields {
        let (is_str, zencan_type, size) = match type_to_zentype_and_size(&field.ty) {
            Ok(ty) => ty,
            Err(e) => panic!("{e}"),
        };

        let pdo_mapping = match field.pdo {
            PdoMapping::None => quote!(zencan_node::common::objects::PdoMapping::None),
            PdoMapping::Tpdo => quote!(zencan_node::common::objects::PdoMapping::Tpdo),
            PdoMapping::Rpdo => quote!(zencan_node::common::objects::PdoMapping::Rpdo),
            PdoMapping::Both => quote!(zencan_node::common::objects::PdoMapping::Both),
        };
        let persist = field.persist.is_present();

        sub_info_cases.extend(quote! {
            #sub_idx => Ok(zencan_node::common::objects::SubInfo {
                size: #size,
                data_type: #zencan_type,
                access_type: zencan_node::common::objects::AccessType::Rw,
                persist: #persist,
                pdo_mapping: #pdo_mapping,
                ..Default::default()
            }),
        });

        let write_snippet;
        let read_snippet;
        if is_str {
            write_snippet = string_write_snippet(field.ident.as_ref().unwrap(), size);
            read_snippet = string_read_snippet(field.ident.as_ref().unwrap(), size);
        } else {
            write_snippet = scalar_write_snippet(field.ident.as_ref().unwrap(), &field.ty, size);
            read_snippet = scalar_read_snippet(field.ident.as_ref().unwrap());
        }

        write_cases.extend(quote! {
            #sub_idx => {
                #write_snippet
                Ok(())
            }
        });

        read_cases.extend(quote! {
            #sub_idx => {
                #read_snippet
                Ok(())
            }
        });

        sub_idx += 1;
    }

    return quote! {
        impl zencan_node::common::objects::ObjectRawAccess for #struct_name {
            fn sub_info(&self, sub: u8) -> Result<zencan_node::common::objects::SubInfo, zencan_node::common::sdo::AbortCode> {
                match sub {
                    #sub_info_cases
                    _ => Err(zencan_node::common::sdo::AbortCode::NoSuchSubIndex),
                }
            }

            fn write(&self, sub: u8, offset: usize, data: &[u8]) -> Result<(), zencan_node::common::sdo::AbortCode> {
                match sub {
                    #write_cases
                    _ => Err(zencan_node::common::sdo::AbortCode::NoSuchSubIndex),
                }
            }

            fn read(&self, sub: u8, offset: usize, buf: &mut [u8]) -> Result<(), zencan_node::common::sdo::AbortCode> {
                match sub {
                    #read_cases
                    _ => Err(zencan_node::common::sdo::AbortCode::NoSuchSubIndex),
                }
            }

            fn object_code(&self) -> zencan_node::common::objects::ObjectCode {
                zencan_node::common::objects::ObjectCode::Record
            }
        }
    }.into();
}

fn wrap_struct_fields(receiver: &RecordObjectReceiver) -> TokenStream {
    let mut field_tokens = TokenStream::new();
    let struct_name = &receiver.ident;
    let fields = receiver
    .data
    .as_ref()
    .take_struct()
    .expect("Should never be enum")
    .fields;

    for field in &fields {
        let field_name = field.ident.as_ref().expect("Fields must be named");
        let field_ty = &field.ty;
        field_tokens.extend(quote!{
            #field_name: zencan_node::common::AtomicCell<#field_ty>,
        });
    }

    quote! {
        struct #struct_name {
            #field_tokens
        }
    }
}

fn generate_accessor_impl(receiver: &RecordObjectReceiver) -> TokenStream {
    let struct_name = &receiver.ident;
    let fields = receiver
        .data
        .as_ref()
        .take_struct()
        .expect("Should never be enum")
        .fields;

    let mut accessor_methods = TokenStream::new();

    for field in &fields {
        let field_name = field.ident.as_ref().expect("Fields must be named");
        let field_ty = &field.ty;

        let getter_name = format_ident!("get_{}", field_name);
        let setter_name = format_ident!("set_{}", field_name);

        accessor_methods.extend(quote! {
            pub fn #getter_name(&self) -> #field_ty {
                self.#field_name.load()
            }

            pub fn #setter_name(&self, value: #field_ty) {
                self.#field_name.store(value);
            }
        });
    }

    quote! {
        impl #struct_name {
            #accessor_methods
        }
    }
}

pub fn record_object_impl(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(item as DeriveInput);
    let receiver = RecordObjectReceiver::from_derive_input(&derive_input).unwrap();

    let item_tokens = wrap_struct_fields(&receiver);
    let accessors = generate_accessor_impl(&receiver);
    let raw_access = generate_object_raw_access_impl(&receiver);
    quote! {
        #item_tokens
        #accessors
        #raw_access
    }.into()
}
