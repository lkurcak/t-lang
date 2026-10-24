// Core says how an `i64` is shown; a program can't say it again.
use core::fmt::{Display, Write};

impl Display for i64 {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str("n") }
}

fn main() {}
// error: `Display` and `i64` are both `core`'s, so only `core` can implement one for the other; to implement it for a type of your own, wrap it in a struct
