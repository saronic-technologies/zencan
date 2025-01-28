Library to generate object dictionary rust code from an input EDS file.

## Dev Notes

### Better errors

When the generated code is not syntactically correct rust code, prettyplease generates errors which
are less than helpful for determining the cause of the error. In this case, rustfmt provides a much
better output, so:

`cargo run --example build_od -- ~/temp/sample.eds > temp.rs`, and then
`rustfmt temp.rs`