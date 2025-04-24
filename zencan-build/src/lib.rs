use std::path::Path;

use device_config::DeviceConfig;
use snafu::ResultExt;

pub mod device_config;
pub mod errors;
mod codegen;


pub use codegen::device_config_to_tokens;
pub use codegen::device_config_to_string;
use crate::errors::*;


// fn get_default_literal(obj: u32, sub: &SubObject) -> Result<TokenStream, CompileError> {
//     let s = if sub.default_value.is_empty() {
//         match sub.data_type {
//             DataType::Boolean
//             | DataType::Int8
//             | DataType::Int16
//             | DataType::Int32
//             | DataType::UInt8
//             | DataType::UInt16
//             | DataType::UInt32 => "0",
//             DataType::Real32 => "0.0",
//             DataType::VisibleString | DataType::OctetString | DataType::UnicodeString => "",
//             DataType::TimeDifference => "",
//             DataType::TimeOfDay => "",
//             DataType::Domain => "",
//             DataType::Other(_) => "",
//         }
//     } else {
//         // The C# EDS editor puts $NODEID+ on fields which are a function of the node id. We drop this
//         // here if present, on the assumption that such fields are handled in code.
//         sub.default_value.as_str().trim_start_matches("$NODEID+")
//     };

//     match sub.data_type {
//         DataType::Boolean => {
//             let num = s.parse_eds_u64().context(ParseIntSnafu {
//                 message: format!("Can't parse default value on {:x}", obj),
//             })?;
//             let value = num != 0;
//             Ok(quote!(#value))
//         }
//         DataType::Int8 => {
//             let x = s.parse_eds_i64().context(ParseIntSnafu {
//                 message: format!("Can't parse default on {:x}", obj),
//             })? as i8;
//             Ok(quote!(#x))
//         }device_config_to_string
//         DataType::Int16 => {
//             let x = s.parse_eds_i64().context(ParseIntSnafu {
//                 message: format!("Can't parse default on {:x}", obj),
//             })? as i16;
//             Ok(quote!(#x))
//         }
//         DataType::Int32 => {
//             let x = s.parse_eds_i64().context(ParseIntSnafu {
//                 message: format!("Can't parse default on {:x}", obj),
//             })? as i32;
//             Ok(quote!(#x))
//         }
//         DataType::UInt8 => {
//             let x = s.parse_eds_u64().context(ParseIntSnafu {
//                 message: format!("Can't parse default on {:x}", obj),
//             })? as u8;
//             Ok(quote!(#x))
//         }
//         DataType::UInt16 => {
//             let x = s.parse_eds_u64().context(ParseIntSnafu {
//                 message: format!("Can't parse default on {:x}", obj),
//             })? as u16;
//             Ok(quote!(#x))
//         }
//         DataType::UInt32 => {
//             let x = s.padevice_config_to_stringrse_eds_u64().context(ParseIntSnafu {
//                 message: format!("Can't parse default on {:x}", obj),
//             })? as u32;
//             Ok(quote!(#x))
//         }
//         DataType::Real32 => {
//             let x: f64 = s.parse().context(ParseFloatSnafu {
//                 message: format!("Failed parsing float default value on {:x}", obj),
//             })?;
//             Ok(quote!(#x))
//         }
//         DataType::VisibleString => Ok(string_to_byte_literal(s)),
//         DataType::OctetString => Ok(string_to_byte_literal(s)),
//         DataType::UnicodeString => Ok(string_to_byte_literal(s)),
//         DataType::TimeOfDay | DataType::TimeDifference => panic!("Time types unsupported"),
//         DataType::Domain => panic!("How to handle defaults for domain??"),
//         DataType::Other(n) => panic!("Unrecognized datatype: 0x{:x}", n),
//     }
// }


pub fn compile_device_config(
    config_path: impl AsRef<Path>,
    out_path: impl AsRef<Path>,
) -> Result<(), CompileError> {
    let config = DeviceConfig::load(config_path.as_ref())?;

    let code = device_config_to_string(&config, true)?.to_string();

    std::fs::write(out_path.as_ref(), code.as_bytes()).context(IoSnafu)?;
    Ok(())
}

pub fn build_node_from_device_config(name: &str, config_path: impl AsRef<Path>) -> Result<(), CompileError> {
    let output_file_path =
        Path::new(&std::env::var_os("OUT_DIR").ok_or(NotRunViaCargoSnafu.build())?)
            .join(&format!("zencan_node_{}.rs", name));

    compile_device_config(&config_path, &output_file_path)?;

    let env_var = format!("ZENCAN_INCLUDE_GENERATED_{}", name);
    println!("cargo:rustc-env={}={}", env_var, output_file_path.display());

    Ok(())
}
