// A struct's type argument can't be a refinement: its fields may be
// private, and what's in them is the struct's own business.
#[derive(Discard)]
struct Wrap<T> { x: T }

fn f(w: Wrap<{ v: u8 | v < 3 }>) -> u8 { w.x }

fn main() {}
// error: a refinement can only be a type argument of an enum, not of `Wrap<u8>`, a struct
