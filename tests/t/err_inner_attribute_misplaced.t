// `#![..]` is only at the top of a module, before its items.
fn main() {}

#![allow(dead_code)]
// error: `#![..]` is only at the top of a module: of its file, or of the `{ }` of `mod m { .. }`, after its `//!` doc comment
