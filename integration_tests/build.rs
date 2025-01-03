
fn main() {
    canopen_build::build_node_from_eds("EXAMPLE1", "eds_files/example1.eds").unwrap();
    canopen_build::build_node_from_eds("EXAMPLE2", "eds_files/example2.eds").unwrap();
}