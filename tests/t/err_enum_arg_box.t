// Only an enum's type argument can be refined: what a `Box` holds isn't a
// variant's field.
use alloc::boxed::Box;

fn f() -> Box<{ v: u8 | v < 3 }> with alloc { Box::new(1) }

fn main() {}
// error: a refinement can only be a type argument of an enum
