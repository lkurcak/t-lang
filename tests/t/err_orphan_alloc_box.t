// A box is alloc's, as `Box` is declared there: a program can't implement
// core's `Display` for one, even of its own type.
use alloc::boxed::Box;
use core::fmt::{Display, Write};

#[derive(Discard)]
struct S {}

impl Display for Box<S> {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str("s") }
}

fn main() {}
// error: `Display` is `core`'s and `Box<S>` is `alloc`'s, so only `core` or `alloc` can implement one for the other; to implement it for a type of your own, wrap it in a struct
