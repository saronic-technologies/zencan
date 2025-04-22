
fn main() {
    zencan_build::build_node_from_device_config("EXAMPLE1", "device_configs/example1.toml").unwrap();
    zencan_build::build_node_from_device_config("EXAMPLE2", "device_configs/example2.toml").unwrap();
}