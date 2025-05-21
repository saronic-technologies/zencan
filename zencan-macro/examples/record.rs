//! An example, mainly for use with `cargo expand` to see macro output
use zencan_macro::record_object;

#[record_object]
struct TestObject {
    #[record(pdo = "both")]
    val1: u32,
    val2: u16,
    val3: u8,
    val4: i32,
    val5: i16,
    val6: i8,
    val7: f32,
    #[record(persist)]
    val8: [u8; 15],
}

fn main() {

}