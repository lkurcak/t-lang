// Ruling an impl out looks through the impls that could prove its bound:
// `&Bag: Show` holds by `impl Show for &T`, as `Bag: Show`, so `&Bag` is
// both impls' and they overlap.
trait Show {}

trait Tr {
    fn which(&self) -> i64;
}

impl<T: Show> Tr for T {
    fn which(&self) -> i64 { 1 }
}

impl<'a, T: Show> Show for &'a T {}

#[derive(Discard)]
struct Bag {}

impl Show for Bag {}

impl<'a> Tr for &'a Bag { // error: this impl of `Tr` overlaps `impl Tr for T`
    fn which(&self) -> i64 { 2 }
}

fn main() {}
