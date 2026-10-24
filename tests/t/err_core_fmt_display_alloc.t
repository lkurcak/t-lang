// An impl's `fmt` can't say more effects than the trait's, which has only
// the sink's: a `Display` can't allocate on its own.
use core::fmt::{Display, Write};

#[derive(Discard)]
struct S {}

impl Display for S {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with {alloc, W::e} {
        w.write_str("s")
    }
}

fn main() {}
// error: `fmt` may allocate, but `Display`'s `fmt` can't, so its impls can't either
