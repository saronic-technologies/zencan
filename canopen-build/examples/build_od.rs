//! Generate object dictionary rust code from an EDS file
//!
//!

use std::path::PathBuf;

use clap::Parser;

use eds_parser::ElectronicDataSheet;

#[derive(Clone, Debug, Parser)]
struct Args {
    eds: PathBuf,
    #[clap(short, long)]
    format: bool,
}

fn main() {
    let args = Args::parse();

    let eds = ElectronicDataSheet::load(&args.eds).expect("Failed loading EDS file");

    let compiled = canopen_build::compile_eds_to_string(&eds, args.format).expect("Failed to compile");

    println!("{}", compiled);
}