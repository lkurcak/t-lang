// A tuple is `Ord` by a rule of the checker's, not an impl: what isn't
// `Ord` at a tuple is never taken as known.
trait Tr {}

impl<T: Ord> Tr for T {}

#[derive(Discard)]
struct Bag {}

impl Tr for (Bag, i64) {} // error: this impl of `Tr` overlaps `impl Tr for T`

fn main() {}
