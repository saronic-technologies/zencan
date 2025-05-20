Contains macros used by zencan

## Debugging Hints

Cargo expand is useful for seeing the macro output:

`cargo install cargo-expand`
`cargo expand --example record`

When the output generates compile errors, you can do the following to get better errors:

`cargo expand --example record > examples/expanded.rs`
`cargo build --example expanded`


