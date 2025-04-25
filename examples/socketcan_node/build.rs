
fn main() {
    if let Err(e) = zencan_build::build_node_from_device_config("DEVICE", "device_config.toml") {
        eprintln!("Error building node from device config: {}", e);
        std::process::exit(1);
    };
}