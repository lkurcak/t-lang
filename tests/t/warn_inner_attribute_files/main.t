// A module in a file of its own says its own lints at the top of it,
// `#![warn(..)]`, which is inside the `#[allow(..)]` on `mod a;`, so it
// decides. `b` says nothing, and has `#[allow(..)]` on `mod b;`.
#[allow(unused_variables)]
mod a;
#[allow(unused_variables)]
mod b;

fn main() {
    a::f();
    b::f();
}
// warning: unused variable `x`
