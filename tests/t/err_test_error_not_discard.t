// Its error is thrown away once it's written out.
use core::fmt::{Display, Write};

struct E {}

impl Display for E {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str("e") }
}

fn main() {}

#[test]
fn errs() -> Result<(), E> {
    Result::Err(E {})
}
// error: the error a test gives back is thrown away once it's written out, so it must be `Discard`
