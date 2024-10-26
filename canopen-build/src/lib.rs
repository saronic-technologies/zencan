use std::path::Path;

use canopen_common::objects::{AccessType, DataType};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use snafu::{ResultExt, Snafu};

use eds_parser::{ElectronicDataSheet, Object, SubObject};

trait ParseEdsNum {
    fn parse_eds_i64(&self) -> Result<i64, std::num::ParseIntError>;
    fn parse_eds_u64(&self) -> Result<u64, std::num::ParseIntError>;
}

impl<T: AsRef<str>> ParseEdsNum for T {
    fn parse_eds_i64(&self) -> Result<i64, std::num::ParseIntError> {
        let s = self.as_ref();
        if s.starts_with("0x") {
            i64::from_str_radix(s.strip_prefix("0x").unwrap(), 16)
        } else {
            Ok(s.parse()?)
        }
    }
    fn parse_eds_u64(&self) -> Result<u64, std::num::ParseIntError> {
        let s = self.as_ref();
        if s.starts_with("0x") {
            u64::from_str_radix(s.strip_prefix("0x").unwrap(), 16)
        } else {
            Ok(s.parse()?)
        }
    }
}

#[derive(Debug, Snafu)]
pub enum CompileError {
    EdsLoad {
        source: eds_parser::LoadError,
    },
    MissingSub0 {
        obj_num: u32,
    },
    MissingSub1 {
        obj_num: u32,
    },
    MissingSub {
        obj_num: u32,
        sub_num: u32,
    },
    ParseInt {
        message: String,
        source: std::num::ParseIntError,
    },
    ParseFloat {
        message: String,
        source: std::num::ParseFloatError,
    },
    NotRunViaCargo,
    Io {
        source: std::io::Error,
    },
}

fn string_to_byte_literal(s: &str) -> TokenStream {
    let b = s.as_bytes();
    quote!([#(#b),*])
}

fn get_default_literal(obj: u32, sub: &SubObject) -> Result<TokenStream, CompileError> {
    let s = if sub.default_value.is_empty() {
        match sub.data_type {
            DataType::Boolean
            | DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32 => "0",
            DataType::Real32 => "0.0",
            DataType::VisibleString | DataType::OctetString | DataType::UnicodeString => "",
            DataType::TimeDifference => "",
            DataType::TimeOfDay => "",
            DataType::Domain => "",
            DataType::Other(_) => "",
        }
    } else {
        // The C# EDS editor puts $NODEID+ on fields which are a function of the node id. We drop this
        // here if present, on the assumption that such fields are handled in code.
        sub.default_value.as_str().trim_start_matches("$NODEID+")
    };

    match sub.data_type {
        DataType::Boolean => {
            let num = s.parse_eds_u64().context(ParseIntSnafu {
                message: format!("Can't parse default value on {:x}", obj),
            })?;
            let value = num != 0;
            Ok(quote!(#value))
        }
        DataType::Int8 => {
            let x = s.parse_eds_i64().context(ParseIntSnafu {
                message: format!("Can't parse default on {:x}", obj),
            })? as i8;
            Ok(quote!(#x))
        }
        DataType::Int16 => {
            let x = s.parse_eds_i64().context(ParseIntSnafu {
                message: format!("Can't parse default on {:x}", obj),
            })? as i16;
            Ok(quote!(#x))
        }
        DataType::Int32 => {
            let x = s.parse_eds_i64().context(ParseIntSnafu {
                message: format!("Can't parse default on {:x}", obj),
            })? as i32;
            Ok(quote!(#x))
        }
        DataType::UInt8 => {
            let x = s.parse_eds_u64().context(ParseIntSnafu {
                message: format!("Can't parse default on {:x}", obj),
            })? as u8;
            Ok(quote!(#x))
        }
        DataType::UInt16 => {
            let x = s.parse_eds_u64().context(ParseIntSnafu {
                message: format!("Can't parse default on {:x}", obj),
            })? as u16;
            Ok(quote!(#x))
        }
        DataType::UInt32 => {
            let x = s.parse_eds_u64().context(ParseIntSnafu {
                message: format!("Can't parse default on {:x}", obj),
            })? as u32;
            Ok(quote!(#x))
        }
        DataType::Real32 => {
            let x: f64 = s.parse().context(ParseFloatSnafu {
                message: format!("Failed parsing float default value on {:x}", obj),
            })?;
            Ok(quote!(#x))
        }
        DataType::VisibleString => Ok(string_to_byte_literal(s)),
        DataType::OctetString => Ok(string_to_byte_literal(s)),
        DataType::UnicodeString => Ok(string_to_byte_literal(s)),
        DataType::TimeOfDay | DataType::TimeDifference => panic!("Time types unsupported"),
        DataType::Domain => panic!("How to handle defaults for domain??"),
        DataType::Other(n) => panic!("Unrecognized datatype: 0x{:x}", n),
    }
}

/// Convert an AccessType enum to a tokenstream expressing the variant
fn access_type_tokens(at: AccessType) -> TokenStream {
    match at {
        AccessType::Ro => quote!(canopen_common::objects::AccessType::Ro),
        AccessType::Wo => quote!(canopen_common::objects::AccessType::Wo),
        AccessType::Rw => quote!(canopen_common::objects::AccessType::Rw),
        AccessType::Const => quote!(canopen_common::objects::AccessType::Const),
    }
}

#[derive(Clone, Debug, Default)]
struct NativeVariables {
    /// Declarations for mutable struct
    mutable_decs: Vec<TokenStream>,
    /// Declarations for constant struct
    constant_decs: Vec<TokenStream>,
    /// Initialization values for mutable struct
    mutable_inits: Vec<TokenStream>,
    /// Initialization values for constant struct
    constant_inits: Vec<TokenStream>,
}

fn build_var_object(obj: &Object, vars: &mut NativeVariables) -> Result<TokenStream, CompileError> {
    let obj_name = format_ident!("OBJECT{:X}", obj.object_number);
    let sub0 = obj.subs.get(&0).ok_or(CompileError::MissingSub0 {
        obj_num: obj.object_number,
    })?;

    let field_name = format_ident!("object{:x}", obj.object_number);
    let parameter_name = &obj.parameter_name;

    let default_literal = get_default_literal(obj.object_number, &sub0)?;
    let init_statement = quote!(#field_name: #default_literal);

    #[rustfmt::skip]
    let (data_type, var_statement, size) = match sub0.data_type {
        DataType::Boolean => (
            quote!(canopen_common::objects::DataType::Boolean),
            quote!(#field_name: bool),
            1
        ),
        DataType::Int8 => (
            quote!(canopen_common::objects::DataType::Int8),
            quote!{
                #[doc = #parameter_name]
                #field_name: i8
            },
            2
        ),
        DataType::Int16 => (
            quote!(canopen_common::objects::DataType::Int16),
            quote!(#field_name: i16),
            2
        ),
        DataType::Int32 => (
            quote!(canopen_common::objects::DataType::Int32),
            quote!(#field_name: i32),
            4
        ),
        DataType::UInt8 => (
            quote!(canopen_common::objects::DataType::UInt8),
            quote!{
                #[doc = #parameter_name]
                #field_name: u8
            },
            1
        ),
        DataType::UInt16 => (
            quote!(canopen_common::objects::DataType::UInt16),
            quote!(#field_name: u16),
            2
        ),
        DataType::UInt32 => (
            quote!(canopen_common::objects::DataType::UInt32),
            quote!(#field_name: u32),
            4
        ),
        DataType::Real32 => (
            quote!(canopen_common::objects::DataType::Real32),
            quote!(#field_name: f32),
            4
        ),
        DataType::VisibleString => {
            let str_len = sub0.default_value.len();
            (
                quote!(canopen_common::objects::DataType::VisibleString),
                quote!(#field_name: [u8; #str_len]),
                sub0.default_value.len(),
            )
        },
        DataType::OctetString => {
            let str_len = sub0.default_value.len();
            (
                quote!(canopen_common::objects::DataType::OctetString),
                quote!(#field_name: [u8; #str_len]),
                sub0.default_value.len(),
            )
        },
        DataType::UnicodeString => {
            let str_len = sub0.default_value.len();
            (
                quote!(canopen_common::objects::DataType::UnicodeString),
                quote!(#field_name: [u8; #str_len]),
                sub0.default_value.len(),
            )
        },
        DataType::TimeDifference | DataType::TimeOfDay => panic!("Time unsupported"),
        DataType::Domain => panic!("Can't handle DOMAIN"),
        DataType::Other(_) => panic!("Unhandled datatype {:?}", obj),
    };

    let struct_name = match sub0.access_type {
        AccessType::Const => {
            vars.constant_decs.push(var_statement);
            vars.constant_inits.push(init_statement);
            format_ident!("CONST_DATA")
        }
        AccessType::Ro | AccessType::Wo | AccessType::Rw => {
            // Read-only means that the object cannot be written via the CAN bus, but it may be
            // changed by the application and so it requires storage in RAM
            vars.mutable_decs.push(var_statement);
            vars.mutable_inits.push(init_statement);
            format_ident!("MUT_DATA")
        }
    };

    let ptr_inner = match sub0.data_type {
        DataType::Boolean => quote!(&#struct_name.#field_name as *const bool as *const u8),
        DataType::Int8 => quote!(&#struct_name.#field_name as *const i8 as *const u8),
        DataType::Int16 => quote!(&#struct_name.#field_name as *const i16 as *const u8),
        DataType::Int32 => quote!(&#struct_name.#field_name as *const i32 as *const u8),
        DataType::UInt8 => quote!(&#struct_name.#field_name as *const u8),
        DataType::UInt16 => quote!(&#struct_name.#field_name as *const u16 as *const u8),
        DataType::UInt32 => quote!(&#struct_name.#field_name as *const u32 as *const u8),
        DataType::Real32 => quote!(&#struct_name.#field_name as *const f32 as *const u8),
        DataType::VisibleString | DataType::OctetString | DataType::UnicodeString => {
            quote!(#struct_name.#field_name.as_ptr() as *const u8)
        }
        DataType::Domain => quote! {},
        DataType::TimeOfDay | DataType::TimeDifference => panic!("Can't handle time typess"),
        DataType::Other(_) => panic!("Bad type"),
    };
    let access_type = access_type_tokens(sub0.access_type);
    // unused_unsafe allowed because it is applied to const pointers but not needed. TODO.
    Ok(quote! {
        #[allow(unused_unsafe)]
        static #obj_name: canopen_common::objects::ObjectData =
            canopen_common::objects::ObjectData::Var (
                canopen_common::objects::Var {
                    data_type: #data_type,
                    access_type: #access_type,
                    storage: critical_section::Mutex::new(core::cell::RefCell::new(canopen_common::objects::ObjectStorage::Ram(
                        unsafe { #ptr_inner }, #size
                    ))),
                    size: #size,
                }
            );
    }
    .to_token_stream())
}

fn build_array_object(
    obj: &Object,
    vars: &mut NativeVariables,
) -> Result<TokenStream, CompileError> {
    let obj_name = format_ident!("OBJECT{:X}", obj.object_number);
    let sub0 = obj.subs.get(&0).ok_or(CompileError::MissingSub0 {
        obj_num: obj.object_number,
    })?;

    // Add sub 0 with dynamic storage: some arrays, like the error field (0x1003) use the array size
    // as a dynamic parameter (e.g. current length of the error history). If a default value is not
    // provided, it is set to the size of the array
    let sub0_ident = format_ident!("object{:x}_sub0", obj.object_number);
    let default_value: u8 = sub0
        .default_value
        .parse_eds_u64().unwrap_or(obj.sub_number as u64) as u8;
    vars.mutable_decs.push(quote!(#sub0_ident: u8));
    vars.mutable_inits.push(quote!(#sub0_ident: #default_value));

    let mut default_values_vec = Vec::new();
    let sub1 = obj.subs.get(&1).ok_or(CompileError::MissingSub1 {
        obj_num: obj.object_number,
    })?;

    // Build vector of literals for array initializer
    for i in 1..obj.sub_number {
        let sub = obj.subs.get(&(i as u8)).ok_or(
            MissingSubSnafu {
                obj_num: obj.object_number,
                sub_num: i,
            }
            .build(),
        )?;

        default_values_vec.push(get_default_literal(obj.object_number, sub)?);
    }

    let field_name = format_ident!("object{:x}", obj.object_number);
    let array_size = obj.sub_number as usize - 1;
    let (data_type, dec, size) = match sub1.data_type {
        DataType::Boolean => (
            quote!(canopen_common::objects:DataType::Boolean),
            quote!(#field_name: [bool; #array_size]),
            1 * array_size,
        ),
        DataType::Int8 => (
            quote!(canopen_common::objects::DataType::Int8),
            quote!(#field_name: [i8; #array_size]),
            1 * array_size,
        ),
        DataType::Int16 => (
            quote!(canopen_common::objects::DataType::Int16),
            quote!(#field_name: [i16; #array_size]),
            2 * array_size,
        ),
        DataType::Int32 => (
            quote!(canopen_common::objects::DataType::Int32),
            quote!(#field_name: [i32; #array_size]),
            4 * array_size,
        ),
        DataType::UInt8 => (
            quote!(DataType::UInt8),
            quote!(#field_name: [u8; #array_size]),
            1 * array_size,
        ),
        DataType::UInt16 => (
            quote!(canopen_common::objects::DataType::UInt16),
            quote!(#field_name: [u16; #array_size]),
            2,
        ),
        DataType::UInt32 => (
            quote!(canopen_common::objects::DataType::UInt32),
            quote!(#field_name: [u32; #array_size]),
            4,
        ),
        DataType::Real32 => (
            quote!(canopen_common::objects::DataType::Real32),
            quote!(#field_name: [f32; #array_size]),
            4,
        ),
        DataType::VisibleString => (
            quote!(canopen_common::objects::DataType::VisibleString),
            quote!(#field_name: &str),
            sub0.default_value.len(),
        ),
        DataType::OctetString => (
            quote!(canopen_common::objects::DataType::OctetString),
            quote!(#field_name:  &str),
            sub0.default_value.len(),
        ),
        DataType::UnicodeString => (
            quote!(canopen_common::objects::DataType::UnicodeString),
            quote!(#field_name:  &str),
            sub0.default_value.len(),
        ),
        DataType::TimeDifference | DataType::TimeOfDay => panic!("Can't handle times"),
        DataType::Domain => panic!("Can't handle DOMAIN"),
        DataType::Other(_) => panic!("Unhandled datatype {:?}", obj),
    };

    let init = quote!(#field_name: [#(#default_values_vec),*]);

    let struct_name = match sub0.access_type {
        AccessType::Const => {
            vars.constant_decs.push(dec);
            vars.constant_inits.push(init);
            format_ident!("CONST_DATA")
        }
        AccessType::Ro | AccessType::Wo | AccessType::Rw => {
            // Read-only means that the object cannot be written via the CAN bus, but it may be
            // changed by the application and so it requires storage in RAM
            vars.mutable_decs.push(dec);
            vars.mutable_inits.push(init);
            format_ident!("MUT_DATA")
        }
    };

    let access_type = access_type_tokens(sub1.access_type);
    let mut tokens = TokenStream::new();
    tokens.extend(quote! {
        static #obj_name: canopen_common::objects::ObjectData =
            canopen_common::objects::ObjectData::Array (
                canopen_common::objects::Array {
                    data_type: #data_type,
                    access_type: #access_type,
                    storage: critical_section::Mutex::new(core::cell::RefCell::new(canopen_common::objects::ObjectStorage::Ram(
                        unsafe { #struct_name.#field_name.as_ptr() as *const u8 }, #size
                    ))),
                    storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(canopen_common::objects::ObjectStorage::Ram(
                        unsafe { &#struct_name.#sub0_ident as *const u8 }, 1
                    ))),
                    size: #size,
                }
            );
    });

    Ok(tokens)
}

fn build_record_object(
    obj: &Object,
    vars: &mut NativeVariables,
) -> Result<TokenStream, CompileError> {
    let obj_name = format_ident!("OBJECT{:X}", obj.object_number);
    let sub0 = obj.subs.get(&0).ok_or(CompileError::MissingSub0 {
        obj_num: obj.object_number,
    })?;

    // Allocate storage for the constant sub0 value, which gives the highest
    // sub index in the record
    let sub0_ident = format_ident!("object{:x}_sub0", obj.object_number);
    let default_value: u8 = sub0.default_value.parse_eds_u64().expect(&format!(
        "Error parsing array sub0 default value as u8 on object {:x}",
        obj.object_number
    )) as u8;
    vars.constant_decs.push(quote!(#sub0_ident: u8));
    vars.constant_inits
        .push(quote!(#sub0_ident: #default_value));

    let mut storage_items = Vec::new();
    let mut data_types = Vec::new();
    let mut access_types = Vec::new();
    let mut sizes = Vec::new();
    // Allocate storage for the record members. Each field is created separately. It is possible for
    // records to have missing sub objects, i.e. it is allowed to have a sub1, and a sub3, but no
    // sub2.
    //
    // The storage items cannot go directly in the OBJECTXXXX struct, because of rust static
    // limitations, so a separate static variable (OBJECTXXXX_STORAGE) is created to contain the
    // array of ObjectStorage structs for the record.

    for i in 1..obj.sub_number {
        let field_name = format_ident!("object{:x}_sub{:x}", obj.object_number, i);

        // Create the ObjectStorage array for the record
        match obj.subs.get(&(i as u8)) {
            Some(sub) => {
                let (storage_type, data_struct) = match sub.access_type {
                    AccessType::Ro | AccessType::Wo | AccessType::Rw => {
                        (quote!(Ram), quote!(MUT_DATA))
                    }
                    AccessType::Const => (quote!(Const), quote!(CONST_DATA)),
                };
                let (dec, inner_ptr, storage_size) = match sub.data_type {
                    DataType::Boolean => (
                        quote!(#field_name: bool),
                        quote!(&#data_struct.#field_name as *const bool as *const u8),
                        1,
                    ),
                    DataType::Int8 => (
                        quote!(#field_name: i8),
                        quote!(&#data_struct.#field_name as *const i8 as *const u8),
                        1,
                    ),
                    DataType::Int16 => (
                        quote!(#field_name: i16),
                        quote!(&#data_struct.#field_name as *const i16 as *const u8),
                        2,
                    ),
                    DataType::Int32 => (
                        quote!(#field_name: i32),
                        quote!(&#data_struct.#field_name as *const i32 as *const u8),
                        4,
                    ),
                    DataType::UInt8 => (
                        quote!(#field_name: u8),
                        quote!(&#data_struct.#field_name as *const u8 as *const u8),
                        1,
                    ),
                    DataType::UInt16 => (
                        quote!(#field_name: u16),
                        quote!(&#data_struct.#field_name as *const u16 as *const u8),
                        2,
                    ),
                    DataType::UInt32 => (
                        quote!(#field_name: u32),
                        quote!(&#data_struct.#field_name as *const u32 as *const u8),
                        4,
                    ),
                    DataType::Real32 => (
                        quote!(#field_name: f32),
                        quote!(&#data_struct.#field_name as *const f32 as *const u8),
                        4,
                    ),
                    DataType::VisibleString | DataType::OctetString | DataType::UnicodeString => {
                        let str_len = sub.default_value.len();
                        (
                            quote!(#field_name: [u8; #str_len]),
                            quote!(#data_struct.#field_name.as_ptr() as *const u8),
                            str_len,
                        )
                    }
                    DataType::TimeOfDay | DataType::TimeDifference => panic!("NO TIMES YET"),
                    DataType::Domain => panic!("NO DOMAINS PLEASE YET"),
                    DataType::Other(id) => panic!("invalid data type ({})", id),
                };

                data_types.push(match sub.data_type {
                    DataType::Boolean => quote!(Some(canopen_common::objects::DataType::Boolean)),
                    DataType::Int8 => quote!(Some(canopen_common::objects::DataType::Int8)),
                    DataType::Int16 => quote!(Some(canopen_common::objects::DataType::Int16)),
                    DataType::Int32 => quote!(Some(canopen_common::objects::DataType::Int32)),
                    DataType::UInt8 => quote!(Some(canopen_common::objects::DataType::UInt8)),
                    DataType::UInt16 => quote!(Some(canopen_common::objects::DataType::UInt16)),
                    DataType::UInt32 => quote!(Some(canopen_common::objects::DataType::UInt32)),
                    DataType::Real32 => quote!(Some(canopen_common::objects::DataType::Real32)),
                    DataType::VisibleString => {
                        quote!(Some(canopen_common::objects::DataType::VisibleString))
                    }
                    DataType::OctetString => {
                        quote!(Some(canopen_common::objects::DataType::OctetString))
                    }
                    DataType::UnicodeString => {
                        quote!(Some(canopen_common::objects::DataType::UnicodeString))
                    }
                    DataType::Domain => quote!(Some(canopen_common::objects::DataType::Domain)),
                    DataType::Other(id) => panic!("Unknown datatype {}", id),
                    _ => panic!("Unsupported datatype: {:?}", sub.data_type),
                });

                let access_type = access_type_tokens(sub.access_type);
                access_types.push(quote!(Some(#access_type)));
                sizes.push(storage_size);

                // Create the data allocations and initialization values for record field
                let init_value = get_default_literal(obj.object_number, sub)?;
                let init = quote!(#field_name: #init_value);
                match sub.access_type {
                    AccessType::Ro | AccessType::Wo | AccessType::Rw => {
                        vars.mutable_decs.push(dec);
                        vars.mutable_inits.push(init);
                    }
                    AccessType::Const => {
                        vars.constant_decs.push(dec);
                        vars.constant_inits.push(init);
                    }
                }

                storage_items.push(quote!(Some(critical_section::Mutex::new(
                    core::cell::RefCell::new(
                        canopen_common::objects::ObjectStorage::#storage_type(
                            unsafe { #inner_ptr },
                            #storage_size,
                        )
                    )
                ))));
            }
            None => {
                // Sub objects may be missing, so None is used for a placeholder on non-implemented
                // record fields
                storage_items.push(quote!(None));
                data_types.push(quote!(None));
                access_types.push(quote!(None));
                sizes.push(0);
            }
        }
    }

    let struct_name = match sub0.access_type {
        AccessType::Const => {
            format_ident!("CONST_DATA")
        }
        AccessType::Ro | AccessType::Wo | AccessType::Rw => {
            // Read-only means that the object cannot be written via the CAN bus, but it may be
            // changed by the application and so it requires storage in RAM
            format_ident!("MUT_DATA")
        }
    };
    let storage_array_ident = format_ident!("OBJECT{:X}_STORAGE", obj.object_number);
    let storage_array_len = storage_items.len();

    let mut tokens = TokenStream::new();

    // Create the storage array
    tokens.extend(quote! {
        static #storage_array_ident: [Option<critical_section::Mutex<core::cell::RefCell<
            canopen_common::objects::ObjectStorage>>>; #storage_array_len
        ] = [
            #(#storage_items),*
        ];
    });

    // Create the Object
    tokens.extend(quote! {

        // Allow unused_unsafe because it is not needed for const items.
        #[allow(unused_unsafe)]
        static #obj_name: canopen_common::objects::ObjectData =
            canopen_common::objects::ObjectData::Record (
                canopen_common::objects::Record {
                    data_types: &[#(#data_types),*],
                    access_types: &[#(#access_types),*],
                    storage: &#storage_array_ident,
                    storage_sub0: critical_section::Mutex::new(core::cell::RefCell::new(canopen_common::objects::ObjectStorage::Ram(
                        unsafe { &CONST_DATA.#sub0_ident as *const u8 }, 1
                    ))),
                    sizes: &[#(#sizes),*],
                }
            );
    });

    Ok(tokens)
}

pub fn compile_eds_to_string(
    eds: &ElectronicDataSheet,
    format: bool,
) -> Result<String, CompileError> {
    let mut objects = Vec::new();

    objects.extend_from_slice(&eds.mandatory_objects);
    objects.extend_from_slice(&eds.optional_objects);
    objects.extend_from_slice(&eds.manufacturer_objects);

    let mut native_vars = NativeVariables::default();
    let mut object_declarations = Vec::new();
    // Entries in the top table
    let mut od_entries: Vec<TokenStream> = Vec::new();
    for obj in objects {
        match obj.object_type {
            eds_parser::ObjectType::Var => {
                object_declarations.push(build_var_object(&obj, &mut native_vars)?);
            }
            eds_parser::ObjectType::Array => {
                object_declarations.push(build_array_object(&obj, &mut native_vars)?);
            }
            eds_parser::ObjectType::Record => {
                object_declarations.push(build_record_object(&obj, &mut native_vars)?);
            }
            _ => panic!("Unknown object type: {:?}", obj),
        }
        let object_ident = format_ident!("OBJECT{:X}", obj.object_number);
        let index = obj.object_number as u16;
        od_entries.push(quote! {
            canopen_common::objects::ODEntry {
                index: #index,
                data: &#object_ident,
            }
        });
    }

    let mut_decs = &native_vars.mutable_decs;
    let const_decs = &native_vars.constant_decs;
    let mut_inits = &native_vars.mutable_inits;
    let const_inits = &native_vars.constant_inits;
    let table_size = od_entries.len();
    let code = quote! {
        pub struct MutData {
            #(#mut_decs),*
        }

        pub struct ConstData {
            #(#const_decs),*
        }

        static mut MUT_DATA: MutData = MutData {
            #(#mut_inits),*
        };

        const CONST_DATA: ConstData = ConstData {
            #(#const_inits),*
        };

        #(#object_declarations)*

        pub static OD_TABLE: [canopen_common::objects::ODEntry; #table_size] = {
            [
                #(#od_entries),*
            ]
        };
    }
    .to_string();

    if format {
        Ok(prettyplease::unparse(
            &syn::parse_file(&code).expect("Error parsing generated code: "),
        ))
    } else {
        Ok(code)
    }
}

pub fn compile_eds(
    eds_path: impl AsRef<Path>,
    out_path: impl AsRef<Path>,
) -> Result<(), CompileError> {
    let eds = ElectronicDataSheet::load(eds_path).context(EdsLoadSnafu)?;
    let output = compile_eds_to_string(&eds, true)?;
    std::fs::write(out_path.as_ref(), output.as_bytes()).context(IoSnafu)?;
    Ok(())
}

pub fn build_node_from_eds(name: &str, eds_path: impl AsRef<Path>) -> Result<(), CompileError> {
    let output_file_path =
        Path::new(&std::env::var_os("OUT_DIR").ok_or(NotRunViaCargoSnafu.build())?)
            .join(&format!("zencan_node_{}.rs", name));

    compile_eds(&eds_path, &output_file_path)?;

    let env_var = format!("ZENCAN_INCLUDE_GENERATED_{}", name);
    println!("cargo:rustc-env={}={}", env_var, output_file_path.display());

    Ok(())
}
