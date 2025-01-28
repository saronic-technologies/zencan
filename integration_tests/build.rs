
fn main() {
    zencan_build::build_node_from_eds("EXAMPLE1", "eds_files/example1.eds").unwrap();
    zencan_build::build_node_from_eds("EXAMPLE2", "eds_files/example2.eds").unwrap();
}