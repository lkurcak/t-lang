// An `unsized` parameter may be a sized type too: so an impl for `&U` of
// an `unsized U` overlaps one for `&T` of a sized `T`, at `&i64`, say.
// Which one a `&i64` used would be a choice.
use core::fmt::{Debug, Display};

trait Tr { fn tr(&self) -> u8; }

impl<'a, T: Display> Tr for &'a T {
    fn tr(&self) -> u8 { 0 }
}

impl<'a, unsized U: Debug> Tr for &'a U {
    fn tr(&self) -> u8 { 1 }
}

fn main() {}
// error: this impl of `Tr` overlaps `impl Tr for &'a T`
