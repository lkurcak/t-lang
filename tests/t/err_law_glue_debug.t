// `Debug`'s `fmt` of a derived impl is glue too: a law of it is refused
// whatever the impls (docs/88-iterator-laws.md).
use core::fmt::{Debug, Write};

trait Quiet: Debug {
    law fn fmt<W: Write>(&self, w: &mut W) -> { r: Result<(), W::Error> | r.is_ok() } with W::e;
}

fn main() {}
// error: a law can't be of `Debug`'s `fmt`: some impls of `Debug` are the compiler's, derived or built in, with no body to check what it says of
