// Tuples are core's, as `Display` is: only core can say how one shows.
use core::fmt::{Display, Write};

impl<A, B> Display for (A, B) {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str("pair") }
}

fn main() {}
// error: `Display` and `(A, B)` are both `core`'s, so only `core` can implement one for the other
