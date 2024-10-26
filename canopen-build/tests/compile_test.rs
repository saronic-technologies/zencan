use eds_parser::ElectronicDataSheet;

#[test]
fn compile_test() {

    const EDS: &str = include_str!("example.eds");
    let eds = ElectronicDataSheet::from_str(EDS).expect("Failed loading EDS file");

    let compiled = canopen_build::compile_eds_to_string(&eds, false).expect("Failed to compile");

}