// A tuple is `Debug` by a rule of the checker's, not an impl.
use core::fmt::Debug;

trait Tr {}

impl<T: Debug> Tr for T {}

#[derive(Discard)]
struct Bag {}

impl Tr for (Bag, i64) {} // error: this impl of `Tr` overlaps `impl Tr for T`

fn main() {}
