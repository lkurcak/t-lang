// What an impl says an associated type is, is the type of values: it has
// a size.
use core::fmt::Display;

trait Tr { type A; }

impl<'a, unsized T: Display> Tr for &'a T { type A = T; }

fn main() {}
// error: `T` is `unsized`, so it can only be behind a reference, as in `&T`
