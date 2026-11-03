// With no `t.toml`, a program has its target's libraries: on the
// freestanding target, core and alloc, so no `std`.
fn main(os: std::Os) {} // error: `std` is for the hosted target, and this is built for the freestanding one
