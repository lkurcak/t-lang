// `Send` has rules of the checker's as well as impls, so what doesn't
// implement it is never taken as known: these overlap, though `Bag`
// isn't `Send`.
trait Tr {}

impl<T: Send> Tr for T {}

struct Bag { n: i64 }

impl Tr for Bag {} // error: this impl of `Tr` overlaps `impl Tr for T`

fn main() {}
