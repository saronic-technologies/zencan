use std::path::Path;

use device_config::DeviceConfig;
use snafu::ResultExt;

mod codegen;
pub mod device_config;
pub mod errors;
pub mod utils;

use crate::errors::*;
pub use codegen::device_config_to_string;
pub use codegen::device_config_to_tokens;

pub fn compile_device_config(
    config_path: impl AsRef<Path>,
    out_path: impl AsRef<Path>,
) -> Result<(), CompileError> {
    let config = DeviceConfig::load(config_path.as_ref())?;

    let code = device_config_to_string(&config, true)?.to_string();

    std::fs::write(out_path.as_ref(), code.as_bytes()).context(IoSnafu)?;
    Ok(())
}

pub fn build_node_from_device_config(
    name: &str,
    config_path: impl AsRef<Path>,
) -> Result<(), CompileError> {
    let output_file_path =
        Path::new(&std::env::var_os("OUT_DIR").ok_or(NotRunViaCargoSnafu.build())?)
            .join(format!("zencan_node_{}.rs", name));

    compile_device_config(&config_path, &output_file_path)?;

    let env_var = format!("ZENCAN_INCLUDE_GENERATED_{}", name);
    println!("cargo:rustc-env={}={}", env_var, output_file_path.display());

    Ok(())
}
