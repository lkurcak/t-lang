// `fmt` may do only what the sink does: a `Display` that can panic doesn't
// fit the trait.
use core::fmt::{Display, Write};

#[derive(Discard)]
struct Next { n: i64 }

impl Display for Next {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.put(&(self.n + 1)) }
}

fn main() {}
// error: `+` may panic, but `fmt` doesn't say `with panic`
