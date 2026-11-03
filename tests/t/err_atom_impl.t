// `Atom` is sealed: the integers and `bool` are all there are.
use core::sync::Atom;

#[derive(Discard, Copy, Eq)]
struct Flag { on: bool }

impl Atom for Flag {}
// error: `Atom` is sealed: only its own package, `core`, can implement it

fn main() {}
