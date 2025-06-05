# zencan-build

Library crate to generate object dictionary rust code from an input device configuration file. Used to generate static objects for use with `zencan-node` crate.

## Dev Notes

### Better errors

When the generated code is not syntactically correct rust code, prettyplease generates errors which
are less than helpful for determining the cause of the error. In this case, rustfmt provides a much
better output, so:

`cargo run --example build_od -- CONFIG_FILE.toml > temp.rs`, and then
`rustfmt temp.rs`